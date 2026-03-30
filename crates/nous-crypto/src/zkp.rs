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

// ── Equality Proof ────────────────────────────────────────────

/// Proof that two Pedersen commitments contain the same value.
///
/// Given C1 = v*G + r1*H and C2 = v*G + r2*H, proves they share `v`
/// without revealing `v`, `r1`, or `r2`.
///
/// The key insight: C1 - C2 = (r1-r2)*H, so proving knowledge of the
/// discrete log of (C1-C2) w.r.t. H proves equal values.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EqualityProof {
    pub commitment: [u8; 32],
    pub response: [u8; 32],
}

impl EqualityProof {
    /// Prove that two commitments contain the same value.
    pub fn prove(value: u64, blinding_1: &[u8; 32], blinding_2: &[u8; 32]) -> Self {
        let r1 = Scalar::from_bytes_mod_order(*blinding_1);
        let r2 = Scalar::from_bytes_mod_order(*blinding_2);
        let diff = r1 - r2;

        let k = Scalar::random(&mut OsRng);
        let h = pedersen_h();
        let r_commit = (k * h).compress().to_bytes();

        // Challenge includes both commitments and the random commitment.
        let c1 = PedersenCommitment::commit_with(value, blinding_1);
        let c2 = PedersenCommitment::commit_with(value, blinding_2);

        let mut input = Vec::with_capacity(96);
        input.extend_from_slice(&r_commit);
        input.extend_from_slice(&c1.commitment);
        input.extend_from_slice(&c2.commitment);
        let c = challenge_scalar(&input);

        let s = k + c * diff;

        Self {
            commitment: r_commit,
            response: s.to_bytes(),
        }
    }

    /// Verify that two commitments contain the same value.
    pub fn verify(&self, commit_1: &PedersenCommitment, commit_2: &PedersenCommitment) -> bool {
        let c1 = match CompressedRistretto(commit_1.commitment).decompress() {
            Some(p) => p,
            None => return false,
        };
        let c2 = match CompressedRistretto(commit_2.commitment).decompress() {
            Some(p) => p,
            None => return false,
        };
        let r = match CompressedRistretto(self.commitment).decompress() {
            Some(p) => p,
            None => return false,
        };

        let h = pedersen_h();
        let s = Scalar::from_bytes_mod_order(self.response);

        let mut input = Vec::with_capacity(96);
        input.extend_from_slice(&self.commitment);
        input.extend_from_slice(&commit_1.commitment);
        input.extend_from_slice(&commit_2.commitment);
        let c = challenge_scalar(&input);

        // Verify: s*H == R + c*(C1 - C2)
        s * h == r + c * (c1 - c2)
    }
}

// ── Disjunctive (OR) Proof ───────────────────────────────────

/// A 1-of-N disjunctive Schnorr proof.
///
/// Proves knowledge of the discrete log for ONE of a set of public keys
/// without revealing which one. Uses the Cramer-Damgard-Schoenmakers
/// technique.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrProof {
    /// Per-branch commitments.
    pub commitments: Vec<[u8; 32]>,
    /// Per-branch challenges.
    pub challenges: Vec<[u8; 32]>,
    /// Per-branch responses.
    pub responses: Vec<[u8; 32]>,
}

impl OrProof {
    /// Prove knowledge of the secret for public_keys[real_index].
    ///
    /// # Panics
    /// Panics if `real_index >= public_keys.len()` or if `public_keys` is empty.
    pub fn prove(
        secret: &[u8; 32],
        real_index: usize,
        public_keys: &[[u8; 32]],
        message: &[u8],
    ) -> Result<Self> {
        let n = public_keys.len();
        if n == 0 {
            return Err(Error::Crypto("empty public key set".into()));
        }
        if real_index >= n {
            return Err(Error::Crypto("real_index out of bounds".into()));
        }

        let x = Scalar::from_bytes_mod_order(*secret);

        let mut commitments = vec![[0u8; 32]; n];
        let mut challenges = vec![[0u8; 32]; n];
        let mut responses = vec![[0u8; 32]; n];

        // For all fake branches, simulate the proof.
        for i in 0..n {
            if i == real_index {
                continue;
            }
            let ci = Scalar::random(&mut OsRng);
            let si = Scalar::random(&mut OsRng);

            let yi = match CompressedRistretto(public_keys[i]).decompress() {
                Some(p) => p,
                None => return Err(Error::Crypto("invalid public key".into())),
            };

            // R_i = s_i*G - c_i*Y_i (simulated commitment).
            let ri = si * RISTRETTO_BASEPOINT_POINT - ci * yi;

            commitments[i] = ri.compress().to_bytes();
            challenges[i] = ci.to_bytes();
            responses[i] = si.to_bytes();
        }

        // For the real branch, create an honest commitment.
        let k = Scalar::random(&mut OsRng);
        commitments[real_index] = (k * RISTRETTO_BASEPOINT_POINT).compress().to_bytes();

        // Compute the overall challenge.
        let mut hash_input = Vec::new();
        for c in &commitments {
            hash_input.extend_from_slice(c);
        }
        for pk in public_keys {
            hash_input.extend_from_slice(pk);
        }
        hash_input.extend_from_slice(message);
        let overall_c = challenge_scalar(&hash_input);

        // Real branch challenge = overall_c - sum of fake challenges.
        let fake_sum: Scalar = (0..n)
            .filter(|&i| i != real_index)
            .map(|i| Scalar::from_bytes_mod_order(challenges[i]))
            .sum();

        let real_c = overall_c - fake_sum;
        challenges[real_index] = real_c.to_bytes();
        responses[real_index] = (k + real_c * x).to_bytes();

        Ok(Self {
            commitments,
            challenges,
            responses,
        })
    }

    /// Verify a 1-of-N OR proof.
    pub fn verify(&self, public_keys: &[[u8; 32]], message: &[u8]) -> bool {
        let n = public_keys.len();
        if self.commitments.len() != n || self.challenges.len() != n || self.responses.len() != n {
            return false;
        }
        if n == 0 {
            return false;
        }

        // Verify each branch: s_i*G == R_i + c_i*Y_i.
        #[allow(clippy::needless_range_loop)]
        for i in 0..n {
            let ri = match CompressedRistretto(self.commitments[i]).decompress() {
                Some(p) => p,
                None => return false,
            };
            let yi = match CompressedRistretto(public_keys[i]).decompress() {
                Some(p) => p,
                None => return false,
            };
            let si = Scalar::from_bytes_mod_order(self.responses[i]);
            let ci = Scalar::from_bytes_mod_order(self.challenges[i]);

            if si * RISTRETTO_BASEPOINT_POINT != ri + ci * yi {
                return false;
            }
        }

        // Verify overall challenge = sum of per-branch challenges.
        let mut hash_input = Vec::new();
        for c in &self.commitments {
            hash_input.extend_from_slice(c);
        }
        for pk in public_keys {
            hash_input.extend_from_slice(pk);
        }
        hash_input.extend_from_slice(message);
        let expected = challenge_scalar(&hash_input);

        let actual: Scalar = self
            .challenges
            .iter()
            .map(|c| Scalar::from_bytes_mod_order(*c))
            .sum();

        expected == actual
    }
}

// ── Set Membership Proof ──────────────────────────────────────

/// Proof that a committed value belongs to a known set.
///
/// Given a Pedersen commitment C = v*G + r*H and a public set {v1, v2, ..., vn},
/// proves that the committed value v is one of {v1, ..., vn} without
/// revealing which one.
///
/// This uses a disjunctive technique: for each possible value vi in the set,
/// compute C - vi*G = (v-vi)*G + r*H. For the real value, this equals r*H.
/// Proving knowledge of r w.r.t. H for one of these "shifted" commitments
/// proves membership.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SetMembershipProof {
    pub commitments: Vec<[u8; 32]>,
    pub challenges: Vec<[u8; 32]>,
    pub responses: Vec<[u8; 32]>,
}

impl SetMembershipProof {
    /// Prove that a committed value belongs to `valid_values`.
    pub fn prove(value: u64, blinding: &[u8; 32], valid_values: &[u64]) -> Result<Self> {
        if valid_values.is_empty() {
            return Err(Error::Crypto("empty value set".into()));
        }

        let real_index = valid_values
            .iter()
            .position(|&v| v == value)
            .ok_or_else(|| Error::Crypto("value not in set".into()))?;

        let r = Scalar::from_bytes_mod_order(*blinding);
        let h = pedersen_h();
        let n = valid_values.len();

        let commitment_point = Scalar::from(value) * RISTRETTO_BASEPOINT_POINT + r * h;

        let mut proof_commitments = vec![[0u8; 32]; n];
        let mut challenges = vec![[0u8; 32]; n];
        let mut responses = vec![[0u8; 32]; n];

        // For fake branches: simulate proof of knowledge of DL of (C - vi*G) w.r.t. H.
        for i in 0..n {
            if i == real_index {
                continue;
            }
            let ci = Scalar::random(&mut OsRng);
            let si = Scalar::random(&mut OsRng);

            // D_i = C - vi*G (should equal (v-vi)*G + r*H for wrong vi).
            let di = commitment_point - Scalar::from(valid_values[i]) * RISTRETTO_BASEPOINT_POINT;

            // R_i = s_i*H - c_i*D_i.
            let ri = si * h - ci * di;

            proof_commitments[i] = ri.compress().to_bytes();
            challenges[i] = ci.to_bytes();
            responses[i] = si.to_bytes();
        }

        // Real branch: C - v_real*G = r*H, prove knowledge of r.
        let k = Scalar::random(&mut OsRng);
        proof_commitments[real_index] = (k * h).compress().to_bytes();

        // Overall challenge.
        let mut hash_input = Vec::new();
        for c in &proof_commitments {
            hash_input.extend_from_slice(c);
        }
        hash_input.extend_from_slice(&commitment_point.compress().to_bytes());
        for v in valid_values {
            hash_input.extend_from_slice(&v.to_le_bytes());
        }
        let overall = challenge_scalar(&hash_input);

        let fake_sum: Scalar = (0..n)
            .filter(|&i| i != real_index)
            .map(|i| Scalar::from_bytes_mod_order(challenges[i]))
            .sum();

        let real_c = overall - fake_sum;
        challenges[real_index] = real_c.to_bytes();
        responses[real_index] = (k + real_c * r).to_bytes();

        Ok(Self {
            commitments: proof_commitments,
            challenges,
            responses,
        })
    }

    /// Verify that the committed value in `pedersen_commitment` belongs to `valid_values`.
    pub fn verify(&self, pedersen_commitment: &PedersenCommitment, valid_values: &[u64]) -> bool {
        let n = valid_values.len();
        if self.commitments.len() != n || self.challenges.len() != n || self.responses.len() != n {
            return false;
        }
        if n == 0 {
            return false;
        }

        let c_point = match CompressedRistretto(pedersen_commitment.commitment).decompress() {
            Some(p) => p,
            None => return false,
        };
        let h = pedersen_h();

        // Verify each branch: s_i*H == R_i + c_i * (C - vi*G).
        #[allow(clippy::needless_range_loop)]
        for i in 0..n {
            let ri = match CompressedRistretto(self.commitments[i]).decompress() {
                Some(p) => p,
                None => return false,
            };

            let di = c_point - Scalar::from(valid_values[i]) * RISTRETTO_BASEPOINT_POINT;
            let si = Scalar::from_bytes_mod_order(self.responses[i]);
            let ci = Scalar::from_bytes_mod_order(self.challenges[i]);

            if si * h != ri + ci * di {
                return false;
            }
        }

        // Verify challenge sum.
        let mut hash_input = Vec::new();
        for c in &self.commitments {
            hash_input.extend_from_slice(c);
        }
        hash_input.extend_from_slice(&pedersen_commitment.commitment);
        for v in valid_values {
            hash_input.extend_from_slice(&v.to_le_bytes());
        }
        let expected = challenge_scalar(&hash_input);

        let actual: Scalar = self
            .challenges
            .iter()
            .map(|c| Scalar::from_bytes_mod_order(*c))
            .sum();

        expected == actual
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

    // ── Equality proof tests ──

    #[test]
    fn equality_proof_same_value() {
        let r1 = Scalar::random(&mut OsRng);
        let r2 = Scalar::random(&mut OsRng);
        let value = 42u64;

        let c1 = PedersenCommitment::commit_with(value, &r1.to_bytes());
        let c2 = PedersenCommitment::commit_with(value, &r2.to_bytes());

        let proof = EqualityProof::prove(value, &r1.to_bytes(), &r2.to_bytes());
        assert!(proof.verify(&c1, &c2));
    }

    #[test]
    fn equality_proof_rejects_different_values() {
        let r1 = Scalar::random(&mut OsRng);
        let r2 = Scalar::random(&mut OsRng);

        let c1 = PedersenCommitment::commit_with(42, &r1.to_bytes());
        let c2 = PedersenCommitment::commit_with(43, &r2.to_bytes());

        // Prove with same value but commitments differ.
        let proof = EqualityProof::prove(42, &r1.to_bytes(), &r2.to_bytes());
        assert!(!proof.verify(&c1, &c2));
    }

    #[test]
    fn equality_proof_zero_value() {
        let r1 = Scalar::random(&mut OsRng);
        let r2 = Scalar::random(&mut OsRng);

        let c1 = PedersenCommitment::commit_with(0, &r1.to_bytes());
        let c2 = PedersenCommitment::commit_with(0, &r2.to_bytes());

        let proof = EqualityProof::prove(0, &r1.to_bytes(), &r2.to_bytes());
        assert!(proof.verify(&c1, &c2));
    }

    #[test]
    fn equality_proof_serializes() {
        let r1 = Scalar::random(&mut OsRng);
        let r2 = Scalar::random(&mut OsRng);

        let c1 = PedersenCommitment::commit_with(100, &r1.to_bytes());
        let c2 = PedersenCommitment::commit_with(100, &r2.to_bytes());

        let proof = EqualityProof::prove(100, &r1.to_bytes(), &r2.to_bytes());
        let json = serde_json::to_string(&proof).unwrap();
        let restored: EqualityProof = serde_json::from_str(&json).unwrap();
        assert!(restored.verify(&c1, &c2));
    }

    // ── OR proof tests ──

    #[test]
    fn or_proof_two_keys() {
        let (s0, p0) = schnorr_keygen();
        let (_, p1) = schnorr_keygen();
        let keys = [p0, p1];

        let proof = OrProof::prove(&s0, 0, &keys, b"vote").unwrap();
        assert!(proof.verify(&keys, b"vote"));
    }

    #[test]
    fn or_proof_second_key() {
        let (_, p0) = schnorr_keygen();
        let (s1, p1) = schnorr_keygen();
        let keys = [p0, p1];

        let proof = OrProof::prove(&s1, 1, &keys, b"vote").unwrap();
        assert!(proof.verify(&keys, b"vote"));
    }

    #[test]
    fn or_proof_three_keys() {
        let (_, p0) = schnorr_keygen();
        let (_, p1) = schnorr_keygen();
        let (s2, p2) = schnorr_keygen();
        let keys = [p0, p1, p2];

        let proof = OrProof::prove(&s2, 2, &keys, b"membership").unwrap();
        assert!(proof.verify(&keys, b"membership"));
    }

    #[test]
    fn or_proof_rejects_wrong_message() {
        let (s0, p0) = schnorr_keygen();
        let (_, p1) = schnorr_keygen();
        let keys = [p0, p1];

        let proof = OrProof::prove(&s0, 0, &keys, b"original").unwrap();
        assert!(!proof.verify(&keys, b"tampered"));
    }

    #[test]
    fn or_proof_rejects_wrong_keys() {
        let (s0, p0) = schnorr_keygen();
        let (_, p1) = schnorr_keygen();
        let keys = [p0, p1];

        let proof = OrProof::prove(&s0, 0, &keys, b"test").unwrap();

        // Verify against different keys.
        let (_, wrong0) = schnorr_keygen();
        let (_, wrong1) = schnorr_keygen();
        assert!(!proof.verify(&[wrong0, wrong1], b"test"));
    }

    #[test]
    fn or_proof_empty_keys_rejected() {
        let (s, _) = schnorr_keygen();
        let result = OrProof::prove(&s, 0, &[], b"test");
        assert!(result.is_err());
    }

    #[test]
    fn or_proof_out_of_bounds_rejected() {
        let (s, p) = schnorr_keygen();
        let result = OrProof::prove(&s, 1, &[p], b"test");
        assert!(result.is_err());
    }

    #[test]
    fn or_proof_serializes() {
        let (s0, p0) = schnorr_keygen();
        let (_, p1) = schnorr_keygen();
        let keys = [p0, p1];

        let proof = OrProof::prove(&s0, 0, &keys, b"serde").unwrap();
        let json = serde_json::to_string(&proof).unwrap();
        let restored: OrProof = serde_json::from_str(&json).unwrap();
        assert!(restored.verify(&keys, b"serde"));
    }

    #[test]
    fn or_proof_single_key() {
        let (s, p) = schnorr_keygen();
        let proof = OrProof::prove(&s, 0, &[p], b"single").unwrap();
        assert!(proof.verify(&[p], b"single"));
    }

    // ── Set membership proof tests ──

    #[test]
    fn set_membership_proves_inclusion() {
        let value = 42u64;
        let valid = [10, 20, 42, 100, 200];
        let r = Scalar::random(&mut OsRng);
        let c = PedersenCommitment::commit_with(value, &r.to_bytes());

        let proof = SetMembershipProof::prove(value, &r.to_bytes(), &valid).unwrap();
        assert!(proof.verify(&c, &valid));
    }

    #[test]
    fn set_membership_rejects_non_member() {
        let value = 99u64;
        let valid = [10, 20, 42, 100, 200];
        let r = Scalar::random(&mut OsRng);

        let result = SetMembershipProof::prove(value, &r.to_bytes(), &valid);
        assert!(result.is_err());
    }

    #[test]
    fn set_membership_first_element() {
        let valid = [10, 20, 30];
        let r = Scalar::random(&mut OsRng);
        let c = PedersenCommitment::commit_with(10, &r.to_bytes());

        let proof = SetMembershipProof::prove(10, &r.to_bytes(), &valid).unwrap();
        assert!(proof.verify(&c, &valid));
    }

    #[test]
    fn set_membership_last_element() {
        let valid = [10, 20, 30];
        let r = Scalar::random(&mut OsRng);
        let c = PedersenCommitment::commit_with(30, &r.to_bytes());

        let proof = SetMembershipProof::prove(30, &r.to_bytes(), &valid).unwrap();
        assert!(proof.verify(&c, &valid));
    }

    #[test]
    fn set_membership_single_element() {
        let valid = [42];
        let r = Scalar::random(&mut OsRng);
        let c = PedersenCommitment::commit_with(42, &r.to_bytes());

        let proof = SetMembershipProof::prove(42, &r.to_bytes(), &valid).unwrap();
        assert!(proof.verify(&c, &valid));
    }

    #[test]
    fn set_membership_rejects_wrong_commitment() {
        let valid = [10, 20, 30];
        let r1 = Scalar::random(&mut OsRng);
        let r2 = Scalar::random(&mut OsRng);

        // Prove for value 20, but verify against commitment to 30.
        let proof = SetMembershipProof::prove(20, &r1.to_bytes(), &valid).unwrap();
        let wrong_c = PedersenCommitment::commit_with(30, &r2.to_bytes());
        assert!(!proof.verify(&wrong_c, &valid));
    }

    #[test]
    fn set_membership_rejects_wrong_set() {
        let valid = [10, 20, 30];
        let wrong_set = [40, 50, 60];
        let r = Scalar::random(&mut OsRng);
        let c = PedersenCommitment::commit_with(20, &r.to_bytes());

        let proof = SetMembershipProof::prove(20, &r.to_bytes(), &valid).unwrap();
        assert!(!proof.verify(&c, &wrong_set));
    }

    #[test]
    fn set_membership_empty_set_rejected() {
        let r = Scalar::random(&mut OsRng);
        let result = SetMembershipProof::prove(42, &r.to_bytes(), &[]);
        assert!(result.is_err());
    }

    #[test]
    fn set_membership_serializes() {
        let valid = [1, 2, 3, 4, 5];
        let r = Scalar::random(&mut OsRng);
        let c = PedersenCommitment::commit_with(3, &r.to_bytes());

        let proof = SetMembershipProof::prove(3, &r.to_bytes(), &valid).unwrap();
        let json = serde_json::to_string(&proof).unwrap();
        let restored: SetMembershipProof = serde_json::from_str(&json).unwrap();
        assert!(restored.verify(&c, &valid));
    }

    #[test]
    fn set_membership_zero_value() {
        let valid = [0, 1, 2];
        let r = Scalar::random(&mut OsRng);
        let c = PedersenCommitment::commit_with(0, &r.to_bytes());

        let proof = SetMembershipProof::prove(0, &r.to_bytes(), &valid).unwrap();
        assert!(proof.verify(&c, &valid));
    }
}
