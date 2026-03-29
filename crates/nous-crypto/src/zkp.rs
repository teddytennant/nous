//! Zero-knowledge proof primitives.
//!
//! Schnorr proofs of knowledge and Pedersen commitments over the Ristretto group.

use curve25519_dalek::constants::RISTRETTO_BASEPOINT_POINT;
use curve25519_dalek::ristretto::{CompressedRistretto, RistrettoPoint};
use curve25519_dalek::scalar::Scalar;
use rand::rngs::OsRng;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha512};

use nous_core::{Error, Result};

fn pedersen_h() -> RistrettoPoint {
    let hash: [u8; 64] = Sha512::digest(b"nous-pedersen-generator-H-v1").into();
    RistrettoPoint::from_uniform_bytes(&hash)
}

fn challenge_scalar(data: &[u8]) -> Scalar {
    let hash: [u8; 64] = Sha512::digest(data).into();
    Scalar::from_bytes_mod_order_wide(&hash)
}

// ── Schnorr Proof ──────────────────────────────────────────────

/// Non-interactive Schnorr proof of knowledge of a discrete logarithm.
///
/// Proves: "I know `x` such that `Y = xG`" without revealing `x`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SchnorrProof {
    pub commitment: [u8; 32],
    pub response: [u8; 32],
}

/// Generate a random Ristretto keypair for Schnorr proofs.
///
/// Returns `(secret_bytes, public_bytes)`.
pub fn schnorr_keygen() -> ([u8; 32], [u8; 32]) {
    let secret = Scalar::random(&mut OsRng);
    let public = (secret * RISTRETTO_BASEPOINT_POINT).compress().to_bytes();
    (secret.to_bytes(), public)
}

impl SchnorrProof {
    /// Prove knowledge of the discrete log of `public` w.r.t. the Ristretto basepoint.
    pub fn prove(secret: &[u8; 32], public: &[u8; 32], message: &[u8]) -> Self {
        let x = Scalar::from_bytes_mod_order(*secret);
        let k = Scalar::random(&mut OsRng);
        let r = (k * RISTRETTO_BASEPOINT_POINT).compress().to_bytes();

        let mut input = Vec::with_capacity(64 + message.len());
        input.extend_from_slice(&r);
        input.extend_from_slice(public);
        input.extend_from_slice(message);
        let c = challenge_scalar(&input);

        let s = k + c * x;

        Self {
            commitment: r,
            response: s.to_bytes(),
        }
    }

    /// Verify a Schnorr proof against a public key and message.
    pub fn verify(&self, public: &[u8; 32], message: &[u8]) -> bool {
        let y = match CompressedRistretto(*public).decompress() {
            Some(p) => p,
            None => return false,
        };

        let r = match CompressedRistretto(self.commitment).decompress() {
            Some(p) => p,
            None => return false,
        };

        let s = Scalar::from_bytes_mod_order(self.response);

        let mut input = Vec::with_capacity(64 + message.len());
        input.extend_from_slice(&self.commitment);
        input.extend_from_slice(public);
        input.extend_from_slice(message);
        let c = challenge_scalar(&input);

        s * RISTRETTO_BASEPOINT_POINT == r + c * y
    }
}

// ── Pedersen Commitment ────────────────────────────────────────

/// A Pedersen commitment: C = vG + rH.
///
/// Hiding (can't learn v) and binding (can't change v after committing).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PedersenCommitment {
    pub commitment: [u8; 32],
}

/// Opening for a Pedersen commitment.
#[derive(Debug, Clone)]
pub struct PedersenOpening {
    pub value: u64,
    pub blinding: [u8; 32],
}

impl PedersenCommitment {
    /// Commit to a value with a random blinding factor.
    pub fn commit(value: u64) -> (Self, PedersenOpening) {
        let blinding = Scalar::random(&mut OsRng);
        let point = Scalar::from(value) * RISTRETTO_BASEPOINT_POINT + blinding * pedersen_h();

        (
            Self {
                commitment: point.compress().to_bytes(),
            },
            PedersenOpening {
                value,
                blinding: blinding.to_bytes(),
            },
        )
    }

    /// Commit with a specific blinding factor.
    pub fn commit_with(value: u64, blinding: &[u8; 32]) -> Self {
        let r = Scalar::from_bytes_mod_order(*blinding);
        let point = Scalar::from(value) * RISTRETTO_BASEPOINT_POINT + r * pedersen_h();

        Self {
            commitment: point.compress().to_bytes(),
        }
    }

    /// Verify that an opening matches this commitment.
    pub fn verify(&self, opening: &PedersenOpening) -> bool {
        let expected = Self::commit_with(opening.value, &opening.blinding);
        self.commitment == expected.commitment
    }

    /// Homomorphic addition: C(v1,r1) + C(v2,r2) = C(v1+v2, r1+r2).
    pub fn add(&self, other: &Self) -> Result<Self> {
        let a = CompressedRistretto(self.commitment)
            .decompress()
            .ok_or_else(|| Error::Crypto("invalid commitment point".into()))?;

        let b = CompressedRistretto(other.commitment)
            .decompress()
            .ok_or_else(|| Error::Crypto("invalid commitment point".into()))?;

        Ok(Self {
            commitment: (a + b).compress().to_bytes(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ── Schnorr tests ──

    #[test]
    fn schnorr_prove_and_verify() {
        let (secret, public) = schnorr_keygen();
        let proof = SchnorrProof::prove(&secret, &public, b"test message");
        assert!(proof.verify(&public, b"test message"));
    }

    #[test]
    fn schnorr_rejects_wrong_message() {
        let (secret, public) = schnorr_keygen();
        let proof = SchnorrProof::prove(&secret, &public, b"original");
        assert!(!proof.verify(&public, b"tampered"));
    }

    #[test]
    fn schnorr_rejects_wrong_key() {
        let (secret, public) = schnorr_keygen();
        let (_, wrong) = schnorr_keygen();
        let proof = SchnorrProof::prove(&secret, &public, b"test");
        assert!(!proof.verify(&wrong, b"test"));
    }

    #[test]
    fn schnorr_unique_per_invocation() {
        let (secret, public) = schnorr_keygen();
        let p1 = SchnorrProof::prove(&secret, &public, b"test");
        let p2 = SchnorrProof::prove(&secret, &public, b"test");
        assert_ne!(p1.commitment, p2.commitment);
    }

    #[test]
    fn schnorr_serializes() {
        let (secret, public) = schnorr_keygen();
        let proof = SchnorrProof::prove(&secret, &public, b"serde");
        let json = serde_json::to_string(&proof).unwrap();
        let restored: SchnorrProof = serde_json::from_str(&json).unwrap();
        assert!(restored.verify(&public, b"serde"));
    }

    #[test]
    fn schnorr_empty_message() {
        let (secret, public) = schnorr_keygen();
        let proof = SchnorrProof::prove(&secret, &public, b"");
        assert!(proof.verify(&public, b""));
    }

    #[test]
    fn schnorr_large_message() {
        let (secret, public) = schnorr_keygen();
        let big = vec![0xAB; 10_000];
        let proof = SchnorrProof::prove(&secret, &public, &big);
        assert!(proof.verify(&public, &big));
    }

    // ── Pedersen tests ──

    #[test]
    fn pedersen_commit_and_verify() {
        let (c, opening) = PedersenCommitment::commit(42);
        assert!(c.verify(&opening));
    }

    #[test]
    fn pedersen_rejects_wrong_value() {
        let (c, mut opening) = PedersenCommitment::commit(42);
        opening.value = 43;
        assert!(!c.verify(&opening));
    }

    #[test]
    fn pedersen_rejects_wrong_blinding() {
        let (c, opening) = PedersenCommitment::commit(42);
        let wrong = PedersenOpening {
            value: opening.value,
            blinding: [0xFF; 32],
        };
        assert!(!c.verify(&wrong));
    }

    #[test]
    fn pedersen_hiding() {
        let (c1, _) = PedersenCommitment::commit(100);
        let (c2, _) = PedersenCommitment::commit(100);
        assert_ne!(c1.commitment, c2.commitment);
    }

    #[test]
    fn pedersen_binding() {
        let (c, opening) = PedersenCommitment::commit(100);
        let fake = PedersenOpening {
            value: 200,
            blinding: opening.blinding,
        };
        assert!(!c.verify(&fake));
    }

    #[test]
    fn pedersen_homomorphic_addition() {
        let r1 = Scalar::random(&mut OsRng);
        let r2 = Scalar::random(&mut OsRng);

        let c1 = PedersenCommitment::commit_with(30, &r1.to_bytes());
        let c2 = PedersenCommitment::commit_with(12, &r2.to_bytes());
        let sum = c1.add(&c2).unwrap();

        let r_sum = (r1 + r2).to_bytes();
        let expected = PedersenCommitment::commit_with(42, &r_sum);

        assert_eq!(sum.commitment, expected.commitment);
    }

    #[test]
    fn pedersen_zero() {
        let (c, opening) = PedersenCommitment::commit(0);
        assert!(c.verify(&opening));
    }

    #[test]
    fn pedersen_max_value() {
        let (c, opening) = PedersenCommitment::commit(u64::MAX);
        assert!(c.verify(&opening));
    }
}
