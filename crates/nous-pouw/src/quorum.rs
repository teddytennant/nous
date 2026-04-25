//! Quorum certificates: trust-weighted agreement on a job's output.
//!
//! Given a set of revealed [`WorkReceipt`]s for one job, [`form_quorum`]
//! groups them by `output_hash`, computes the trust-weighted vote for each
//! group, and certifies the winner if it crosses the configured threshold.

use std::collections::HashMap;

use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::envelope::{JobEnvelope, JobId};
use crate::receipt::{OutputHash, WorkReceipt};
use crate::state::{ChainState, WorkerId};

/// One job's worth of agreement, ready to be bundled into a block.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct QuorumCertificate {
    pub job_id: JobId,
    pub output_hash: OutputHash,
    pub bounty: u64,
    /// Workers whose receipts agreed on `output_hash` (the winners).
    pub agreeing_workers: Vec<WorkerId>,
    /// Workers whose receipts disagreed (eligible for slashing if the cert finalizes).
    pub dissenting_workers: Vec<WorkerId>,
    /// Trust-weighted fraction of total participating weight that agreed.
    /// Stored * 1e6 to keep canonical encoding stable.
    pub agreement_micro: u32,
}

#[derive(Debug, Error, PartialEq)]
pub enum QuorumError {
    #[error("no receipts for job {0}")]
    Empty(JobId),
    #[error("receipt for wrong job: cert for {expected}, got {actual}")]
    JobMismatch { expected: JobId, actual: JobId },
    #[error("duplicate receipts from same worker for job {0}")]
    Duplicate(JobId),
    #[error("agreement {agreement_micro} below threshold {threshold_micro}")]
    BelowThreshold {
        agreement_micro: u32,
        threshold_micro: u32,
    },
    #[error("worker not registered")]
    UnknownWorker,
}

/// Try to form a quorum certificate from a set of receipts for one job.
///
/// `threshold_micro` is in micros (e.g. 666_667 ≈ 2/3). The winning group's
/// trust-weighted share of the total weight must be at least this value.
pub fn form_quorum(
    envelope: &JobEnvelope,
    receipts: &[WorkReceipt],
    state: &ChainState,
    threshold_micro: u32,
) -> Result<QuorumCertificate, QuorumError> {
    let job_id = envelope.id();
    if receipts.is_empty() {
        return Err(QuorumError::Empty(job_id));
    }

    // Reject duplicates and wrong-job receipts up front.
    let mut seen = std::collections::HashSet::new();
    for r in receipts {
        if r.job_id != job_id {
            return Err(QuorumError::JobMismatch {
                expected: job_id,
                actual: r.job_id,
            });
        }
        if !seen.insert(r.worker) {
            return Err(QuorumError::Duplicate(job_id));
        }
    }

    // Group receipts by output_hash, summing each group's trust weight.
    let mut groups: HashMap<OutputHash, (Vec<WorkerId>, f64)> = HashMap::new();
    let mut total_weight = 0.0_f64;
    for r in receipts {
        let info = state
            .workers
            .get(&r.worker)
            .ok_or(QuorumError::UnknownWorker)?;
        let w = info.weight().max(1.0); // floor at 1.0 so brand-new workers contribute
        total_weight += w;
        let entry = groups
            .entry(r.output_hash)
            .or_insert_with(|| (Vec::new(), 0.0));
        entry.0.push(r.worker);
        entry.1 += w;
    }

    // Pick the winner: highest weight, ties broken by lexicographic output_hash.
    let (winner_hash, (winner_workers, winner_weight)) = groups
        .into_iter()
        .max_by(|a, b| {
            a.1.1
                .partial_cmp(&b.1.1)
                .unwrap_or(std::cmp::Ordering::Equal)
                .then_with(|| a.0.cmp(&b.0))
        })
        .expect("non-empty checked above");

    let agreement = (winner_weight / total_weight * 1_000_000.0).round() as u32;
    if agreement < threshold_micro {
        return Err(QuorumError::BelowThreshold {
            agreement_micro: agreement,
            threshold_micro,
        });
    }

    // Anyone not in winner_workers is a dissenter.
    let winner_set: std::collections::HashSet<_> = winner_workers.iter().copied().collect();
    let mut agreeing = winner_workers;
    agreeing.sort();
    let mut dissenting: Vec<WorkerId> = receipts
        .iter()
        .map(|r| r.worker)
        .filter(|w| !winner_set.contains(w))
        .collect();
    dissenting.sort();

    Ok(QuorumCertificate {
        job_id,
        output_hash: winner_hash,
        bounty: envelope.bounty,
        agreeing_workers: agreeing,
        dissenting_workers: dissenting,
        agreement_micro: agreement,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::envelope::ModelPin;
    use crate::receipt::sign_receipt;
    use ed25519_dalek::SigningKey;
    use rand::rngs::OsRng;

    fn envelope(bounty: u64) -> JobEnvelope {
        JobEnvelope {
            nonce: 1,
            workflow_cid: [0; 32],
            workflow_payload: vec![],
            model: ModelPin::new("m", 0),
            n_replicas: 5,
            bounty,
            deadline_ms: 1000,
        }
    }

    fn receipt(sk: &SigningKey, job_id: JobId, output: u8) -> WorkReceipt {
        let mut r = WorkReceipt {
            job_id,
            worker: WorkerId::from_verifying_key(&sk.verifying_key()),
            output_hash: [output; 32],
            trace_root: [0; 32],
            rubric: None,
            latency_ms: 1,
            signature: vec![],
        };
        sign_receipt(&mut r, sk);
        r
    }

    fn setup(n_workers: usize, stake: u64, trust: f64) -> (Vec<SigningKey>, ChainState) {
        let mut state = ChainState::new();
        let sks: Vec<_> = (0..n_workers)
            .map(|_| SigningKey::generate(&mut OsRng))
            .collect();
        for sk in &sks {
            let id = WorkerId::from_verifying_key(&sk.verifying_key());
            state.register_worker(id, stake, trust);
        }
        (sks, state)
    }

    #[test]
    fn unanimous_quorum() {
        let (sks, state) = setup(5, 100, 1.0);
        let env = envelope(500);
        let receipts: Vec<_> = sks.iter().map(|sk| receipt(sk, env.id(), 0xaa)).collect();
        let cert = form_quorum(&env, &receipts, &state, 666_667).unwrap();
        assert_eq!(cert.agreeing_workers.len(), 5);
        assert_eq!(cert.dissenting_workers.len(), 0);
        assert_eq!(cert.agreement_micro, 1_000_000);
        assert_eq!(cert.bounty, 500);
        assert_eq!(cert.output_hash, [0xaa; 32]);
    }

    #[test]
    fn split_3_2_majority_wins() {
        let (sks, state) = setup(5, 100, 1.0);
        let env = envelope(500);
        let mut receipts = Vec::new();
        for sk in &sks[..3] {
            receipts.push(receipt(sk, env.id(), 0xaa));
        }
        for sk in &sks[3..] {
            receipts.push(receipt(sk, env.id(), 0xbb));
        }
        let cert = form_quorum(&env, &receipts, &state, 500_000).unwrap();
        assert_eq!(cert.output_hash, [0xaa; 32]);
        assert_eq!(cert.agreeing_workers.len(), 3);
        assert_eq!(cert.dissenting_workers.len(), 2);
    }

    #[test]
    fn split_3_2_below_two_thirds_fails() {
        let (sks, state) = setup(5, 100, 1.0);
        let env = envelope(500);
        let mut receipts = Vec::new();
        for sk in &sks[..3] {
            receipts.push(receipt(sk, env.id(), 0xaa));
        }
        for sk in &sks[3..] {
            receipts.push(receipt(sk, env.id(), 0xbb));
        }
        let err = form_quorum(&env, &receipts, &state, 666_667).unwrap_err();
        assert!(matches!(err, QuorumError::BelowThreshold { .. }));
    }

    #[test]
    fn trust_weighting_overrides_count() {
        // 3 low-trust workers vs 2 high-trust workers; high-trust group wins.
        let mut state = ChainState::new();
        let low: Vec<_> = (0..3).map(|_| SigningKey::generate(&mut OsRng)).collect();
        let high: Vec<_> = (0..2).map(|_| SigningKey::generate(&mut OsRng)).collect();
        for sk in &low {
            state.register_worker(WorkerId::from_verifying_key(&sk.verifying_key()), 100, 0.1);
        }
        for sk in &high {
            state.register_worker(WorkerId::from_verifying_key(&sk.verifying_key()), 100, 1.0);
        }
        let env = envelope(500);
        let mut receipts = Vec::new();
        for sk in &low {
            receipts.push(receipt(sk, env.id(), 0xaa)); // low says aa
        }
        for sk in &high {
            receipts.push(receipt(sk, env.id(), 0xbb)); // high says bb
        }
        let cert = form_quorum(&env, &receipts, &state, 500_000).unwrap();
        // high group: 2 * 100 * 1.0 = 200
        // low group: 3 * 100 * 0.1 = 30
        assert_eq!(cert.output_hash, [0xbb; 32]);
        assert_eq!(cert.agreeing_workers.len(), 2);
    }

    #[test]
    fn duplicate_worker_rejected() {
        let (sks, state) = setup(2, 100, 1.0);
        let env = envelope(500);
        let r1 = receipt(&sks[0], env.id(), 0xaa);
        let r2 = receipt(&sks[0], env.id(), 0xbb); // same worker, different output
        let err = form_quorum(&env, &[r1, r2], &state, 500_000).unwrap_err();
        assert!(matches!(err, QuorumError::Duplicate(_)));
    }

    #[test]
    fn wrong_job_rejected() {
        let (sks, state) = setup(1, 100, 1.0);
        let env = envelope(500);
        let other = envelope(999);
        let r = receipt(&sks[0], other.id(), 0xaa);
        let err = form_quorum(&env, &[r], &state, 500_000).unwrap_err();
        assert!(matches!(err, QuorumError::JobMismatch { .. }));
    }

    #[test]
    fn empty_receipts_rejected() {
        let (_sks, state) = setup(1, 100, 1.0);
        let env = envelope(500);
        let err = form_quorum(&env, &[], &state, 500_000).unwrap_err();
        assert!(matches!(err, QuorumError::Empty(_)));
    }

    #[test]
    fn unknown_worker_rejected() {
        let state = ChainState::new(); // no workers registered
        let env = envelope(500);
        let sk = SigningKey::generate(&mut OsRng);
        let r = receipt(&sk, env.id(), 0xaa);
        let err = form_quorum(&env, &[r], &state, 500_000).unwrap_err();
        assert_eq!(err, QuorumError::UnknownWorker);
    }
}
