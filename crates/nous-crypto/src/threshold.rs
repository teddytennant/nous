//! Threshold cryptography: Shamir's Secret Sharing and Feldman VSS.
//!
//! Enables (t, n) secret sharing where any `t` of `n` participants can
//! reconstruct a secret, but `t-1` participants learn nothing. Feldman VSS
//! adds verifiable commitments so participants can check their shares
//! without revealing the secret.
//!
//! Built on the Ristretto group for compatibility with our ZKP and Schnorr
//! proof infrastructure.

use curve25519_dalek::constants::RISTRETTO_BASEPOINT_POINT;
use curve25519_dalek::ristretto::RistrettoPoint;
use curve25519_dalek::scalar::Scalar;
use rand::rngs::OsRng;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use zeroize::Zeroize;

use nous_core::{Error, Result};

// ── Types ──────────────────────────────────────────────────────

/// A share of a secret, identified by a participant index.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Share {
    /// 1-indexed participant identifier (never 0).
    pub index: u32,
    /// The share value (a scalar).
    value: [u8; 32],
}

impl Share {
    pub fn new(index: u32, scalar: &Scalar) -> Self {
        Self {
            index,
            value: scalar.to_bytes(),
        }
    }

    pub fn scalar(&self) -> Scalar {
        Scalar::from_bytes_mod_order(self.value)
    }
}

impl Drop for Share {
    fn drop(&mut self) {
        self.value.zeroize();
    }
}

/// Feldman VSS commitments — public commitments to polynomial coefficients.
/// Allows share holders to verify their shares without learning the secret.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VssCommitments {
    /// Compressed Ristretto points: `a_j * G` for each coefficient `a_j`.
    pub points: Vec<[u8; 32]>,
}

impl VssCommitments {
    /// The threshold (minimum shares needed to reconstruct).
    pub fn threshold(&self) -> usize {
        self.points.len()
    }

    /// The commitment to the secret itself (the constant term).
    pub fn secret_commitment(&self) -> [u8; 32] {
        self.points[0]
    }

    /// Verify that a share is consistent with these commitments.
    pub fn verify_share(&self, share: &Share) -> bool {
        let index_scalar = Scalar::from(share.index as u64);
        let share_point = share.scalar() * RISTRETTO_BASEPOINT_POINT;

        // Compute sum of a_j * G * i^j for j = 0..threshold
        let mut expected = RistrettoPoint::default();
        let mut power = Scalar::ONE;

        for commitment_bytes in &self.points {
            let point = match curve25519_dalek::ristretto::CompressedRistretto(*commitment_bytes)
                .decompress()
            {
                Some(p) => p,
                None => return false,
            };
            expected += power * point;
            power *= index_scalar;
        }

        share_point == expected
    }
}

/// Configuration for a threshold scheme.
#[derive(Debug, Clone, Copy)]
pub struct ThresholdConfig {
    /// Minimum shares needed to reconstruct (t).
    pub threshold: u32,
    /// Total number of shares (n).
    pub total: u32,
}

impl ThresholdConfig {
    pub fn new(threshold: u32, total: u32) -> Result<Self> {
        if threshold == 0 {
            return Err(Error::Crypto("threshold must be >= 1".into()));
        }
        if threshold > total {
            return Err(Error::Crypto(
                "threshold must be <= total participants".into(),
            ));
        }
        if total > 255 {
            return Err(Error::Crypto("max 255 participants".into()));
        }
        Ok(Self { threshold, total })
    }
}

// ── Shamir's Secret Sharing ────────────────────────────────────

/// Split a scalar secret into `n` shares with threshold `t`.
///
/// Uses a random polynomial of degree `t-1` where `f(0) = secret`.
/// Any `t` shares can reconstruct; `t-1` reveals nothing.
pub fn split_secret(secret: &Scalar, config: ThresholdConfig) -> Vec<Share> {
    // Random polynomial coefficients: a_0 = secret, a_1..a_{t-1} random
    let mut coefficients = Vec::with_capacity(config.threshold as usize);
    coefficients.push(*secret);
    for _ in 1..config.threshold {
        coefficients.push(Scalar::random(&mut OsRng));
    }

    // Evaluate polynomial at each participant index (1..=n)
    (1..=config.total)
        .map(|i| {
            let x = Scalar::from(i as u64);
            let y = evaluate_polynomial(&coefficients, &x);
            Share::new(i, &y)
        })
        .collect()
}

/// Split a secret and produce Feldman VSS commitments for verification.
pub fn split_secret_vss(secret: &Scalar, config: ThresholdConfig) -> (Vec<Share>, VssCommitments) {
    let mut coefficients = Vec::with_capacity(config.threshold as usize);
    coefficients.push(*secret);
    for _ in 1..config.threshold {
        coefficients.push(Scalar::random(&mut OsRng));
    }

    // Shares: f(i) for i = 1..=n
    let shares: Vec<Share> = (1..=config.total)
        .map(|i| {
            let x = Scalar::from(i as u64);
            let y = evaluate_polynomial(&coefficients, &x);
            Share::new(i, &y)
        })
        .collect();

    // Commitments: a_j * G for each coefficient
    let points: Vec<[u8; 32]> = coefficients
        .iter()
        .map(|a| (a * RISTRETTO_BASEPOINT_POINT).compress().to_bytes())
        .collect();

    (shares, VssCommitments { points })
}

/// Reconstruct the secret from `t` or more shares using Lagrange interpolation.
///
/// Returns an error if fewer than 2 shares are provided or if duplicate
/// indices are found.
pub fn reconstruct_secret(shares: &[Share]) -> Result<Scalar> {
    if shares.len() < 2 {
        return Err(Error::Crypto(
            "need at least 2 shares to reconstruct".into(),
        ));
    }

    // Check for duplicate indices
    let mut seen = std::collections::HashSet::new();
    for share in shares {
        if !seen.insert(share.index) {
            return Err(Error::Crypto(format!(
                "duplicate share index: {}",
                share.index
            )));
        }
    }

    let mut secret = Scalar::ZERO;

    for (i, share_i) in shares.iter().enumerate() {
        let xi = Scalar::from(share_i.index as u64);
        let mut lagrange = Scalar::ONE;

        for (j, share_j) in shares.iter().enumerate() {
            if i == j {
                continue;
            }
            let xj = Scalar::from(share_j.index as u64);
            // lagrange *= xj / (xj - xi)
            let diff = xj - xi;
            lagrange *= xj * diff.invert();
        }

        secret += lagrange * share_i.scalar();
    }

    Ok(secret)
}

/// Reconstruct the secret and verify it matches the VSS commitment.
pub fn reconstruct_and_verify(shares: &[Share], commitments: &VssCommitments) -> Result<Scalar> {
    let secret = reconstruct_secret(shares)?;

    // Verify: secret * G == commitments[0]
    let reconstructed_point = (secret * RISTRETTO_BASEPOINT_POINT).compress().to_bytes();

    if reconstructed_point != commitments.secret_commitment() {
        return Err(Error::Crypto(
            "reconstructed secret does not match VSS commitment".into(),
        ));
    }

    Ok(secret)
}

// ── Distributed Key Generation (simplified Pedersen DKG) ───────

/// Result of a completed DKG ceremony.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DkgResult {
    /// The combined public key (sum of all participants' public commitments).
    pub group_public_key: [u8; 32],
    /// Each participant's share index.
    pub participant_indices: Vec<u32>,
    /// Threshold required for signing.
    pub threshold: u32,
}

/// Run a simplified DKG ceremony for `n` participants with threshold `t`.
///
/// In a real deployment, each participant generates their own polynomial
/// and distributes shares over secure channels. This function simulates
/// the ceremony locally for testing and single-node scenarios.
pub fn dkg_simulate(config: ThresholdConfig) -> Result<(DkgResult, Vec<Share>)> {
    let n = config.total as usize;
    let t = config.threshold;

    // Each participant generates a secret and splits it
    let mut participant_secrets = Vec::with_capacity(n);
    let mut all_vss = Vec::with_capacity(n);

    for _ in 0..n {
        let secret = Scalar::random(&mut OsRng);
        let (shares, vss) = split_secret_vss(&secret, config);
        participant_secrets.push(shares);
        all_vss.push(vss);
    }

    // Each participant collects shares from all others and sums them
    let mut combined_shares = Vec::with_capacity(n);
    for i in 0..n {
        let mut combined = Scalar::ZERO;
        for (p, participant_shares) in participant_secrets.iter().enumerate() {
            // Verify each incoming share against its sender's VSS commitments
            let share = &participant_shares[i];
            let vss = &all_vss[p];
            if !vss.verify_share(share) {
                return Err(Error::Crypto(format!(
                    "VSS verification failed for share from participant {}",
                    p + 1
                )));
            }
            combined += share.scalar();
        }
        combined_shares.push(Share::new((i + 1) as u32, &combined));
    }

    // Group public key = sum of all participants' constant-term commitments
    let mut group_public = RistrettoPoint::default();
    for vss in &all_vss {
        let point = curve25519_dalek::ristretto::CompressedRistretto(vss.secret_commitment())
            .decompress()
            .ok_or_else(|| Error::Crypto("invalid VSS commitment point".into()))?;
        group_public += point;
    }

    let result = DkgResult {
        group_public_key: group_public.compress().to_bytes(),
        participant_indices: (1..=config.total).collect(),
        threshold: t,
    };

    Ok((result, combined_shares))
}

// ── Threshold Signature (Schnorr-based) ────────────────────────

/// A partial signature from one participant in a threshold signing session.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PartialSignature {
    pub index: u32,
    pub nonce_commitment: [u8; 32],
    pub response: [u8; 32],
}

/// A combined threshold signature.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ThresholdSignature {
    pub nonce_commitment: [u8; 32],
    pub response: [u8; 32],
}

/// Generate a signing nonce for a threshold signing session.
/// Returns `(nonce_secret, nonce_public_bytes)`.
pub fn generate_signing_nonce() -> ([u8; 32], [u8; 32]) {
    let k = Scalar::random(&mut OsRng);
    let r = (k * RISTRETTO_BASEPOINT_POINT).compress().to_bytes();
    (k.to_bytes(), r)
}

/// Create a partial signature using a share and nonce.
pub fn partial_sign(
    share: &Share,
    nonce_secret: &[u8; 32],
    combined_nonce: &[u8; 32],
    group_public_key: &[u8; 32],
    message: &[u8],
    participant_indices: &[u32],
) -> PartialSignature {
    let k = Scalar::from_bytes_mod_order(*nonce_secret);
    let xi = share.scalar();

    // Challenge: H(R || group_pk || message)
    let c = threshold_challenge(combined_nonce, group_public_key, message);

    // Lagrange coefficient for this participant
    let lambda = lagrange_coefficient(share.index, participant_indices);

    // s_i = k_i + c * lambda_i * x_i
    let s = k + c * lambda * xi;

    PartialSignature {
        index: share.index,
        nonce_commitment: (k * RISTRETTO_BASEPOINT_POINT).compress().to_bytes(),
        response: s.to_bytes(),
    }
}

/// Combine partial signatures into a threshold signature.
pub fn combine_partial_signatures(
    partials: &[PartialSignature],
    combined_nonce: &[u8; 32],
) -> ThresholdSignature {
    let mut s = Scalar::ZERO;
    for partial in partials {
        s += Scalar::from_bytes_mod_order(partial.response);
    }
    ThresholdSignature {
        nonce_commitment: *combined_nonce,
        response: s.to_bytes(),
    }
}

/// Verify a threshold signature against the group public key.
pub fn verify_threshold_signature(
    sig: &ThresholdSignature,
    group_public_key: &[u8; 32],
    message: &[u8],
) -> bool {
    let r =
        match curve25519_dalek::ristretto::CompressedRistretto(sig.nonce_commitment).decompress() {
            Some(p) => p,
            None => return false,
        };

    let group_pk =
        match curve25519_dalek::ristretto::CompressedRistretto(*group_public_key).decompress() {
            Some(p) => p,
            None => return false,
        };

    let s = Scalar::from_bytes_mod_order(sig.response);
    let c = threshold_challenge(&sig.nonce_commitment, group_public_key, message);

    // Verify: s * G == R + c * group_pk
    s * RISTRETTO_BASEPOINT_POINT == r + c * group_pk
}

// ── Helpers ────────────────────────────────────────────────────

fn evaluate_polynomial(coefficients: &[Scalar], x: &Scalar) -> Scalar {
    // Horner's method
    let mut result = Scalar::ZERO;
    for coeff in coefficients.iter().rev() {
        result = result * x + coeff;
    }
    result
}

fn lagrange_coefficient(index: u32, indices: &[u32]) -> Scalar {
    let xi = Scalar::from(index as u64);
    let mut lambda = Scalar::ONE;
    for &j in indices {
        if j == index {
            continue;
        }
        let xj = Scalar::from(j as u64);
        lambda *= xj * (xj - xi).invert();
    }
    lambda
}

fn threshold_challenge(nonce: &[u8; 32], group_pk: &[u8; 32], message: &[u8]) -> Scalar {
    let mut hasher = Sha256::new();
    hasher.update(b"nous-threshold-sig-v1");
    hasher.update(nonce);
    hasher.update(group_pk);
    hasher.update(message);
    let hash: [u8; 32] = hasher.finalize().into();
    Scalar::from_bytes_mod_order(hash)
}

#[cfg(test)]
mod tests {
    use super::*;

    // ── ThresholdConfig ────────────────────────────────────────

    #[test]
    fn config_valid() {
        let config = ThresholdConfig::new(2, 3).unwrap();
        assert_eq!(config.threshold, 2);
        assert_eq!(config.total, 3);
    }

    #[test]
    fn config_rejects_zero_threshold() {
        assert!(ThresholdConfig::new(0, 3).is_err());
    }

    #[test]
    fn config_rejects_threshold_exceeding_total() {
        assert!(ThresholdConfig::new(4, 3).is_err());
    }

    #[test]
    fn config_rejects_too_many_participants() {
        assert!(ThresholdConfig::new(2, 256).is_err());
    }

    #[test]
    fn config_allows_threshold_equals_total() {
        assert!(ThresholdConfig::new(3, 3).is_ok());
    }

    #[test]
    fn config_allows_threshold_one() {
        assert!(ThresholdConfig::new(1, 5).is_ok());
    }

    // ── Shamir split/reconstruct ───────────────────────────────

    #[test]
    fn split_and_reconstruct_2_of_3() {
        let secret = Scalar::random(&mut OsRng);
        let config = ThresholdConfig::new(2, 3).unwrap();
        let shares = split_secret(&secret, config);
        assert_eq!(shares.len(), 3);

        // Any 2 shares reconstruct the secret
        let recovered = reconstruct_secret(&shares[0..2]).unwrap();
        assert_eq!(recovered, secret);
    }

    #[test]
    fn split_and_reconstruct_3_of_5() {
        let secret = Scalar::random(&mut OsRng);
        let config = ThresholdConfig::new(3, 5).unwrap();
        let shares = split_secret(&secret, config);

        // Shares 2, 3, 5
        let subset = vec![shares[1].clone(), shares[2].clone(), shares[4].clone()];
        let recovered = reconstruct_secret(&subset).unwrap();
        assert_eq!(recovered, secret);
    }

    #[test]
    fn reconstruct_with_all_shares() {
        let secret = Scalar::random(&mut OsRng);
        let config = ThresholdConfig::new(2, 5).unwrap();
        let shares = split_secret(&secret, config);

        let recovered = reconstruct_secret(&shares).unwrap();
        assert_eq!(recovered, secret);
    }

    #[test]
    fn reconstruct_rejects_single_share() {
        let secret = Scalar::random(&mut OsRng);
        let config = ThresholdConfig::new(2, 3).unwrap();
        let shares = split_secret(&secret, config);

        assert!(reconstruct_secret(&shares[0..1]).is_err());
    }

    #[test]
    fn reconstruct_rejects_duplicate_indices() {
        let secret = Scalar::random(&mut OsRng);
        let config = ThresholdConfig::new(2, 3).unwrap();
        let shares = split_secret(&secret, config);

        let duped = vec![shares[0].clone(), shares[0].clone()];
        assert!(reconstruct_secret(&duped).is_err());
    }

    #[test]
    fn shares_have_correct_indices() {
        let secret = Scalar::random(&mut OsRng);
        let config = ThresholdConfig::new(2, 4).unwrap();
        let shares = split_secret(&secret, config);

        for (i, share) in shares.iter().enumerate() {
            assert_eq!(share.index, (i + 1) as u32);
        }
    }

    #[test]
    fn different_secrets_produce_different_shares() {
        let s1 = Scalar::random(&mut OsRng);
        let s2 = Scalar::random(&mut OsRng);
        let config = ThresholdConfig::new(2, 3).unwrap();

        let shares1 = split_secret(&s1, config);
        let shares2 = split_secret(&s2, config);

        // At least one share should differ (overwhelmingly likely)
        let any_different = shares1
            .iter()
            .zip(shares2.iter())
            .any(|(a, b)| a.scalar() != b.scalar());
        assert!(any_different);
    }

    // ── Feldman VSS ────────────────────────────────────────────

    #[test]
    fn vss_shares_verify() {
        let secret = Scalar::random(&mut OsRng);
        let config = ThresholdConfig::new(2, 3).unwrap();
        let (shares, commitments) = split_secret_vss(&secret, config);

        for share in &shares {
            assert!(commitments.verify_share(share));
        }
    }

    #[test]
    fn vss_rejects_tampered_share() {
        let secret = Scalar::random(&mut OsRng);
        let config = ThresholdConfig::new(2, 3).unwrap();
        let (shares, commitments) = split_secret_vss(&secret, config);

        // Tamper with a share
        let tampered = Share::new(shares[0].index, &Scalar::random(&mut OsRng));
        assert!(!commitments.verify_share(&tampered));
    }

    #[test]
    fn vss_rejects_wrong_index() {
        let secret = Scalar::random(&mut OsRng);
        let config = ThresholdConfig::new(2, 3).unwrap();
        let (shares, commitments) = split_secret_vss(&secret, config);

        // Use share 1's value with index 2
        let wrong_index = Share::new(2, &shares[0].scalar());
        assert!(!commitments.verify_share(&wrong_index));
    }

    #[test]
    fn vss_threshold_matches_config() {
        let secret = Scalar::random(&mut OsRng);
        let config = ThresholdConfig::new(3, 5).unwrap();
        let (_, commitments) = split_secret_vss(&secret, config);
        assert_eq!(commitments.threshold(), 3);
    }

    #[test]
    fn vss_reconstruct_and_verify() {
        let secret = Scalar::random(&mut OsRng);
        let config = ThresholdConfig::new(2, 3).unwrap();
        let (shares, commitments) = split_secret_vss(&secret, config);

        let recovered = reconstruct_and_verify(&shares[0..2], &commitments).unwrap();
        assert_eq!(recovered, secret);
    }

    #[test]
    fn vss_reconstruct_rejects_bad_commitment() {
        let secret = Scalar::random(&mut OsRng);
        let config = ThresholdConfig::new(2, 3).unwrap();
        let (shares, _) = split_secret_vss(&secret, config);

        // Commitments from a different secret
        let other = Scalar::random(&mut OsRng);
        let (_, wrong_commitments) = split_secret_vss(&other, config);

        assert!(reconstruct_and_verify(&shares[0..2], &wrong_commitments).is_err());
    }

    #[test]
    fn vss_secret_commitment_matches() {
        let secret = Scalar::random(&mut OsRng);
        let expected = (secret * RISTRETTO_BASEPOINT_POINT).compress().to_bytes();
        let config = ThresholdConfig::new(2, 3).unwrap();
        let (_, commitments) = split_secret_vss(&secret, config);

        assert_eq!(commitments.secret_commitment(), expected);
    }

    // ── DKG ────────────────────────────────────────────────────

    #[test]
    fn dkg_produces_valid_shares() {
        let config = ThresholdConfig::new(2, 3).unwrap();
        let (result, shares) = dkg_simulate(config).unwrap();

        assert_eq!(shares.len(), 3);
        assert_eq!(result.participant_indices.len(), 3);
        assert_eq!(result.threshold, 2);
    }

    #[test]
    fn dkg_shares_reconstruct_to_consistent_secret() {
        let config = ThresholdConfig::new(2, 3).unwrap();
        let (_, shares) = dkg_simulate(config).unwrap();

        // Any 2 shares should reconstruct the same secret
        let r1 = reconstruct_secret(&[shares[0].clone(), shares[1].clone()]).unwrap();
        let r2 = reconstruct_secret(&[shares[1].clone(), shares[2].clone()]).unwrap();
        let r3 = reconstruct_secret(&[shares[0].clone(), shares[2].clone()]).unwrap();

        assert_eq!(r1, r2);
        assert_eq!(r2, r3);
    }

    #[test]
    fn dkg_group_key_matches_reconstructed_secret() {
        let config = ThresholdConfig::new(2, 3).unwrap();
        let (result, shares) = dkg_simulate(config).unwrap();

        let secret = reconstruct_secret(&shares[0..2]).unwrap();
        let expected_pk = (secret * RISTRETTO_BASEPOINT_POINT).compress().to_bytes();

        assert_eq!(result.group_public_key, expected_pk);
    }

    // ── Threshold Signatures ───────────────────────────────────

    #[test]
    fn threshold_sign_and_verify() {
        let config = ThresholdConfig::new(2, 3).unwrap();
        let (result, shares) = dkg_simulate(config).unwrap();
        let message = b"threshold signed message";

        // Signing session with participants 1 and 2
        let signers = &[shares[0].clone(), shares[1].clone()];
        let indices: Vec<u32> = signers.iter().map(|s| s.index).collect();

        // Each signer generates a nonce
        let (nonce1_secret, nonce1_pub) = generate_signing_nonce();
        let (nonce2_secret, nonce2_pub) = generate_signing_nonce();

        // Combine nonces: R = R1 + R2
        let r1 = curve25519_dalek::ristretto::CompressedRistretto(nonce1_pub)
            .decompress()
            .unwrap();
        let r2 = curve25519_dalek::ristretto::CompressedRistretto(nonce2_pub)
            .decompress()
            .unwrap();
        let combined_nonce = (r1 + r2).compress().to_bytes();

        // Each signer produces a partial signature
        let partial1 = partial_sign(
            &signers[0],
            &nonce1_secret,
            &combined_nonce,
            &result.group_public_key,
            message,
            &indices,
        );
        let partial2 = partial_sign(
            &signers[1],
            &nonce2_secret,
            &combined_nonce,
            &result.group_public_key,
            message,
            &indices,
        );

        // Combine partial signatures
        let sig = combine_partial_signatures(&[partial1, partial2], &combined_nonce);

        // Verify against group public key
        assert!(verify_threshold_signature(
            &sig,
            &result.group_public_key,
            message
        ));
    }

    #[test]
    fn threshold_signature_rejects_wrong_message() {
        let config = ThresholdConfig::new(2, 3).unwrap();
        let (result, shares) = dkg_simulate(config).unwrap();

        let signers = &[shares[0].clone(), shares[1].clone()];
        let indices: Vec<u32> = signers.iter().map(|s| s.index).collect();

        let (n1s, n1p) = generate_signing_nonce();
        let (n2s, n2p) = generate_signing_nonce();

        let r1 = curve25519_dalek::ristretto::CompressedRistretto(n1p)
            .decompress()
            .unwrap();
        let r2 = curve25519_dalek::ristretto::CompressedRistretto(n2p)
            .decompress()
            .unwrap();
        let combined_nonce = (r1 + r2).compress().to_bytes();

        let p1 = partial_sign(
            &signers[0],
            &n1s,
            &combined_nonce,
            &result.group_public_key,
            b"correct message",
            &indices,
        );
        let p2 = partial_sign(
            &signers[1],
            &n2s,
            &combined_nonce,
            &result.group_public_key,
            b"correct message",
            &indices,
        );

        let sig = combine_partial_signatures(&[p1, p2], &combined_nonce);

        assert!(!verify_threshold_signature(
            &sig,
            &result.group_public_key,
            b"wrong message"
        ));
    }

    #[test]
    fn threshold_signature_rejects_wrong_key() {
        let config = ThresholdConfig::new(2, 3).unwrap();
        let (result, shares) = dkg_simulate(config).unwrap();
        let message = b"test message";

        let signers = &[shares[0].clone(), shares[1].clone()];
        let indices: Vec<u32> = signers.iter().map(|s| s.index).collect();

        let (n1s, n1p) = generate_signing_nonce();
        let (n2s, n2p) = generate_signing_nonce();

        let r1 = curve25519_dalek::ristretto::CompressedRistretto(n1p)
            .decompress()
            .unwrap();
        let r2 = curve25519_dalek::ristretto::CompressedRistretto(n2p)
            .decompress()
            .unwrap();
        let combined_nonce = (r1 + r2).compress().to_bytes();

        let p1 = partial_sign(
            &signers[0],
            &n1s,
            &combined_nonce,
            &result.group_public_key,
            message,
            &indices,
        );
        let p2 = partial_sign(
            &signers[1],
            &n2s,
            &combined_nonce,
            &result.group_public_key,
            message,
            &indices,
        );

        let sig = combine_partial_signatures(&[p1, p2], &combined_nonce);

        // Different group key
        let (other_result, _) = dkg_simulate(config).unwrap();
        assert!(!verify_threshold_signature(
            &sig,
            &other_result.group_public_key,
            message
        ));
    }

    #[test]
    fn threshold_sign_with_different_signer_subsets() {
        let config = ThresholdConfig::new(2, 4).unwrap();
        let (result, shares) = dkg_simulate(config).unwrap();
        let message = b"any subset works";

        // Sign with shares 2 and 4
        let signers = &[shares[1].clone(), shares[3].clone()];
        let indices: Vec<u32> = signers.iter().map(|s| s.index).collect();

        let (n1s, n1p) = generate_signing_nonce();
        let (n2s, n2p) = generate_signing_nonce();

        let r1 = curve25519_dalek::ristretto::CompressedRistretto(n1p)
            .decompress()
            .unwrap();
        let r2 = curve25519_dalek::ristretto::CompressedRistretto(n2p)
            .decompress()
            .unwrap();
        let combined_nonce = (r1 + r2).compress().to_bytes();

        let p1 = partial_sign(
            &signers[0],
            &n1s,
            &combined_nonce,
            &result.group_public_key,
            message,
            &indices,
        );
        let p2 = partial_sign(
            &signers[1],
            &n2s,
            &combined_nonce,
            &result.group_public_key,
            message,
            &indices,
        );

        let sig = combine_partial_signatures(&[p1, p2], &combined_nonce);
        assert!(verify_threshold_signature(
            &sig,
            &result.group_public_key,
            message
        ));
    }

    // ── Helpers ────────────────────────────────────────────────

    #[test]
    fn polynomial_evaluation_constant() {
        // f(x) = 42
        let coeffs = vec![Scalar::from(42u64)];
        let result = evaluate_polynomial(&coeffs, &Scalar::from(99u64));
        assert_eq!(result, Scalar::from(42u64));
    }

    #[test]
    fn polynomial_evaluation_linear() {
        // f(x) = 3 + 5x → f(2) = 13
        let coeffs = vec![Scalar::from(3u64), Scalar::from(5u64)];
        let result = evaluate_polynomial(&coeffs, &Scalar::from(2u64));
        assert_eq!(result, Scalar::from(13u64));
    }

    #[test]
    fn polynomial_evaluation_quadratic() {
        // f(x) = 1 + 2x + 3x^2 → f(3) = 1 + 6 + 27 = 34
        let coeffs = vec![Scalar::from(1u64), Scalar::from(2u64), Scalar::from(3u64)];
        let result = evaluate_polynomial(&coeffs, &Scalar::from(3u64));
        assert_eq!(result, Scalar::from(34u64));
    }

    #[test]
    fn lagrange_coefficient_two_parties() {
        // For indices {1, 2}, lagrange(1) = 2/(2-1) = 2, lagrange(2) = 1/(1-2) = -1
        let indices = vec![1, 2];
        let l1 = lagrange_coefficient(1, &indices);
        let l2 = lagrange_coefficient(2, &indices);

        assert_eq!(l1, Scalar::from(2u64));
        // -1 mod order
        assert_eq!(l2, -Scalar::ONE);
    }
}
