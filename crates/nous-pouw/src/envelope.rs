//! Job envelope: the immutable specification of a single unit of useful work.
//!
//! The envelope is content-addressed by [`JobId`] = blake3 of its canonical
//! serialization, so any worker that has the envelope can verify the job id
//! locally without consulting a registry.

use serde::{Deserialize, Serialize};

/// Content-addressed job identifier.
///
/// `job_id = blake3(canonical_json(envelope_without_id))`. Two workers with
/// the same envelope always derive the same id; a tampered envelope yields
/// a different id and is dropped by validators.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct JobId(pub [u8; 32]);

impl JobId {
    pub fn as_bytes(&self) -> &[u8; 32] {
        &self.0
    }

    pub fn to_hex(self) -> String {
        hex::encode(self.0)
    }
}

impl std::fmt::Display for JobId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", &self.to_hex()[..16])
    }
}

/// Pinned model + sampling parameters that make the work reproducible across
/// workers (insofar as the underlying inference engine respects the seed).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ModelPin {
    /// Stable model identifier (e.g. "claude-haiku-4-5-20251001").
    pub model_id: String,
    /// Sampling temperature * 1000 (integer to keep canonical encoding stable).
    pub temperature_milli: u16,
    /// Top-p * 1000.
    pub top_p_milli: u16,
    /// Sampling seed; combined with `JobId` to produce per-worker deterministic
    /// streams when the underlying executor supports seeding.
    pub seed: u64,
}

impl ModelPin {
    pub fn new(model_id: impl Into<String>, seed: u64) -> Self {
        Self {
            model_id: model_id.into(),
            temperature_milli: 0,
            top_p_milli: 1000,
            seed,
        }
    }
}

/// A unit of useful work to be executed by N redundant workers.
///
/// `workflow_cid` is opaque to consensus — the [`crate::network::Network`]
/// (or a content-addressable store layered on it) is responsible for resolving
/// it to the actual workflow definition. In v0 sim we pass an inline payload.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct JobEnvelope {
    /// Caller-assigned monotonic nonce (prevents replay of the same workflow
    /// at different times producing the same id by accident).
    pub nonce: u64,
    /// Content id of the workflow definition (blake3 of canonical JSON).
    pub workflow_cid: [u8; 32],
    /// Inline workflow payload. v0 sim only — v1 fetches by `workflow_cid`.
    pub workflow_payload: Vec<u8>,
    /// Model + seed pinning.
    pub model: ModelPin,
    /// How many independent workers must execute this job for quorum.
    pub n_replicas: u8,
    /// Bounty paid (split across the winning quorum).
    pub bounty: u64,
    /// Wall-clock deadline for receipts to be accepted, in milliseconds since
    /// the chain's genesis.
    pub deadline_ms: u64,
}

impl JobEnvelope {
    /// Compute the content-addressed [`JobId`].
    pub fn id(&self) -> JobId {
        let bytes = serde_json::to_vec(self).expect("JobEnvelope is always JSON-serializable");
        let hash = blake3::hash(&bytes);
        JobId(*hash.as_bytes())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample() -> JobEnvelope {
        JobEnvelope {
            nonce: 42,
            workflow_cid: [1u8; 32],
            workflow_payload: b"hello".to_vec(),
            model: ModelPin::new("claude-haiku-4-5-20251001", 1234),
            n_replicas: 5,
            bounty: 100,
            deadline_ms: 60_000,
        }
    }

    #[test]
    fn job_id_is_deterministic() {
        let a = sample().id();
        let b = sample().id();
        assert_eq!(a, b);
    }

    #[test]
    fn job_id_changes_with_payload() {
        let mut a = sample();
        let original = a.id();
        a.workflow_payload = b"different".to_vec();
        assert_ne!(original, a.id());
    }

    #[test]
    fn job_id_changes_with_model() {
        let mut a = sample();
        let original = a.id();
        a.model.seed = 9999;
        assert_ne!(original, a.id());
    }

    #[test]
    fn job_id_changes_with_nonce() {
        let mut a = sample();
        let original = a.id();
        a.nonce += 1;
        assert_ne!(original, a.id());
    }

    #[test]
    fn job_id_hex_round_trip() {
        let id = sample().id();
        let hex = id.to_hex();
        assert_eq!(hex.len(), 64);
        let bytes = hex::decode(&hex).unwrap();
        assert_eq!(&bytes[..], id.as_bytes());
    }

    #[test]
    fn job_id_serde_round_trip() {
        let id = sample().id();
        let json = serde_json::to_string(&id).unwrap();
        let back: JobId = serde_json::from_str(&json).unwrap();
        assert_eq!(id, back);
    }

    #[test]
    fn model_pin_new_defaults_temperature_zero() {
        let m = ModelPin::new("x", 7);
        assert_eq!(m.temperature_milli, 0);
        assert_eq!(m.top_p_milli, 1000);
        assert_eq!(m.seed, 7);
    }
}
