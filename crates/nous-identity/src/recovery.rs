use std::collections::HashMap;

use chrono::{DateTime, Utc};
use rand::RngCore;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use nous_core::{Error, Result};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecoveryShare {
    pub index: u8,
    pub guardian_did: String,
    pub data: Vec<u8>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum RecoveryStatus {
    Pending,
    Approved,
    Denied,
    Expired,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecoveryRequest {
    pub id: String,
    pub requester_did: String,
    pub target_did: String,
    pub status: RecoveryStatus,
    pub approvals: Vec<String>,
    pub denials: Vec<String>,
    pub threshold: u8,
    pub created_at: DateTime<Utc>,
}

impl RecoveryRequest {
    pub fn new(requester_did: &str, target_did: &str, threshold: u8) -> Self {
        Self {
            id: format!("recovery:{}", Uuid::new_v4()),
            requester_did: requester_did.into(),
            target_did: target_did.into(),
            status: RecoveryStatus::Pending,
            approvals: Vec::new(),
            denials: Vec::new(),
            threshold,
            created_at: Utc::now(),
        }
    }

    pub fn approve(&mut self, guardian_did: &str) -> Result<()> {
        if self.status != RecoveryStatus::Pending {
            return Err(Error::InvalidInput("recovery is not pending".into()));
        }
        if self.approvals.contains(&guardian_did.to_string()) {
            return Err(Error::InvalidInput("already approved".into()));
        }

        self.approvals.push(guardian_did.into());

        if self.approvals.len() >= self.threshold as usize {
            self.status = RecoveryStatus::Approved;
        }

        Ok(())
    }

    pub fn deny(&mut self, guardian_did: &str) -> Result<()> {
        if self.status != RecoveryStatus::Pending {
            return Err(Error::InvalidInput("recovery is not pending".into()));
        }

        self.denials.push(guardian_did.into());
        Ok(())
    }

    pub fn is_approved(&self) -> bool {
        self.status == RecoveryStatus::Approved
    }

    pub fn approval_count(&self) -> usize {
        self.approvals.len()
    }

    pub fn remaining_approvals(&self) -> usize {
        (self.threshold as usize).saturating_sub(self.approvals.len())
    }
}

/// Shamir's Secret Sharing over GF(256).
///
/// Splits a secret into `n` shares requiring `threshold` to reconstruct.
pub fn split_secret(secret: &[u8], threshold: u8, num_shares: u8) -> Result<Vec<Vec<u8>>> {
    if threshold == 0 {
        return Err(Error::InvalidInput("threshold must be positive".into()));
    }
    if num_shares < threshold {
        return Err(Error::InvalidInput(
            "num_shares must be >= threshold".into(),
        ));
    }
    // num_shares is u8 so max is already 255

    let mut rng = rand::thread_rng();
    let mut shares: Vec<Vec<u8>> = (0..num_shares).map(|_| Vec::with_capacity(secret.len())).collect();

    for &byte in secret {
        // Generate random polynomial coefficients: a[0] = byte, a[1..threshold] = random
        let mut coeffs = vec![0u8; threshold as usize];
        coeffs[0] = byte;
        for c in coeffs.iter_mut().skip(1) {
            let mut buf = [0u8; 1];
            rng.fill_bytes(&mut buf);
            *c = buf[0];
        }

        // Evaluate polynomial at x = 1..num_shares
        for (i, share) in shares.iter_mut().enumerate() {
            let x = (i + 1) as u8;
            let y = eval_polynomial(&coeffs, x);
            share.push(y);
        }
    }

    Ok(shares)
}

/// Reconstruct a secret from shares using Lagrange interpolation in GF(256).
///
/// `shares` is a list of (index, data) where index is 1-based.
pub fn reconstruct_secret(shares: &[(u8, &[u8])]) -> Result<Vec<u8>> {
    if shares.is_empty() {
        return Err(Error::InvalidInput("no shares provided".into()));
    }

    let len = shares[0].1.len();
    for (_, data) in shares.iter().skip(1) {
        if data.len() != len {
            return Err(Error::InvalidInput("share lengths must match".into()));
        }
    }

    let mut secret = Vec::with_capacity(len);

    for byte_idx in 0..len {
        let mut value = 0u8;

        for (i, &(xi, _)) in shares.iter().enumerate() {
            let yi = shares[i].1[byte_idx];
            let mut basis = yi;

            for (j, &(xj, _)) in shares.iter().enumerate() {
                if i != j {
                    // basis *= xj / (xj - xi) in GF(256)
                    let num = xj;
                    let den = gf256_sub(xj, xi);
                    if den == 0 {
                        return Err(Error::InvalidInput(
                            "duplicate share indices".into(),
                        ));
                    }
                    basis = gf256_mul(basis, gf256_mul(num, gf256_inv(den)));
                }
            }

            value = gf256_add(value, basis);
        }

        secret.push(value);
    }

    Ok(secret)
}

/// GF(256) arithmetic with irreducible polynomial x^8 + x^4 + x^3 + x + 1 (0x11B).
fn gf256_add(a: u8, b: u8) -> u8 {
    a ^ b
}

fn gf256_sub(a: u8, b: u8) -> u8 {
    a ^ b // same as add in GF(256)
}

fn gf256_mul(a: u8, b: u8) -> u8 {
    let mut result: u16 = 0;
    let mut a = a as u16;
    let mut b = b;

    while b > 0 {
        if b & 1 != 0 {
            result ^= a;
        }
        a <<= 1;
        if a & 0x100 != 0 {
            a ^= 0x11B;
        }
        b >>= 1;
    }

    result as u8
}

fn gf256_inv(a: u8) -> u8 {
    if a == 0 {
        return 0;
    }
    // Fermat's little theorem: a^(-1) = a^254 in GF(256)
    let mut result = 1u8;
    let mut base = a;
    let mut exp = 254u32;
    while exp > 0 {
        if exp & 1 != 0 {
            result = gf256_mul(result, base);
        }
        base = gf256_mul(base, base);
        exp >>= 1;
    }
    result
}

fn eval_polynomial(coeffs: &[u8], x: u8) -> u8 {
    let mut result = 0u8;
    let mut x_pow = 1u8;

    for &coeff in coeffs {
        result = gf256_add(result, gf256_mul(coeff, x_pow));
        x_pow = gf256_mul(x_pow, x);
    }

    result
}

#[derive(Debug)]
pub struct RecoveryConfig {
    pub guardians: Vec<String>,
    pub threshold: u8,
    shares: HashMap<String, Vec<u8>>,
}

impl RecoveryConfig {
    pub fn new(guardians: Vec<String>, threshold: u8) -> Result<Self> {
        if guardians.len() < threshold as usize {
            return Err(Error::InvalidInput(
                "need at least threshold guardians".into(),
            ));
        }
        if threshold == 0 {
            return Err(Error::InvalidInput("threshold must be positive".into()));
        }

        Ok(Self {
            guardians,
            threshold,
            shares: HashMap::new(),
        })
    }

    pub fn generate_shares(&mut self, secret: &[u8]) -> Result<Vec<RecoveryShare>> {
        let raw_shares =
            split_secret(secret, self.threshold, self.guardians.len() as u8)?;

        let mut result = Vec::new();
        for (i, (guardian, data)) in self.guardians.iter().zip(raw_shares.into_iter()).enumerate() {
            self.shares.insert(guardian.clone(), data.clone());
            result.push(RecoveryShare {
                index: (i + 1) as u8,
                guardian_did: guardian.clone(),
                data,
                created_at: Utc::now(),
            });
        }

        Ok(result)
    }

    pub fn recover(&self, shares: &[RecoveryShare]) -> Result<Vec<u8>> {
        if shares.len() < self.threshold as usize {
            return Err(Error::InvalidInput(format!(
                "need at least {} shares, got {}",
                self.threshold,
                shares.len()
            )));
        }

        let indexed: Vec<(u8, &[u8])> = shares
            .iter()
            .map(|s| (s.index, s.data.as_slice()))
            .collect();

        reconstruct_secret(&indexed)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn gf256_mul_identity() {
        for a in 0..=255u8 {
            assert_eq!(gf256_mul(a, 1), a);
        }
    }

    #[test]
    fn gf256_mul_zero() {
        for a in 0..=255u8 {
            assert_eq!(gf256_mul(a, 0), 0);
        }
    }

    #[test]
    fn gf256_inverse() {
        for a in 1..=255u8 {
            let inv = gf256_inv(a);
            assert_eq!(gf256_mul(a, inv), 1, "failed for a={a}, inv={inv}");
        }
    }

    #[test]
    fn split_and_reconstruct_2_of_3() {
        let secret = b"test secret 123!";
        let shares = split_secret(secret, 2, 3).unwrap();
        assert_eq!(shares.len(), 3);

        // Any 2 shares should reconstruct
        let indexed: Vec<(u8, &[u8])> = vec![(1, &shares[0]), (3, &shares[2])];
        let recovered = reconstruct_secret(&indexed).unwrap();
        assert_eq!(recovered, secret);
    }

    #[test]
    fn split_and_reconstruct_3_of_5() {
        let secret = b"longer secret data for testing recovery";
        let shares = split_secret(secret, 3, 5).unwrap();
        assert_eq!(shares.len(), 5);

        let indexed: Vec<(u8, &[u8])> = vec![(2, &shares[1]), (4, &shares[3]), (5, &shares[4])];
        let recovered = reconstruct_secret(&indexed).unwrap();
        assert_eq!(recovered, secret);
    }

    #[test]
    fn split_and_reconstruct_all_shares() {
        let secret = b"hello";
        let shares = split_secret(secret, 2, 3).unwrap();

        let indexed: Vec<(u8, &[u8])> =
            vec![(1, &shares[0]), (2, &shares[1]), (3, &shares[2])];
        let recovered = reconstruct_secret(&indexed).unwrap();
        assert_eq!(recovered, secret);
    }

    #[test]
    fn split_1_of_1() {
        let secret = b"trivial";
        let shares = split_secret(secret, 1, 1).unwrap();
        assert_eq!(shares.len(), 1);

        let indexed: Vec<(u8, &[u8])> = vec![(1, &shares[0])];
        let recovered = reconstruct_secret(&indexed).unwrap();
        assert_eq!(recovered, secret);
    }

    #[test]
    fn split_threshold_zero_rejected() {
        assert!(split_secret(b"test", 0, 3).is_err());
    }

    #[test]
    fn split_insufficient_shares_rejected() {
        assert!(split_secret(b"test", 5, 3).is_err());
    }

    #[test]
    fn reconstruct_empty_fails() {
        assert!(reconstruct_secret(&[]).is_err());
    }

    #[test]
    fn reconstruct_mismatched_lengths_fails() {
        let a = vec![1, 2, 3];
        let b = vec![4, 5];
        assert!(reconstruct_secret(&[(1, &a), (2, &b)]).is_err());
    }

    #[test]
    fn reconstruct_duplicate_indices_fails() {
        let share = vec![1, 2, 3];
        assert!(reconstruct_secret(&[(1, &share), (1, &share)]).is_err());
    }

    #[test]
    fn recovery_config_generate_and_recover() {
        let guardians = vec!["alice".into(), "bob".into(), "charlie".into()];
        let mut config = RecoveryConfig::new(guardians, 2).unwrap();

        let secret = b"my_signing_key_32_bytes_long!!!!";
        let shares = config.generate_shares(secret).unwrap();
        assert_eq!(shares.len(), 3);

        // Recover with 2 shares
        let recovered = config.recover(&shares[0..2]).unwrap();
        assert_eq!(recovered, secret);
    }

    #[test]
    fn recovery_config_insufficient_shares_fails() {
        let guardians = vec!["a".into(), "b".into(), "c".into()];
        let mut config = RecoveryConfig::new(guardians, 2).unwrap();

        let shares = config.generate_shares(b"secret").unwrap();
        assert!(config.recover(&shares[0..1]).is_err());
    }

    #[test]
    fn recovery_config_threshold_exceeds_guardians_fails() {
        assert!(RecoveryConfig::new(vec!["a".into(), "b".into()], 3).is_err());
    }

    #[test]
    fn recovery_config_zero_threshold_fails() {
        assert!(RecoveryConfig::new(vec!["a".into()], 0).is_err());
    }

    #[test]
    fn recovery_request_approve_flow() {
        let mut req = RecoveryRequest::new("requester", "target", 2);
        assert_eq!(req.status, RecoveryStatus::Pending);
        assert_eq!(req.remaining_approvals(), 2);

        req.approve("guardian-a").unwrap();
        assert_eq!(req.remaining_approvals(), 1);
        assert!(!req.is_approved());

        req.approve("guardian-b").unwrap();
        assert_eq!(req.remaining_approvals(), 0);
        assert!(req.is_approved());
    }

    #[test]
    fn recovery_request_double_approve_fails() {
        let mut req = RecoveryRequest::new("requester", "target", 2);
        req.approve("guardian-a").unwrap();
        assert!(req.approve("guardian-a").is_err());
    }

    #[test]
    fn recovery_request_deny() {
        let mut req = RecoveryRequest::new("requester", "target", 2);
        req.deny("guardian-a").unwrap();
        assert_eq!(req.denials.len(), 1);
        assert_eq!(req.status, RecoveryStatus::Pending);
    }

    #[test]
    fn recovery_request_approve_after_completion_fails() {
        let mut req = RecoveryRequest::new("requester", "target", 1);
        req.approve("guardian-a").unwrap();
        assert!(req.approve("guardian-b").is_err());
    }

    #[test]
    fn recovery_share_serializes() {
        let share = RecoveryShare {
            index: 1,
            guardian_did: "did:key:z...".into(),
            data: vec![1, 2, 3, 4],
            created_at: Utc::now(),
        };
        let json = serde_json::to_string(&share).unwrap();
        let restored: RecoveryShare = serde_json::from_str(&json).unwrap();
        assert_eq!(restored.index, 1);
    }

    #[test]
    fn recovery_request_serializes() {
        let req = RecoveryRequest::new("requester", "target", 2);
        let json = serde_json::to_string(&req).unwrap();
        let restored: RecoveryRequest = serde_json::from_str(&json).unwrap();
        assert_eq!(restored.threshold, 2);
    }

    #[test]
    fn split_reconstruct_32_byte_key() {
        // Simulate recovering a real ed25519 signing key
        let key_bytes = [42u8; 32];
        let shares = split_secret(&key_bytes, 3, 5).unwrap();

        // Use shares 1, 3, 5
        let indexed: Vec<(u8, &[u8])> = vec![
            (1, &shares[0]),
            (3, &shares[2]),
            (5, &shares[4]),
        ];
        let recovered = reconstruct_secret(&indexed).unwrap();
        assert_eq!(recovered, key_bytes);
    }
}
