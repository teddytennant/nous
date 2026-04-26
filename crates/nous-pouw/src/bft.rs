//! Tendermint-lite BFT finality.
//!
//! Each block is finalized when ⅔ of the *active validator stake* has signed
//! a [`Vote`] for that block's hash at that height. The collected votes form
//! a [`VoteCertificate`], which the next block carries in its
//! [`BlockHeader::parent_qc`](crate::block::BlockHeader::parent_qc) field.
//!
//! v0 simplification: single-round BFT (no separate prevote/precommit phases,
//! no view changes, no leader rotation on timeout). The simulator drives all
//! phases synchronously. v1 will lift this to async with timeouts.

use std::collections::{BTreeMap, BTreeSet};

use ed25519_dalek::{Signer as DalekSigner, SigningKey};
use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::block::{BlockHash, BlockHeight};
use crate::state::{ChainState, WorkerId};

/// One validator's signed approval of a proposed block.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Vote {
    pub height: BlockHeight,
    pub block_hash: BlockHash,
    pub validator: WorkerId,
    pub signature: Vec<u8>,
}

impl Vote {
    /// Sign a fresh vote.
    pub fn new_signed(height: BlockHeight, block_hash: BlockHash, sk: &SigningKey) -> Self {
        let validator = WorkerId::from_verifying_key(&sk.verifying_key());
        let mut v = Vote {
            height,
            block_hash,
            validator,
            signature: vec![],
        };
        let sig = sk.sign(&v.signing_bytes());
        v.signature = sig.to_bytes().to_vec();
        v
    }

    /// Canonical bytes that the validator signs.
    pub fn signing_bytes(&self) -> Vec<u8> {
        let unsigned = Vote {
            signature: vec![],
            ..self.clone()
        };
        serde_json::to_vec(&unsigned).expect("Vote is JSON-serializable")
    }

    pub fn verify(&self) -> Result<(), BftError> {
        let vk = ed25519_dalek::VerifyingKey::from_bytes(&self.validator.0)
            .map_err(|_| BftError::InvalidKey)?;
        let sig_bytes: [u8; 64] = self
            .signature
            .as_slice()
            .try_into()
            .map_err(|_| BftError::InvalidSignature)?;
        let sig = ed25519_dalek::Signature::from_bytes(&sig_bytes);
        ed25519_dalek::Verifier::verify(&vk, &self.signing_bytes(), &sig)
            .map_err(|_| BftError::InvalidSignature)
    }
}

/// A finality justification: ⅔ stake-weighted votes for one block.
///
/// The certificate is small enough to embed in the *next* block's header
/// (`parent_qc`). Anyone can verify it offline against the validator set.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct VoteCertificate {
    pub height: BlockHeight,
    pub block_hash: BlockHash,
    pub votes: Vec<Vote>,
    /// Sum of stake-weighted votes / total active stake, * 1e6 (canonical).
    pub stake_micro: u32,
}

#[derive(Debug, Error, PartialEq)]
pub enum BftError {
    #[error("invalid signature")]
    InvalidSignature,
    #[error("invalid validator key")]
    InvalidKey,
    #[error("vote for wrong height: cert {cert}, vote {vote}")]
    HeightMismatch {
        cert: BlockHeight,
        vote: BlockHeight,
    },
    #[error("vote for wrong block hash")]
    HashMismatch,
    #[error("voter is not in the active validator set")]
    NotValidator,
    #[error("duplicate vote from {voter}")]
    DuplicateVote { voter: String },
    #[error("stake threshold not met: {stake_micro}/{required_micro}")]
    BelowThreshold {
        stake_micro: u32,
        required_micro: u32,
    },
    #[error("no active validators")]
    NoValidators,
    #[error("equivocation detected from {voter}")]
    Equivocation { voter: String },
}

/// Aggregate a set of votes into a [`VoteCertificate`] iff they cross the
/// stake-weighted threshold. Default threshold is ⅔ (666_667 micro).
pub fn form_quorum_cert(
    height: BlockHeight,
    block_hash: BlockHash,
    votes: Vec<Vote>,
    state: &ChainState,
    threshold_micro: u32,
) -> Result<VoteCertificate, BftError> {
    if state.validators.is_empty() {
        return Err(BftError::NoValidators);
    }

    // Verify each vote's signature, height, hash; reject duplicates.
    let mut seen: BTreeSet<WorkerId> = BTreeSet::new();
    let mut accepted: Vec<Vote> = Vec::new();
    for v in &votes {
        if v.height != height {
            return Err(BftError::HeightMismatch {
                cert: height,
                vote: v.height,
            });
        }
        if v.block_hash != block_hash {
            return Err(BftError::HashMismatch);
        }
        if !state.validators.contains(&v.validator) {
            return Err(BftError::NotValidator);
        }
        if !seen.insert(v.validator) {
            return Err(BftError::DuplicateVote {
                voter: v.validator.short(),
            });
        }
        v.verify()?;
        accepted.push(v.clone());
    }

    // Compute stake-weighted share.
    let total: u64 = state
        .validators
        .iter()
        .filter_map(|w| state.workers.get(w))
        .filter(|w| !w.slashed)
        .map(|w| w.stake)
        .sum();
    if total == 0 {
        return Err(BftError::NoValidators);
    }
    let voted: u64 = accepted
        .iter()
        .filter_map(|v| state.workers.get(&v.validator))
        .filter(|w| !w.slashed)
        .map(|w| w.stake)
        .sum();
    let stake_micro = ((voted as u128) * 1_000_000 / (total as u128)) as u32;
    if stake_micro < threshold_micro {
        return Err(BftError::BelowThreshold {
            stake_micro,
            required_micro: threshold_micro,
        });
    }

    accepted.sort_by(|a, b| a.validator.cmp(&b.validator));
    Ok(VoteCertificate {
        height,
        block_hash,
        votes: accepted,
        stake_micro,
    })
}

/// Verify a `VoteCertificate` offline against `state`. Used when receiving
/// the next block whose header carries `parent_qc`.
pub fn verify_quorum_cert(
    cert: &VoteCertificate,
    state: &ChainState,
    threshold_micro: u32,
) -> Result<(), BftError> {
    let _ = form_quorum_cert(
        cert.height,
        cert.block_hash,
        cert.votes.clone(),
        state,
        threshold_micro,
    )?;
    Ok(())
}

/// Detect a validator that signed two contradictory votes at the same height.
pub fn detect_double_vote(a: &Vote, b: &Vote) -> Option<WorkerId> {
    if a.height == b.height && a.validator == b.validator && a.block_hash != b.block_hash {
        // Verify both before reporting (so attackers can't burn validators
        // with forged votes).
        if a.verify().is_ok() && b.verify().is_ok() {
            return Some(a.validator);
        }
    }
    None
}

/// Group votes by (height, block_hash) for tally inspection.
pub fn tally_by_block(
    votes: &[Vote],
    state: &ChainState,
) -> BTreeMap<(BlockHeight, BlockHash), u64> {
    let mut out: BTreeMap<(BlockHeight, BlockHash), u64> = BTreeMap::new();
    for v in votes {
        if !state.validators.contains(&v.validator) {
            continue;
        }
        let stake = state
            .workers
            .get(&v.validator)
            .filter(|w| !w.slashed)
            .map(|w| w.stake)
            .unwrap_or(0);
        *out.entry((v.height, v.block_hash)).or_default() += stake;
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use ed25519_dalek::SigningKey;
    use rand::rngs::OsRng;

    fn validator_set(n: usize, stake: u64) -> (ChainState, Vec<SigningKey>) {
        let mut state = ChainState::new();
        let sks: Vec<_> = (0..n).map(|_| SigningKey::generate(&mut OsRng)).collect();
        for sk in &sks {
            let id = WorkerId::from_verifying_key(&sk.verifying_key());
            state.register_worker(id, stake, 1.0);
            state.validators.insert(id);
        }
        (state, sks)
    }

    fn block_hash(seed: u8) -> BlockHash {
        let mut h = [0u8; 32];
        h[0] = seed;
        h
    }

    #[test]
    fn vote_signs_and_verifies() {
        let sk = SigningKey::generate(&mut OsRng);
        let v = Vote::new_signed(1, block_hash(1), &sk);
        v.verify().unwrap();
    }

    #[test]
    fn vote_rejects_tampered_hash() {
        let sk = SigningKey::generate(&mut OsRng);
        let mut v = Vote::new_signed(1, block_hash(1), &sk);
        v.block_hash = block_hash(2);
        assert!(v.verify().is_err());
    }

    #[test]
    fn unanimous_quorum_forms() {
        let (state, sks) = validator_set(4, 100);
        let h = block_hash(7);
        let votes: Vec<_> = sks.iter().map(|sk| Vote::new_signed(5, h, sk)).collect();
        let cert = form_quorum_cert(5, h, votes, &state, 666_667).unwrap();
        assert_eq!(cert.votes.len(), 4);
        assert_eq!(cert.stake_micro, 1_000_000);
    }

    #[test]
    fn three_of_four_meets_two_thirds() {
        let (state, sks) = validator_set(4, 100);
        let h = block_hash(8);
        let votes: Vec<_> = sks
            .iter()
            .take(3)
            .map(|sk| Vote::new_signed(2, h, sk))
            .collect();
        let cert = form_quorum_cert(2, h, votes, &state, 666_667).unwrap();
        assert!(cert.stake_micro >= 666_667);
    }

    #[test]
    fn two_of_four_misses_two_thirds() {
        let (state, sks) = validator_set(4, 100);
        let h = block_hash(9);
        let votes: Vec<_> = sks
            .iter()
            .take(2)
            .map(|sk| Vote::new_signed(2, h, sk))
            .collect();
        let err = form_quorum_cert(2, h, votes, &state, 666_667).unwrap_err();
        assert!(matches!(err, BftError::BelowThreshold { .. }));
    }

    #[test]
    fn duplicate_vote_rejected() {
        let (state, sks) = validator_set(4, 100);
        let h = block_hash(10);
        let v = Vote::new_signed(1, h, &sks[0]);
        let err = form_quorum_cert(1, h, vec![v.clone(), v], &state, 666_667).unwrap_err();
        assert!(matches!(err, BftError::DuplicateVote { .. }));
    }

    #[test]
    fn non_validator_rejected() {
        let (state, _sks) = validator_set(2, 100);
        let outsider = SigningKey::generate(&mut OsRng);
        let h = block_hash(11);
        let votes = vec![Vote::new_signed(1, h, &outsider)];
        let err = form_quorum_cert(1, h, votes, &state, 666_667).unwrap_err();
        assert_eq!(err, BftError::NotValidator);
    }

    #[test]
    fn vote_for_wrong_block_rejected() {
        let (state, sks) = validator_set(2, 100);
        let votes = vec![Vote::new_signed(1, block_hash(1), &sks[0])];
        let err = form_quorum_cert(1, block_hash(2), votes, &state, 666_667).unwrap_err();
        assert_eq!(err, BftError::HashMismatch);
    }

    #[test]
    fn weighted_by_stake_not_count() {
        let mut state = ChainState::new();
        let big = SigningKey::generate(&mut OsRng);
        let big_id = WorkerId::from_verifying_key(&big.verifying_key());
        state.register_worker(big_id, 1_000_000, 1.0);
        state.validators.insert(big_id);
        let smalls: Vec<_> = (0..3).map(|_| SigningKey::generate(&mut OsRng)).collect();
        for sk in &smalls {
            let id = WorkerId::from_verifying_key(&sk.verifying_key());
            state.register_worker(id, 1, 1.0);
            state.validators.insert(id);
        }
        // Only the big validator votes — that alone clears ⅔ on stake.
        let h = block_hash(12);
        let votes = vec![Vote::new_signed(1, h, &big)];
        let cert = form_quorum_cert(1, h, votes, &state, 666_667).unwrap();
        assert!(cert.stake_micro >= 666_667);
    }

    #[test]
    fn double_vote_detected() {
        let sk = SigningKey::generate(&mut OsRng);
        let a = Vote::new_signed(1, block_hash(1), &sk);
        let b = Vote::new_signed(1, block_hash(2), &sk);
        let bad = detect_double_vote(&a, &b);
        assert_eq!(bad, Some(WorkerId::from_verifying_key(&sk.verifying_key())));
    }

    #[test]
    fn double_vote_same_block_not_equivocation() {
        let sk = SigningKey::generate(&mut OsRng);
        let a = Vote::new_signed(1, block_hash(1), &sk);
        let b = a.clone();
        assert_eq!(detect_double_vote(&a, &b), None);
    }

    #[test]
    fn verify_quorum_cert_round_trip() {
        let (state, sks) = validator_set(5, 50);
        let h = block_hash(33);
        let votes: Vec<_> = sks.iter().map(|sk| Vote::new_signed(7, h, sk)).collect();
        let cert = form_quorum_cert(7, h, votes, &state, 666_667).unwrap();
        verify_quorum_cert(&cert, &state, 666_667).unwrap();
    }

    #[test]
    fn tally_returns_per_block_stake() {
        let (state, sks) = validator_set(3, 100);
        let h1 = block_hash(1);
        let h2 = block_hash(2);
        let votes = vec![
            Vote::new_signed(1, h1, &sks[0]),
            Vote::new_signed(1, h1, &sks[1]),
            Vote::new_signed(1, h2, &sks[2]),
        ];
        let tally = tally_by_block(&votes, &state);
        assert_eq!(tally[&(1, h1)], 200);
        assert_eq!(tally[&(1, h2)], 100);
    }

    #[test]
    fn no_validators_errors_cleanly() {
        let state = ChainState::new();
        let sk = SigningKey::generate(&mut OsRng);
        let v = Vote::new_signed(1, block_hash(0), &sk);
        let err = form_quorum_cert(1, block_hash(0), vec![v], &state, 666_667).unwrap_err();
        assert_eq!(err, BftError::NoValidators);
    }
}
