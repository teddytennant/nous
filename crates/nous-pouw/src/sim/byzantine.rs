//! Configurable executor: each worker can be honest, lazy, lying, or copying.

use std::collections::{HashMap, HashSet};

use ed25519_dalek::SigningKey;

use crate::engine::WorkExecutor;
use crate::envelope::JobEnvelope;
use crate::receipt::{WorkReceipt, sign_receipt};
use crate::state::WorkerId;

/// What flavor of misbehavior (if any) a single worker exhibits.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ByzantineKind {
    /// Returns the canonical hash of the workflow payload.
    Honest,
    /// Returns a constant garbage hash (always disagrees).
    Liar,
    /// Returns a different garbage hash on every call (random fork).
    Chaotic,
    /// Returns the honest hash but with the wrong worker id (replay).
    /// Used by the replay test.
    Impersonator { real_worker: WorkerId },
}

pub struct ConfigurableExecutor {
    sks: HashMap<WorkerId, SigningKey>,
    behaviors: HashMap<WorkerId, ByzantineKind>,
    /// Counter to generate distinct chaotic outputs across calls.
    chaos: u64,
    /// Workers we've seen this round; used by the optional "no-op" mode where
    /// some workers just don't reply at all.
    skip: HashSet<WorkerId>,
}

impl ConfigurableExecutor {
    pub fn new(sks: &[SigningKey]) -> Self {
        let mut map = HashMap::new();
        let mut behaviors = HashMap::new();
        for sk in sks {
            let id = WorkerId::from_verifying_key(&sk.verifying_key());
            let bytes = sk.to_bytes();
            map.insert(id, SigningKey::from_bytes(&bytes));
            behaviors.insert(id, ByzantineKind::Honest);
        }
        Self {
            sks: map,
            behaviors,
            chaos: 0,
            skip: HashSet::new(),
        }
    }

    pub fn set_behavior(&mut self, worker: WorkerId, k: ByzantineKind) {
        self.behaviors.insert(worker, k);
    }

    pub fn skip_next_round(&mut self, worker: WorkerId) {
        self.skip.insert(worker);
    }

    /// Bulk-set: the first `n` workers (in iteration order) become liars.
    pub fn make_first_n_liars(&mut self, n: usize) {
        let ids: Vec<WorkerId> = self.behaviors.keys().copied().collect();
        for id in ids.into_iter().take(n) {
            self.set_behavior(id, ByzantineKind::Liar);
        }
    }
}

impl WorkExecutor for ConfigurableExecutor {
    fn execute(&mut self, worker: WorkerId, job: &JobEnvelope) -> WorkReceipt {
        // Workers that "go offline" return a malformed receipt that fails
        // signature verification — engine drops it.
        if self.skip.contains(&worker) {
            return WorkReceipt {
                job_id: job.id(),
                worker,
                output_hash: [0; 32],
                trace_root: [0; 32],
                rubric: None,
                latency_ms: 0,
                signature: vec![0u8; 64], // bogus
            };
        }

        let kind = self
            .behaviors
            .get(&worker)
            .copied()
            .unwrap_or(ByzantineKind::Honest);

        let (output_hash, claimed_worker, signing_id) = match kind {
            ByzantineKind::Honest => {
                let h = blake3::hash(&job.workflow_payload);
                (*h.as_bytes(), worker, worker)
            }
            ByzantineKind::Liar => {
                let h = blake3::hash(b"GARBAGE");
                (*h.as_bytes(), worker, worker)
            }
            ByzantineKind::Chaotic => {
                self.chaos = self.chaos.wrapping_add(1);
                let h = blake3::hash(&self.chaos.to_le_bytes());
                (*h.as_bytes(), worker, worker)
            }
            ByzantineKind::Impersonator { real_worker } => {
                // Sign with our own key but claim to be someone else — verify
                // will fail because pubkey/sig mismatch.
                let h = blake3::hash(&job.workflow_payload);
                (*h.as_bytes(), real_worker, worker)
            }
        };

        let sk = self.sks.get(&signing_id).expect("signing key for worker");
        let mut r = WorkReceipt {
            job_id: job.id(),
            worker: claimed_worker,
            output_hash,
            trace_root: [0; 32],
            rubric: None,
            latency_ms: 1,
            signature: vec![],
        };
        sign_receipt(&mut r, sk);
        r
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::envelope::ModelPin;
    use ed25519_dalek::SigningKey;
    use rand::rngs::OsRng;

    fn job() -> JobEnvelope {
        JobEnvelope {
            nonce: 1,
            workflow_cid: [0; 32],
            workflow_payload: b"input".to_vec(),
            model: ModelPin::new("m", 0),
            n_replicas: 3,
            bounty: 100,
            deadline_ms: 1000,
        }
    }

    #[test]
    fn honest_executor_produces_canonical_hash() {
        let sks: Vec<_> = (0..3).map(|_| SigningKey::generate(&mut OsRng)).collect();
        let mut exec = ConfigurableExecutor::new(&sks);
        let w = WorkerId::from_verifying_key(&sks[0].verifying_key());
        let r = exec.execute(w, &job());
        assert_eq!(r.output_hash, *blake3::hash(b"input").as_bytes());
        r.verify().unwrap();
    }

    #[test]
    fn liar_returns_garbage_hash() {
        let sks: Vec<_> = (0..1).map(|_| SigningKey::generate(&mut OsRng)).collect();
        let mut exec = ConfigurableExecutor::new(&sks);
        let w = WorkerId::from_verifying_key(&sks[0].verifying_key());
        exec.set_behavior(w, ByzantineKind::Liar);
        let r = exec.execute(w, &job());
        assert_eq!(r.output_hash, *blake3::hash(b"GARBAGE").as_bytes());
    }

    #[test]
    fn chaotic_returns_distinct_hashes_per_call() {
        let sks: Vec<_> = (0..1).map(|_| SigningKey::generate(&mut OsRng)).collect();
        let mut exec = ConfigurableExecutor::new(&sks);
        let w = WorkerId::from_verifying_key(&sks[0].verifying_key());
        exec.set_behavior(w, ByzantineKind::Chaotic);
        let a = exec.execute(w, &job()).output_hash;
        let b = exec.execute(w, &job()).output_hash;
        assert_ne!(a, b);
    }

    #[test]
    fn skipped_worker_returns_invalid_receipt() {
        let sks: Vec<_> = (0..1).map(|_| SigningKey::generate(&mut OsRng)).collect();
        let mut exec = ConfigurableExecutor::new(&sks);
        let w = WorkerId::from_verifying_key(&sks[0].verifying_key());
        exec.skip_next_round(w);
        let r = exec.execute(w, &job());
        assert!(r.verify().is_err());
    }

    #[test]
    fn impersonator_receipt_fails_verification() {
        let sks: Vec<_> = (0..2).map(|_| SigningKey::generate(&mut OsRng)).collect();
        let real = WorkerId::from_verifying_key(&sks[0].verifying_key());
        let attacker = WorkerId::from_verifying_key(&sks[1].verifying_key());
        let mut exec = ConfigurableExecutor::new(&sks);
        exec.set_behavior(attacker, ByzantineKind::Impersonator { real_worker: real });
        let r = exec.execute(attacker, &job());
        // signed by attacker, claims to be real → verify fails
        assert!(r.verify().is_err());
    }

    #[test]
    fn make_first_n_liars_takes_exactly_n() {
        let sks: Vec<_> = (0..5).map(|_| SigningKey::generate(&mut OsRng)).collect();
        let mut exec = ConfigurableExecutor::new(&sks);
        exec.make_first_n_liars(2);
        let liars = exec
            .behaviors
            .values()
            .filter(|b| matches!(b, ByzantineKind::Liar))
            .count();
        assert_eq!(liars, 2);
    }
}
