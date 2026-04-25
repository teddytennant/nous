//! Worker receipts: what one worker says it produced for one job.
//!
//! A receipt has two phases on the wire:
//!
//! 1. **Commit** — worker publishes [`ReceiptCommitment`] = blake3(receipt || nonce).
//!    No payload yet, so other workers cannot copy it.
//! 2. **Reveal** — after the dispatch deadline, worker publishes the full
//!    [`WorkReceipt`] + nonce. Validators check that
//!    `commitment == blake3(receipt_bytes || nonce)` for each prior commit.
//!
//! Without commit-reveal, a Sybil swarm could wait for one honest worker to
//! publish, copy its `output_hash`, and free-ride into the quorum.

use ed25519_dalek::{Signer as DalekSigner, SigningKey, VerifyingKey};
use serde::{Deserialize, Serialize};

use crate::envelope::JobId;
use crate::state::WorkerId;

/// 32-byte digest of the canonical output of a job.
///
/// Two workers that ran the same job with the same model + seed against a
/// deterministic tool stack should produce the same `OutputHash`. When they
/// don't, quorum naturally splits and consensus picks the winning subset.
pub type OutputHash = [u8; 32];

/// 32-byte Merkle root over the trace of intermediate steps (LLM calls,
/// tool calls). Used by optimistic fraud-proof systems in v1+; in v0 we
/// just include it in the receipt and hash it into the commitment.
pub type TraceRoot = [u8; 32];

/// External rubric score from a judging step (0.0 – 1.0). Optional —
/// pure-redundancy jobs leave it at `None`.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct RubricScore(pub f32);

/// What a worker claims to have produced for one job.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct WorkReceipt {
    pub job_id: JobId,
    pub worker: WorkerId,
    pub output_hash: OutputHash,
    pub trace_root: TraceRoot,
    pub rubric: Option<RubricScore>,
    pub latency_ms: u64,
    /// ed25519 signature over the canonical receipt body (everything above).
    pub signature: Vec<u8>,
}

impl WorkReceipt {
    /// Canonical bytes that the worker signs.
    fn signing_bytes(&self) -> Vec<u8> {
        // Serialize a copy with empty signature so the signature itself is
        // not in the signed payload (avoids the chicken-and-egg).
        let unsigned = WorkReceipt {
            signature: Vec::new(),
            ..self.clone()
        };
        serde_json::to_vec(&unsigned).expect("WorkReceipt is JSON-serializable")
    }

    /// Verify the receipt's signature against its claimed worker pubkey.
    pub fn verify(&self) -> Result<(), nous_core::Error> {
        let vk = VerifyingKey::from_bytes(&self.worker.0)
            .map_err(|e| nous_core::Error::Crypto(format!("bad worker pubkey: {e}")))?;
        let sig_bytes: [u8; 64] = self
            .signature
            .as_slice()
            .try_into()
            .map_err(|_| nous_core::Error::Crypto("signature must be 64 bytes".into()))?;
        let sig = ed25519_dalek::Signature::from_bytes(&sig_bytes);
        let bytes = self.signing_bytes();
        ed25519_dalek::Verifier::verify(&vk, &bytes, &sig)
            .map_err(|e| nous_core::Error::Crypto(format!("receipt sig invalid: {e}")))
    }

    /// Commitment used in the commit-reveal flow.
    pub fn commitment(&self, nonce: &[u8; 32]) -> ReceiptCommitment {
        let mut h = blake3::Hasher::new();
        h.update(&self.signing_bytes());
        h.update(nonce);
        ReceiptCommitment {
            job_id: self.job_id,
            worker: self.worker,
            digest: *h.finalize().as_bytes(),
        }
    }
}

/// Commitment phase: lock in *what the worker will reveal* without revealing
/// the output_hash itself yet.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReceiptCommitment {
    pub job_id: JobId,
    pub worker: WorkerId,
    pub digest: [u8; 32],
}

/// Sign a receipt body. The receipt's `signature` field is overwritten.
pub fn sign_receipt(receipt: &mut WorkReceipt, signing: &SigningKey) {
    let bytes = receipt.signing_bytes();
    let sig = signing.sign(&bytes);
    receipt.signature = sig.to_bytes().to_vec();
}

#[cfg(test)]
mod tests {
    use super::*;
    use ed25519_dalek::SigningKey;
    use rand::rngs::OsRng;

    fn make_receipt(sk: &SigningKey, output: u8) -> WorkReceipt {
        let mut r = WorkReceipt {
            job_id: JobId([7u8; 32]),
            worker: WorkerId::from_verifying_key(&sk.verifying_key()),
            output_hash: [output; 32],
            trace_root: [0xab; 32],
            rubric: Some(RubricScore(0.9)),
            latency_ms: 123,
            signature: Vec::new(),
        };
        sign_receipt(&mut r, sk);
        r
    }

    #[test]
    fn sign_verify_round_trip() {
        let sk = SigningKey::generate(&mut OsRng);
        let r = make_receipt(&sk, 1);
        r.verify().expect("verify ok");
    }

    #[test]
    fn verify_rejects_tampered_output() {
        let sk = SigningKey::generate(&mut OsRng);
        let mut r = make_receipt(&sk, 1);
        r.output_hash = [0xff; 32];
        assert!(r.verify().is_err());
    }

    #[test]
    fn verify_rejects_wrong_worker_key() {
        let sk1 = SigningKey::generate(&mut OsRng);
        let sk2 = SigningKey::generate(&mut OsRng);
        let mut r = make_receipt(&sk1, 1);
        r.worker = WorkerId::from_verifying_key(&sk2.verifying_key());
        assert!(r.verify().is_err());
    }

    #[test]
    fn verify_rejects_bad_signature_length() {
        let sk = SigningKey::generate(&mut OsRng);
        let mut r = make_receipt(&sk, 1);
        r.signature.truncate(10);
        assert!(r.verify().is_err());
    }

    #[test]
    fn commitment_changes_with_nonce() {
        let sk = SigningKey::generate(&mut OsRng);
        let r = make_receipt(&sk, 1);
        let c1 = r.commitment(&[0u8; 32]);
        let c2 = r.commitment(&[1u8; 32]);
        assert_ne!(c1.digest, c2.digest);
    }

    #[test]
    fn commitment_changes_with_output() {
        let sk = SigningKey::generate(&mut OsRng);
        let r1 = make_receipt(&sk, 1);
        let r2 = make_receipt(&sk, 2);
        assert_ne!(r1.commitment(&[0u8; 32]), r2.commitment(&[0u8; 32]));
    }

    #[test]
    fn commitment_carries_metadata() {
        let sk = SigningKey::generate(&mut OsRng);
        let r = make_receipt(&sk, 1);
        let c = r.commitment(&[0u8; 32]);
        assert_eq!(c.job_id, r.job_id);
        assert_eq!(c.worker, r.worker);
    }
}
