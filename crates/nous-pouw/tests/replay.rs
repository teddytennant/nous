//! Receipt commit-reveal blocks "see what others said and copy it" attacks.
//!
//! The on-wire flow is: workers publish [`ReceiptCommitment`] first; later
//! they reveal the full receipt. Validators only accept reveals whose
//! `commitment(receipt, nonce)` matches a prior commit *from the same
//! worker* on this job. A copying attacker sees the first reveal but cannot
//! retroactively publish a commitment matching it.

use ed25519_dalek::SigningKey;
use nous_pouw::envelope::{JobEnvelope, JobId, ModelPin};
use nous_pouw::receipt::{WorkReceipt, sign_receipt};
use nous_pouw::state::WorkerId;
use rand::rngs::OsRng;

fn job() -> JobEnvelope {
    JobEnvelope {
        nonce: 1,
        workflow_cid: [0; 32],
        workflow_payload: b"hello".to_vec(),
        model: ModelPin::new("m", 0),
        n_replicas: 3,
        bounty: 100,
        deadline_ms: 1_000,
    }
}

fn signed_receipt(sk: &SigningKey, job_id: JobId, output_byte: u8) -> WorkReceipt {
    let mut r = WorkReceipt {
        job_id,
        worker: WorkerId::from_verifying_key(&sk.verifying_key()),
        output_hash: [output_byte; 32],
        trace_root: [0; 32],
        rubric: None,
        latency_ms: 1,
        signature: vec![],
    };
    sign_receipt(&mut r, sk);
    r
}

#[test]
fn commit_then_reveal_matches() {
    let sk = SigningKey::generate(&mut OsRng);
    let r = signed_receipt(&sk, job().id(), 0xab);
    let nonce = [7u8; 32];
    let commit = r.commitment(&nonce);
    // Reveal phase: validator recomputes commitment from revealed receipt+nonce.
    let recomputed = r.commitment(&nonce);
    assert_eq!(commit, recomputed);
}

#[test]
fn copy_attack_caught_by_commit_mismatch() {
    let honest_sk = SigningKey::generate(&mut OsRng);
    let attacker_sk = SigningKey::generate(&mut OsRng);
    let env = job();

    // Honest worker commits + reveals.
    let honest_r = signed_receipt(&honest_sk, env.id(), 0xab);
    let honest_nonce = [1u8; 32];
    let honest_commit = honest_r.commitment(&honest_nonce);

    // Attacker observes honest's revealed receipt and tries to publish their
    // own copy. Their pre-deadline commitment was for some *other* output
    // (because they hadn't seen honest yet). So when they reveal a copy of
    // honest's output, their commitment doesn't match.
    let attacker_pre = signed_receipt(&attacker_sk, env.id(), 0x99);
    let attacker_nonce = [2u8; 32];
    let attacker_commit_before = attacker_pre.commitment(&attacker_nonce);

    let attacker_copy = signed_receipt(&attacker_sk, env.id(), 0xab);
    let attacker_recomputed = attacker_copy.commitment(&attacker_nonce);

    // Verifier check: prior commit != recomputed commit → reveal rejected.
    assert_ne!(attacker_commit_before, attacker_recomputed);
    // (And the honest worker's reveal would still match.)
    assert_eq!(honest_commit, honest_r.commitment(&honest_nonce));
}

#[test]
fn nonce_must_be_consistent_across_commit_and_reveal() {
    let sk = SigningKey::generate(&mut OsRng);
    let r = signed_receipt(&sk, job().id(), 0xab);
    let commit = r.commitment(&[1u8; 32]);
    let with_other_nonce = r.commitment(&[2u8; 32]);
    assert_ne!(commit, with_other_nonce);
}

#[test]
fn impersonator_signature_fails_verification() {
    // Even if an attacker copies the output_hash, they cannot forge the
    // ed25519 signature of the original worker.
    let honest_sk = SigningKey::generate(&mut OsRng);
    let attacker_sk = SigningKey::generate(&mut OsRng);
    let env = job();

    let mut forged = signed_receipt(&attacker_sk, env.id(), 0xab);
    // Pretend it came from the honest worker, keeping attacker's signature.
    forged.worker = WorkerId::from_verifying_key(&honest_sk.verifying_key());
    assert!(forged.verify().is_err());
}
