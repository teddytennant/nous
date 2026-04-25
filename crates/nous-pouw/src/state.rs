//! Chain state: balances, stakes, trust, finalized blocks.
//!
//! Pure in-memory state machine — no I/O. v1 will back this with SQLite via
//! `nous-storage`, but the state-transition rules live here and never change.

use std::collections::{BTreeMap, BTreeSet, HashMap};

use ed25519_dalek::VerifyingKey;
use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::block::{Block, BlockHash, BlockHeight};
use crate::envelope::JobId;
use crate::mint::MintReceipt;
use crate::quorum::QuorumCertificate;
use crate::slashing::SlashEvent;

/// Worker public key (ed25519), wrapped for stable serde + ordering.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct WorkerId(pub [u8; 32]);

impl WorkerId {
    pub fn from_verifying_key(vk: &VerifyingKey) -> Self {
        Self(vk.to_bytes())
    }

    pub fn verifying_key(&self) -> Result<VerifyingKey, StateError> {
        VerifyingKey::from_bytes(&self.0).map_err(|e| StateError::InvalidKey(e.to_string()))
    }

    pub fn short(&self) -> String {
        hex::encode(&self.0[..6])
    }
}

impl Serialize for WorkerId {
    fn serialize<S: serde::Serializer>(&self, s: S) -> std::result::Result<S::Ok, S::Error> {
        s.serialize_str(&hex::encode(self.0))
    }
}

impl<'de> Deserialize<'de> for WorkerId {
    fn deserialize<D: serde::Deserializer<'de>>(d: D) -> std::result::Result<Self, D::Error> {
        let s = String::deserialize(d)?;
        let bytes = hex::decode(&s).map_err(serde::de::Error::custom)?;
        let arr: [u8; 32] = bytes
            .try_into()
            .map_err(|_| serde::de::Error::custom("worker id must be 32 bytes"))?;
        Ok(Self(arr))
    }
}

/// Per-worker state tracked by the chain.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct WorkerInfo {
    /// Tokens locked as collateral against misbehavior.
    pub stake: u64,
    /// Earned balance (mint receipts minus transfers — for v0 we don't model
    /// transfers, so this monotonically grows).
    pub balance: u64,
    /// Subjective trust score in [0.0, 1.0]. Provided externally (e.g. by the
    /// axon `TrustScorer`); the chain just uses it for selection + quorum
    /// weighting and never recomputes it.
    pub trust: f64,
    /// Whether this worker is currently slashed (frozen out of selection).
    pub slashed: bool,
}

impl WorkerInfo {
    /// Effective weight for selection / quorum: `stake * trust`, with slashed
    /// workers contributing zero.
    pub fn weight(&self) -> f64 {
        if self.slashed {
            0.0
        } else {
            (self.stake as f64) * self.trust
        }
    }
}

/// The 32-byte digest of the entire chain state (computed deterministically
/// from the BTreeMap-ordered `workers` and `used_jobs`).
pub type StateRoot = [u8; 32];

/// In-memory chain state.
#[derive(Debug, Clone, Default)]
pub struct ChainState {
    pub height: BlockHeight,
    pub head_hash: BlockHash,
    pub workers: BTreeMap<WorkerId, WorkerInfo>,
    /// Jobs whose mints have been applied — prevents double-mint per job.
    pub used_jobs: BTreeSet<JobId>,
    /// Per-(worker, job) mint guard — also prevents the same worker being
    /// minted twice for the same job under different blocks.
    pub used_worker_jobs: BTreeSet<(WorkerId, JobId)>,
    /// Finalized block hashes by height (for equivocation detection).
    pub finalized: HashMap<BlockHeight, BlockHash>,
    /// Total supply (sum of all mints minus all slashes that burned stake).
    pub total_supply: u64,
}

#[derive(Debug, Error)]
pub enum StateError {
    #[error("block height mismatch: expected {expected}, got {actual}")]
    HeightMismatch {
        expected: BlockHeight,
        actual: BlockHeight,
    },
    #[error("prev_hash mismatch at height {height}")]
    PrevHashMismatch { height: BlockHeight },
    #[error("double mint for job {0}")]
    DoubleMint(JobId),
    #[error("mint to unknown worker {0}")]
    UnknownWorker(String),
    #[error("invalid signature")]
    InvalidSignature,
    #[error("invalid public key: {0}")]
    InvalidKey(String),
    #[error("slash target unknown: {0}")]
    UnknownSlashTarget(String),
    #[error("quorum cert references unknown worker")]
    UnknownQuorumWorker,
    #[error("crypto error: {0}")]
    Crypto(#[from] nous_core::Error),
}

impl ChainState {
    pub fn new() -> Self {
        Self::default()
    }

    /// Register a new worker with stake + initial trust. Idempotent: if the
    /// worker exists, stake is *added* and trust is replaced.
    pub fn register_worker(&mut self, worker: WorkerId, stake: u64, trust: f64) {
        let entry = self.workers.entry(worker).or_default();
        entry.stake = entry.stake.saturating_add(stake);
        entry.trust = trust.clamp(0.0, 1.0);
    }

    /// Update a worker's trust score (typically called from the trust bridge
    /// after a quorum outcome).
    pub fn set_trust(&mut self, worker: WorkerId, trust: f64) {
        if let Some(info) = self.workers.get_mut(&worker) {
            info.trust = trust.clamp(0.0, 1.0);
        }
    }

    /// Apply a finalized block. The caller is responsible for finality
    /// (e.g. ⅔ stake-weighted votes) — `apply_block` only enforces structural
    /// validity and the no-double-mint invariant.
    pub fn apply_block(&mut self, block: &Block) -> Result<(), StateError> {
        // Height must be exactly height+1 (or 0 for genesis).
        if block.header.height != self.height + 1 {
            return Err(StateError::HeightMismatch {
                expected: self.height + 1,
                actual: block.header.height,
            });
        }
        if block.header.prev_hash != self.head_hash {
            return Err(StateError::PrevHashMismatch {
                height: block.header.height,
            });
        }

        // Apply slashes first (a slashed worker cannot be in this block's mints
        // if the slash and mint are in the same block — slashes win).
        for slash in &block.body.slashes {
            self.apply_slash(slash)?;
        }

        // Apply quorum certs: each cert mints to its winning workers.
        for cert in &block.body.certs {
            self.apply_quorum_cert(cert)?;
        }

        // Apply explicit mint receipts (for any mints not flowing through a
        // cert, e.g. genesis grants). Verifies no double-mint.
        for mint in &block.body.mints {
            self.apply_mint(mint)?;
        }

        self.height = block.header.height;
        self.head_hash = block.hash();
        self.finalized.insert(block.header.height, block.hash());
        Ok(())
    }

    fn apply_quorum_cert(&mut self, cert: &QuorumCertificate) -> Result<(), StateError> {
        if self.used_jobs.contains(&cert.job_id) {
            return Err(StateError::DoubleMint(cert.job_id));
        }
        // Each agreeing worker gets bounty / n_winners.
        let winners = cert.agreeing_workers.len();
        if winners == 0 {
            return Ok(());
        }
        let per = cert.bounty / winners as u64;
        for worker in &cert.agreeing_workers {
            if !self.workers.contains_key(worker) {
                return Err(StateError::UnknownQuorumWorker);
            }
            let pair = (*worker, cert.job_id);
            if self.used_worker_jobs.contains(&pair) {
                return Err(StateError::DoubleMint(cert.job_id));
            }
            self.workers.get_mut(worker).unwrap().balance += per;
            self.used_worker_jobs.insert(pair);
            self.total_supply = self.total_supply.saturating_add(per);
        }
        self.used_jobs.insert(cert.job_id);
        Ok(())
    }

    fn apply_mint(&mut self, mint: &MintReceipt) -> Result<(), StateError> {
        let worker = WorkerId(mint.recipient);
        if !self.workers.contains_key(&worker) {
            return Err(StateError::UnknownWorker(worker.short()));
        }
        let pair = (worker, mint.job_id);
        if self.used_worker_jobs.contains(&pair) {
            return Err(StateError::DoubleMint(mint.job_id));
        }
        self.workers.get_mut(&worker).unwrap().balance += mint.amount;
        self.used_worker_jobs.insert(pair);
        self.total_supply = self.total_supply.saturating_add(mint.amount);
        Ok(())
    }

    fn apply_slash(&mut self, slash: &SlashEvent) -> Result<(), StateError> {
        let worker = slash.worker;
        let info = self
            .workers
            .get_mut(&worker)
            .ok_or_else(|| StateError::UnknownSlashTarget(worker.short()))?;
        let burn = info.stake.min(slash.amount);
        info.stake -= burn;
        info.slashed = true;
        // Slashed stake is burned, reducing total supply.
        self.total_supply = self.total_supply.saturating_sub(burn);
        Ok(())
    }

    /// Deterministic state digest. Used by [`crate::block::BlockHeader::state_root`].
    pub fn root(&self) -> StateRoot {
        let mut h = blake3::Hasher::new();
        h.update(&self.height.to_le_bytes());
        h.update(&self.head_hash);
        h.update(&self.total_supply.to_le_bytes());
        for (id, info) in &self.workers {
            h.update(&id.0);
            h.update(&info.stake.to_le_bytes());
            h.update(&info.balance.to_le_bytes());
            h.update(&info.trust.to_bits().to_le_bytes());
            h.update(&[info.slashed as u8]);
        }
        for job in &self.used_jobs {
            h.update(&job.0);
        }
        for (worker, job) in &self.used_worker_jobs {
            h.update(&worker.0);
            h.update(&job.0);
        }
        *h.finalize().as_bytes()
    }

    /// Total active (non-slashed) stake — denominator for ⅔ finality.
    pub fn active_stake(&self) -> u64 {
        self.workers
            .values()
            .filter(|w| !w.slashed)
            .map(|w| w.stake)
            .sum()
    }

    /// Eligible workers for selection: stake > 0 and not slashed.
    pub fn eligible_workers(&self) -> Vec<WorkerId> {
        self.workers
            .iter()
            .filter(|(_, w)| !w.slashed && w.stake > 0)
            .map(|(id, _)| *id)
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ed25519_dalek::SigningKey;
    use rand::rngs::OsRng;

    fn worker_id(seed: u8) -> WorkerId {
        let mut bytes = [0u8; 32];
        bytes[0] = seed;
        WorkerId(bytes)
    }

    fn signing_worker() -> WorkerId {
        let sk = SigningKey::generate(&mut OsRng);
        WorkerId::from_verifying_key(&sk.verifying_key())
    }

    #[test]
    fn register_worker_idempotent_adds_stake() {
        let mut s = ChainState::new();
        let w = worker_id(1);
        s.register_worker(w, 100, 0.5);
        s.register_worker(w, 50, 0.8);
        assert_eq!(s.workers[&w].stake, 150);
        assert_eq!(s.workers[&w].trust, 0.8);
    }

    #[test]
    fn trust_clamped_to_unit_interval() {
        let mut s = ChainState::new();
        let w = worker_id(2);
        s.register_worker(w, 1, 1.5);
        assert_eq!(s.workers[&w].trust, 1.0);
        s.set_trust(w, -0.3);
        assert_eq!(s.workers[&w].trust, 0.0);
    }

    #[test]
    fn weight_zero_when_slashed() {
        let info = WorkerInfo {
            stake: 100,
            balance: 0,
            trust: 1.0,
            slashed: true,
        };
        assert_eq!(info.weight(), 0.0);
    }

    #[test]
    fn weight_is_stake_times_trust() {
        let info = WorkerInfo {
            stake: 1000,
            balance: 0,
            trust: 0.5,
            slashed: false,
        };
        assert_eq!(info.weight(), 500.0);
    }

    #[test]
    fn root_changes_when_state_changes() {
        let mut s = ChainState::new();
        let r0 = s.root();
        s.register_worker(worker_id(1), 100, 0.5);
        let r1 = s.root();
        assert_ne!(r0, r1);
    }

    #[test]
    fn root_stable_across_insert_order() {
        let mut a = ChainState::new();
        let mut b = ChainState::new();
        a.register_worker(worker_id(1), 1, 0.1);
        a.register_worker(worker_id(2), 2, 0.2);
        b.register_worker(worker_id(2), 2, 0.2);
        b.register_worker(worker_id(1), 1, 0.1);
        assert_eq!(a.root(), b.root());
    }

    #[test]
    fn active_stake_excludes_slashed() {
        let mut s = ChainState::new();
        s.register_worker(worker_id(1), 100, 1.0);
        s.register_worker(worker_id(2), 50, 1.0);
        s.workers.get_mut(&worker_id(2)).unwrap().slashed = true;
        assert_eq!(s.active_stake(), 100);
    }

    #[test]
    fn eligible_excludes_zero_stake_and_slashed() {
        let mut s = ChainState::new();
        s.register_worker(worker_id(1), 100, 1.0);
        s.register_worker(worker_id(2), 0, 1.0);
        s.register_worker(worker_id(3), 50, 1.0);
        s.workers.get_mut(&worker_id(3)).unwrap().slashed = true;
        let eligible = s.eligible_workers();
        assert_eq!(eligible, vec![worker_id(1)]);
    }

    #[test]
    fn worker_id_serde_round_trip() {
        let w = signing_worker();
        let json = serde_json::to_string(&w).unwrap();
        let back: WorkerId = serde_json::from_str(&json).unwrap();
        assert_eq!(w, back);
    }
}
