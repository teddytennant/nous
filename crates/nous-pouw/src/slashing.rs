//! Slashing: catch and punish protocol violations.
//!
//! Two violation classes in v0:
//!
//! - **Equivocation**: a leader signs two different blocks at the same height.
//!   Anyone who collects both signed headers can submit an [`EquivocationProof`]
//!   that slashes the leader's stake.
//! - **Dissent**: a worker who signed a receipt that lost the quorum (i.e. is
//!   in `cert.dissenting_workers`). Slashed by a smaller fraction.

use ed25519_dalek::Verifier;
use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::block::BlockHeader;
use crate::state::WorkerId;

/// What was slashed and why.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SlashEvent {
    pub worker: WorkerId,
    pub kind: SlashKind,
    pub amount: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SlashKind {
    Equivocation,
    Dissent,
}

/// Two block headers at the same height signed by the same leader = proof
/// of equivocation. Anyone can submit; the slash is mandatory.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EquivocationProof {
    pub leader: WorkerId,
    pub a: BlockHeader,
    pub b: BlockHeader,
}

#[derive(Debug, Error, PartialEq)]
pub enum EquivocationError {
    #[error("headers are not at the same height")]
    DifferentHeights,
    #[error("headers are identical (no equivocation)")]
    SameBlock,
    #[error("leader mismatch: a={a}, b={b}")]
    LeaderMismatch { a: String, b: String },
    #[error("signature on header a is invalid")]
    InvalidSigA,
    #[error("signature on header b is invalid")]
    InvalidSigB,
    #[error("invalid leader pubkey")]
    InvalidKey,
}

/// Verify an [`EquivocationProof`] and convert it to a [`SlashEvent`].
///
/// Both block headers must:
///   1. Be at the same height
///   2. Hash to *different* values
///   3. List the same `leader`
///   4. Carry a valid signature by that leader
pub fn detect_equivocation(
    proof: &EquivocationProof,
    slash_amount: u64,
) -> Result<SlashEvent, EquivocationError> {
    if proof.a.height != proof.b.height {
        return Err(EquivocationError::DifferentHeights);
    }
    if proof.a == proof.b {
        return Err(EquivocationError::SameBlock);
    }
    if proof.a.leader != proof.b.leader {
        return Err(EquivocationError::LeaderMismatch {
            a: proof.a.leader.short(),
            b: proof.b.leader.short(),
        });
    }
    if proof.a.leader != proof.leader {
        return Err(EquivocationError::LeaderMismatch {
            a: proof.a.leader.short(),
            b: proof.leader.short(),
        });
    }

    let vk = ed25519_dalek::VerifyingKey::from_bytes(&proof.leader.0)
        .map_err(|_| EquivocationError::InvalidKey)?;

    verify_header_sig(&vk, &proof.a).map_err(|_| EquivocationError::InvalidSigA)?;
    verify_header_sig(&vk, &proof.b).map_err(|_| EquivocationError::InvalidSigB)?;

    Ok(SlashEvent {
        worker: proof.leader,
        kind: SlashKind::Equivocation,
        amount: slash_amount,
    })
}

fn verify_header_sig(
    vk: &ed25519_dalek::VerifyingKey,
    header: &BlockHeader,
) -> Result<(), nous_core::Error> {
    let sig_bytes: [u8; 64] = header
        .signature
        .as_slice()
        .try_into()
        .map_err(|_| nous_core::Error::Crypto("sig length".into()))?;
    let sig = ed25519_dalek::Signature::from_bytes(&sig_bytes);
    let bytes = header.signing_bytes();
    vk.verify(&bytes, &sig)
        .map_err(|e| nous_core::Error::Crypto(format!("{e}")))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::block::{BlockBody, sign_block};
    use ed25519_dalek::SigningKey;
    use rand::rngs::OsRng;

    fn header(sk: &SigningKey, height: u64, prev: [u8; 32], state_root: [u8; 32]) -> BlockHeader {
        let leader = WorkerId::from_verifying_key(&sk.verifying_key());
        let body = BlockBody {
            certs: vec![],
            slashes: vec![],
            mints: vec![],
        };
        let mut hdr = BlockHeader {
            height,
            prev_hash: prev,
            state_root,
            body_hash: body.hash(),
            timestamp_ms: 0,
            leader,
            signature: vec![],
        };
        sign_block(&mut hdr, sk);
        hdr
    }

    #[test]
    fn equivocation_detected() {
        let sk = SigningKey::generate(&mut OsRng);
        let a = header(&sk, 5, [0; 32], [1; 32]);
        let b = header(&sk, 5, [0; 32], [2; 32]); // different state_root
        let proof = EquivocationProof {
            leader: a.leader,
            a,
            b,
        };
        let slash = detect_equivocation(&proof, 1000).unwrap();
        assert_eq!(slash.amount, 1000);
        assert_eq!(slash.kind, SlashKind::Equivocation);
    }

    #[test]
    fn equivocation_rejects_different_heights() {
        let sk = SigningKey::generate(&mut OsRng);
        let a = header(&sk, 5, [0; 32], [1; 32]);
        let b = header(&sk, 6, [0; 32], [1; 32]);
        let proof = EquivocationProof {
            leader: a.leader,
            a,
            b,
        };
        assert_eq!(
            detect_equivocation(&proof, 1).unwrap_err(),
            EquivocationError::DifferentHeights
        );
    }

    #[test]
    fn equivocation_rejects_same_block() {
        let sk = SigningKey::generate(&mut OsRng);
        let a = header(&sk, 5, [0; 32], [1; 32]);
        let b = a.clone();
        let proof = EquivocationProof {
            leader: a.leader,
            a,
            b,
        };
        assert_eq!(
            detect_equivocation(&proof, 1).unwrap_err(),
            EquivocationError::SameBlock
        );
    }

    #[test]
    fn equivocation_rejects_invalid_signature() {
        let sk = SigningKey::generate(&mut OsRng);
        let mut a = header(&sk, 5, [0; 32], [1; 32]);
        let mut b = header(&sk, 5, [0; 32], [2; 32]);
        a.signature = vec![0u8; 64];
        b.signature = vec![0u8; 64];
        let proof = EquivocationProof {
            leader: a.leader,
            a,
            b,
        };
        assert_eq!(
            detect_equivocation(&proof, 1).unwrap_err(),
            EquivocationError::InvalidSigA
        );
    }

    #[test]
    fn equivocation_rejects_leader_mismatch() {
        let sk1 = SigningKey::generate(&mut OsRng);
        let sk2 = SigningKey::generate(&mut OsRng);
        let a = header(&sk1, 5, [0; 32], [1; 32]);
        let b = header(&sk2, 5, [0; 32], [2; 32]);
        let proof = EquivocationProof {
            leader: a.leader,
            a,
            b,
        };
        assert!(matches!(
            detect_equivocation(&proof, 1).unwrap_err(),
            EquivocationError::LeaderMismatch { .. }
        ));
    }
}
