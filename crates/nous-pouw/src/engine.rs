//! Round driver: pulls receipts in, ships a signed block out.
//!
//! In v0 the engine is purely synchronous and drives one round at a time.
//! The simulator (in [`crate::sim`]) wraps multiple engines + a fake
//! [`Network`] to test multi-node behavior.

use std::collections::HashMap;

use ed25519_dalek::SigningKey;
use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::block::{Block, BlockBody, BlockHeader, sign_block, verify_block};
use crate::envelope::{JobEnvelope, JobId};
use crate::quorum::{QuorumError, form_quorum};
use crate::receipt::WorkReceipt;
use crate::selection::select_workers;
use crate::slashing::{SlashEvent, SlashKind};
use crate::state::{ChainState, StateError, WorkerId};

/// Tunable consensus parameters.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EngineConfig {
    /// How many independent workers must execute each job.
    pub n_replicas_per_job: u8,
    /// Trust-weighted micro-fraction needed to certify a job's output.
    /// 666_667 ≈ 2/3.
    pub quorum_threshold_micro: u32,
    /// Stake slashed per equivocation event.
    pub equivocation_slash: u64,
    /// Stake slashed per dissenting receipt (typically much smaller than equivocation).
    pub dissent_slash: u64,
}

impl Default for EngineConfig {
    fn default() -> Self {
        Self {
            n_replicas_per_job: 5,
            quorum_threshold_micro: 666_667,
            equivocation_slash: 1_000,
            dissent_slash: 10,
        }
    }
}

/// Anything that can run a job for a worker. The simulator implements this
/// with a configurable byzantine fraction; `axon-pouw` implements it by
/// running the actual workflow.
pub trait WorkExecutor {
    fn execute(&mut self, worker: WorkerId, job: &JobEnvelope) -> WorkReceipt;
}

/// What one round produced.
#[derive(Debug)]
pub struct RoundOutcome {
    pub block: Block,
    pub failed_jobs: Vec<(JobId, QuorumError)>,
}

#[derive(Debug, Error)]
pub enum EngineError {
    #[error("state error: {0}")]
    State(#[from] StateError),
    #[error("crypto error: {0}")]
    Crypto(#[from] nous_core::Error),
}

pub struct Engine {
    pub state: ChainState,
    pub config: EngineConfig,
    pub epoch: u64,
}

impl Engine {
    pub fn new(state: ChainState, config: EngineConfig) -> Self {
        Self {
            state,
            config,
            epoch: 0,
        }
    }

    /// Run one round: select workers per job, collect receipts, form quorums,
    /// build + sign + apply the block.
    ///
    /// `leader_sk` must correspond to a registered worker (the leader for
    /// this round, typically chosen by the simulator round-robin or VRF).
    pub fn step<E: WorkExecutor>(
        &mut self,
        executor: &mut E,
        jobs: &[JobEnvelope],
        leader_sk: &SigningKey,
        timestamp_ms: u64,
    ) -> Result<RoundOutcome, EngineError> {
        self.step_full(executor, jobs, &[], leader_sk, timestamp_ms, None, true)
    }

    /// Build (and optionally apply) a block with the full BFT options.
    ///
    /// - `transactions`: the leader pulls these from its mempool and embeds them.
    /// - `parent_qc`: justifies the *previous* block as finalized.
    /// - `auto_apply`: if true, apply the resulting block to local state. Set
    ///   to `false` in multi-validator BFT where the block is only applied
    ///   after ⅔ vote certificate forms.
    #[allow(clippy::too_many_arguments)]
    pub fn step_full<E: WorkExecutor>(
        &mut self,
        executor: &mut E,
        jobs: &[JobEnvelope],
        transactions: &[crate::tx::Transaction],
        leader_sk: &SigningKey,
        timestamp_ms: u64,
        parent_qc: Option<crate::bft::VoteCertificate>,
        auto_apply: bool,
    ) -> Result<RoundOutcome, EngineError> {
        self.epoch += 1;
        let prev_hash = self.state.head_hash;
        let leader = WorkerId::from_verifying_key(&leader_sk.verifying_key());

        let mut certs = Vec::new();
        let mut failed_jobs = Vec::new();
        let mut dissenters: std::collections::BTreeSet<WorkerId> = Default::default();

        for job in jobs {
            let selected = select_workers(
                &self.state,
                &prev_hash,
                self.epoch,
                self.config.n_replicas_per_job as usize,
            );
            if selected.is_empty() {
                continue;
            }
            let receipts: Vec<WorkReceipt> = selected
                .iter()
                .map(|w| executor.execute(*w, job))
                .filter(|r| r.verify().is_ok())
                .collect();
            match form_quorum(
                job,
                &receipts,
                &self.state,
                self.config.quorum_threshold_micro,
            ) {
                Ok(cert) => {
                    for d in &cert.dissenting_workers {
                        dissenters.insert(*d);
                    }
                    certs.push(cert);
                }
                Err(e) => failed_jobs.push((job.id(), e)),
            }
        }

        let slashes: Vec<SlashEvent> = dissenters
            .into_iter()
            .map(|w| SlashEvent {
                worker: w,
                kind: SlashKind::Dissent,
                amount: self.config.dissent_slash,
            })
            .collect();

        let body = BlockBody {
            certs,
            slashes,
            mints: vec![],
            transactions: transactions.to_vec(),
        };
        let body_hash = body.hash();

        // Project state to compute state_root: the root *after* apply_block.
        let mut projected = self.state.clone();
        let pre_height = projected.height;
        let mut header = BlockHeader {
            height: pre_height + 1,
            prev_hash,
            state_root: [0; 32],
            body_hash,
            timestamp_ms,
            leader,
            signature: vec![],
            parent_qc: parent_qc.clone(),
        };
        let mut probe_block = Block {
            header: header.clone(),
            body: body.clone(),
        };
        sign_block(&mut probe_block.header, leader_sk);
        projected.apply_block(&probe_block)?;
        header.state_root = projected.root();

        let mut final_header = header;
        sign_block(&mut final_header, leader_sk);
        let block = Block {
            header: final_header,
            body,
        };
        verify_block(&block)?;

        if auto_apply {
            self.state.apply_block(&block)?;
        }
        Ok(RoundOutcome { block, failed_jobs })
    }

    /// Apply a block produced by another validator (after BFT QC forms).
    pub fn apply_external_block(&mut self, block: &Block) -> Result<(), EngineError> {
        verify_block(block)?;
        self.state.apply_block(block)?;
        Ok(())
    }
}

/// Convenience: aggregate per-worker mints from a finalized block.
pub fn mints_from_block(block: &Block) -> HashMap<WorkerId, u64> {
    let mut out: HashMap<WorkerId, u64> = HashMap::new();
    for cert in &block.body.certs {
        let n = cert.agreeing_workers.len() as u64;
        if n == 0 {
            continue;
        }
        let per = cert.bounty / n;
        for w in &cert.agreeing_workers {
            *out.entry(*w).or_default() += per;
        }
    }
    for m in &block.body.mints {
        *out.entry(WorkerId(m.recipient)).or_default() += m.amount;
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::envelope::ModelPin;
    use crate::receipt::sign_receipt;
    use ed25519_dalek::SigningKey;
    use rand::rngs::OsRng;

    struct HonestExecutor {
        sks: HashMap<WorkerId, SigningKey>,
    }

    impl HonestExecutor {
        fn new(sks: &[SigningKey]) -> Self {
            let mut map = HashMap::new();
            for sk in sks {
                let id = WorkerId::from_verifying_key(&sk.verifying_key());
                // SigningKey doesn't impl Clone in ed25519-dalek; reconstruct from bytes.
                let bytes = sk.to_bytes();
                map.insert(id, SigningKey::from_bytes(&bytes));
            }
            Self { sks: map }
        }
    }

    impl WorkExecutor for HonestExecutor {
        fn execute(&mut self, worker: WorkerId, job: &JobEnvelope) -> WorkReceipt {
            // Honest output: hash of payload.
            let h = blake3::hash(&job.workflow_payload);
            let mut r = WorkReceipt {
                job_id: job.id(),
                worker,
                output_hash: *h.as_bytes(),
                trace_root: [0; 32],
                rubric: None,
                latency_ms: 1,
                signature: vec![],
            };
            sign_receipt(&mut r, self.sks.get(&worker).expect("worker key"));
            r
        }
    }

    fn setup(n: usize, stake: u64) -> (Vec<SigningKey>, ChainState) {
        let mut state = ChainState::new();
        let sks: Vec<_> = (0..n).map(|_| SigningKey::generate(&mut OsRng)).collect();
        for sk in &sks {
            state.register_worker(
                WorkerId::from_verifying_key(&sk.verifying_key()),
                stake,
                1.0,
            );
        }
        (sks, state)
    }

    fn job(nonce: u64, payload: &[u8]) -> JobEnvelope {
        JobEnvelope {
            nonce,
            workflow_cid: [0; 32],
            workflow_payload: payload.to_vec(),
            model: ModelPin::new("m", 0),
            n_replicas: 5,
            bounty: 1000,
            deadline_ms: 1_000_000,
        }
    }

    #[test]
    fn one_honest_round_produces_finalized_block() {
        let (sks, state) = setup(8, 100);
        let cfg = EngineConfig {
            n_replicas_per_job: 5,
            ..Default::default()
        };
        let mut engine = Engine::new(state, cfg);
        let mut exec = HonestExecutor::new(&sks);
        let jobs = vec![job(1, b"hello")];
        let outcome = engine.step(&mut exec, &jobs, &sks[0], 0).unwrap();
        assert_eq!(outcome.failed_jobs.len(), 0);
        assert_eq!(outcome.block.body.certs.len(), 1);
        assert_eq!(outcome.block.body.certs[0].dissenting_workers.len(), 0);
        assert!(outcome.block.body.slashes.is_empty());
        assert_eq!(engine.state.height, 1);
    }

    #[test]
    fn multiple_jobs_in_one_round() {
        let (sks, state) = setup(8, 100);
        let mut engine = Engine::new(state, EngineConfig::default());
        let mut exec = HonestExecutor::new(&sks);
        let jobs = vec![job(1, b"a"), job(2, b"b"), job(3, b"c")];
        let outcome = engine.step(&mut exec, &jobs, &sks[0], 0).unwrap();
        assert_eq!(outcome.block.body.certs.len(), 3);
    }

    #[test]
    fn block_chains_to_previous_head() {
        let (sks, state) = setup(8, 100);
        let mut engine = Engine::new(state, EngineConfig::default());
        let mut exec = HonestExecutor::new(&sks);
        let _ = engine.step(&mut exec, &[job(1, b"a")], &sks[0], 0).unwrap();
        let head_after_1 = engine.state.head_hash;
        let outcome = engine.step(&mut exec, &[job(2, b"b")], &sks[1], 1).unwrap();
        assert_eq!(outcome.block.header.prev_hash, head_after_1);
        assert_eq!(engine.state.height, 2);
    }

    #[test]
    fn no_jobs_still_advances_height() {
        let (sks, state) = setup(4, 100);
        let mut engine = Engine::new(state, EngineConfig::default());
        let mut exec = HonestExecutor::new(&sks);
        let outcome = engine.step(&mut exec, &[], &sks[0], 0).unwrap();
        assert_eq!(outcome.block.body.certs.len(), 0);
        assert_eq!(engine.state.height, 1);
    }

    #[test]
    fn mints_from_block_sums_per_worker() {
        let (sks, state) = setup(8, 100);
        let mut engine = Engine::new(state, EngineConfig::default());
        let mut exec = HonestExecutor::new(&sks);
        let outcome = engine.step(&mut exec, &[job(1, b"x")], &sks[0], 0).unwrap();
        let mints = mints_from_block(&outcome.block);
        let total: u64 = mints.values().sum();
        // bounty 1000 / 5 winners = 200 each, total 1000
        assert_eq!(total, 1000);
        for amount in mints.values() {
            assert_eq!(*amount, 200);
        }
    }

    #[test]
    fn balance_updates_match_mints() {
        let (sks, state) = setup(8, 100);
        let mut engine = Engine::new(state, EngineConfig::default());
        let mut exec = HonestExecutor::new(&sks);
        let outcome = engine.step(&mut exec, &[job(1, b"x")], &sks[0], 0).unwrap();
        let mints = mints_from_block(&outcome.block);
        for (w, expected) in mints {
            assert_eq!(engine.state.workers[&w].balance, expected);
        }
    }
}
