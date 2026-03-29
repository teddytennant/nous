//! Zero-knowledge private voting using Pedersen commitments and range proofs.
//!
//! A voter commits to their vote weight using a Pedersen commitment (hiding + binding),
//! then proves the committed weight is within a valid range [0, max_credits] without
//! revealing it. The tally can verify all proofs and compute homomorphic sums of
//! commitments to confirm the result without learning individual vote weights.

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

fn challenge(data: &[u8]) -> Scalar {
    let hash: [u8; 64] = Sha512::digest(data).into();
    Scalar::from_bytes_mod_order_wide(&hash)
}

/// A committed vote: the voter's weight is hidden inside a Pedersen commitment.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommittedVote {
    /// Pedersen commitment to the vote weight: C = weight·G + blinding·H.
    pub commitment: [u8; 32],
    /// Range proof that weight ∈ [0, 2^n).
    pub range_proof: RangeProof,
    /// The choice (For/Against/Abstain) is public — only the weight is hidden.
    pub choice: super::vote::VoteChoice,
    /// Voter DID (public, for sybil resistance).
    pub voter_did: String,
    /// Proposal ID.
    pub proposal_id: String,
}

/// A ring-signature-based range proof using OR-proofs on bit decomposition.
///
/// Proves that the committed value v satisfies 0 ≤ v < 2^n by decomposing v
/// into bits and proving each bit ∈ {0, 1} via an OR-proof (Schnorr ring signature)
/// on the pair (C_i, C_i - G) for each bit commitment C_i.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RangeProof {
    /// Per-bit commitments: C_i = v_i·G + r_i·H where v_i ∈ {0,1}.
    pub bit_commitments: Vec<[u8; 32]>,
    /// OR-proof e0-values (challenge for the bit=0 branch).
    pub e0_values: Vec<[u8; 32]>,
    /// OR-proof e1-values (challenge for the bit=1 branch).
    pub e1_values: Vec<[u8; 32]>,
    /// OR-proof s0-values (response for the bit=0 branch).
    pub s0_values: Vec<[u8; 32]>,
    /// OR-proof s1-values (response for the bit=1 branch).
    pub s1_values: Vec<[u8; 32]>,
}

/// Opening data for a committed vote (kept secret by the voter).
pub struct VoteOpening {
    pub weight: u64,
    pub blinding: Scalar,
    pub bit_blindings: Vec<Scalar>,
}

/// Number of bits for the range proof (supports weights up to 2^16 - 1 = 65535).
const RANGE_BITS: usize = 16;

/// Create a committed vote with a range proof.
pub fn commit_vote(
    proposal_id: &str,
    voter_did: &str,
    choice: super::vote::VoteChoice,
    weight: u64,
) -> Result<(CommittedVote, VoteOpening)> {
    if weight >= (1u64 << RANGE_BITS) {
        return Err(Error::Governance(format!(
            "weight {weight} exceeds maximum {}",
            (1u64 << RANGE_BITS) - 1
        )));
    }

    let h = pedersen_h();

    // Commit to the total weight.
    let blinding = Scalar::random(&mut OsRng);
    let commitment = Scalar::from(weight) * RISTRETTO_BASEPOINT_POINT + blinding * h;

    // Decompose weight into bits and create per-bit commitments.
    let mut bit_blindings = Vec::with_capacity(RANGE_BITS);
    let mut bit_commitments = Vec::with_capacity(RANGE_BITS);
    let mut e0_values = Vec::with_capacity(RANGE_BITS);
    let mut e1_values = Vec::with_capacity(RANGE_BITS);
    let mut s0_values = Vec::with_capacity(RANGE_BITS);
    let mut s1_values = Vec::with_capacity(RANGE_BITS);

    let mut blinding_sum = Scalar::ZERO;

    for i in 0..RANGE_BITS {
        let bit = ((weight >> i) & 1) as u8;

        // Choose blinding factors so that Σ 2^i·r_i = total blinding.
        let r_i = if i == RANGE_BITS - 1 {
            // r_last = (blinding - blinding_sum) * (2^last)^{-1}
            let power = Scalar::from(1u64 << i);
            let remainder = blinding - blinding_sum;
            remainder * power.invert()
        } else {
            let r = Scalar::random(&mut OsRng);
            blinding_sum += Scalar::from(1u64 << i) * r;
            r
        };

        bit_blindings.push(r_i);

        // C_i = bit·G + r_i·H
        let c_i = Scalar::from(bit as u64) * RISTRETTO_BASEPOINT_POINT + r_i * h;
        let c_i_compressed = c_i.compress().to_bytes();
        bit_commitments.push(c_i_compressed);

        // OR-proof proving bit ∈ {0,1}.
        let (e0, e1, s0, s1) = ring_sign_bit(c_i, r_i, bit, i as u32);
        e0_values.push(e0.to_bytes());
        e1_values.push(e1.to_bytes());
        s0_values.push(s0.to_bytes());
        s1_values.push(s1.to_bytes());
    }

    let proof = RangeProof {
        bit_commitments,
        e0_values,
        e1_values,
        s0_values,
        s1_values,
    };

    let vote = CommittedVote {
        commitment: commitment.compress().to_bytes(),
        range_proof: proof,
        choice,
        voter_did: voter_did.to_string(),
        proposal_id: proposal_id.to_string(),
    };

    let opening = VoteOpening {
        weight,
        blinding,
        bit_blindings,
    };

    Ok((vote, opening))
}

/// Verify a committed vote's range proof.
pub fn verify_committed_vote(vote: &CommittedVote) -> Result<()> {
    let proof = &vote.range_proof;
    let h = pedersen_h();

    if proof.bit_commitments.len() != RANGE_BITS {
        return Err(Error::Governance("wrong number of bit commitments".into()));
    }

    // Verify each bit OR-proof.
    for i in 0..RANGE_BITS {
        let c_i = CompressedRistretto(proof.bit_commitments[i])
            .decompress()
            .ok_or_else(|| Error::Governance("invalid bit commitment point".into()))?;

        let e0 = Scalar::from_bytes_mod_order(proof.e0_values[i]);
        let e1 = Scalar::from_bytes_mod_order(proof.e1_values[i]);
        let s0 = Scalar::from_bytes_mod_order(proof.s0_values[i]);
        let s1 = Scalar::from_bytes_mod_order(proof.s1_values[i]);

        if !ring_verify_bit(c_i, e0, e1, s0, s1, i as u32) {
            return Err(Error::Governance(format!(
                "range proof failed at bit {i}"
            )));
        }
    }

    // Verify that bit commitments sum (weighted by powers of 2) to the total commitment.
    let total = CompressedRistretto(vote.commitment)
        .decompress()
        .ok_or_else(|| Error::Governance("invalid vote commitment".into()))?;

    let mut reconstructed = RistrettoPoint::default();
    for (i, bc) in proof.bit_commitments.iter().enumerate() {
        let point = CompressedRistretto(*bc)
            .decompress()
            .ok_or_else(|| Error::Governance("invalid bit commitment".into()))?;
        reconstructed += Scalar::from(1u64 << i) * point;
    }

    if total != reconstructed {
        return Err(Error::Governance(
            "bit commitments do not sum to total commitment".into(),
        ));
    }

    Ok(())
}

/// Homomorphically add two commitments.
pub fn add_commitments(a: &[u8; 32], b: &[u8; 32]) -> Result<[u8; 32]> {
    let pa = CompressedRistretto(*a)
        .decompress()
        .ok_or_else(|| Error::Governance("invalid commitment a".into()))?;
    let pb = CompressedRistretto(*b)
        .decompress()
        .ok_or_else(|| Error::Governance("invalid commitment b".into()))?;
    Ok((pa + pb).compress().to_bytes())
}

/// Verify that a commitment opens to a given value and blinding.
pub fn verify_opening(commitment: &[u8; 32], weight: u64, blinding: &Scalar) -> bool {
    let expected = Scalar::from(weight) * RISTRETTO_BASEPOINT_POINT + *blinding * pedersen_h();
    expected.compress().to_bytes() == *commitment
}

// ── OR-proof ring signature for bit proofs ────────────────────────
//
// Standard Schnorr OR-proof: proves knowledge of DL of either
// C_i (bit=0: C_i = r_i·H) or C_i - G (bit=1: C_i - G = r_i·H).
//
// Returns (e0, e1, s0, s1) where e0 + e1 = H(C_i, R0, R1, index).

fn ring_hash(
    c_i: &RistrettoPoint,
    r0: &RistrettoPoint,
    r1: &RistrettoPoint,
    bit_index: u32,
) -> Scalar {
    let mut data = Vec::with_capacity(128);
    data.extend_from_slice(&c_i.compress().to_bytes());
    data.extend_from_slice(&r0.compress().to_bytes());
    data.extend_from_slice(&r1.compress().to_bytes());
    data.extend_from_slice(&bit_index.to_le_bytes());
    challenge(&data)
}

/// Sign returns (e0, e1, s0, s1).
fn ring_sign_bit(
    c_i: RistrettoPoint,
    r_i: Scalar,
    bit: u8,
    bit_index: u32,
) -> (Scalar, Scalar, Scalar, Scalar) {
    let h = pedersen_h();
    let g = RISTRETTO_BASEPOINT_POINT;

    if bit == 0 {
        // We know DL of C_i w.r.t. H: C_i = r_i·H.
        // Simulate bit=1 branch.
        let e1 = Scalar::random(&mut OsRng);
        let s1 = Scalar::random(&mut OsRng);
        let r1_sim = s1 * h + e1 * (c_i - g);

        // Real branch.
        let k = Scalar::random(&mut OsRng);
        let r0 = k * h;

        let e_total = ring_hash(&c_i, &r0, &r1_sim, bit_index);
        let e0 = e_total - e1;
        let s0 = k - e0 * r_i;

        (e0, e1, s0, s1)
    } else {
        // We know DL of (C_i - G) w.r.t. H: C_i - G = r_i·H.
        // Simulate bit=0 branch.
        let e0 = Scalar::random(&mut OsRng);
        let s0 = Scalar::random(&mut OsRng);
        let r0_sim = s0 * h + e0 * c_i;

        // Real branch.
        let k = Scalar::random(&mut OsRng);
        let r1 = k * h;

        let e_total = ring_hash(&c_i, &r0_sim, &r1, bit_index);
        let e1 = e_total - e0;
        let s1 = k - e1 * r_i;

        (e0, e1, s0, s1)
    }
}

fn ring_verify_bit(
    c_i: RistrettoPoint,
    e0: Scalar,
    e1: Scalar,
    s0: Scalar,
    s1: Scalar,
    bit_index: u32,
) -> bool {
    let h = pedersen_h();
    let g = RISTRETTO_BASEPOINT_POINT;

    // Reconstruct R0 = s0·H + e0·C_i (bit=0 branch)
    let r0 = s0 * h + e0 * c_i;
    // Reconstruct R1 = s1·H + e1·(C_i - G) (bit=1 branch)
    let r1 = s1 * h + e1 * (c_i - g);

    // Check: e0 + e1 == H(C_i, R0, R1, index)
    let e_total = ring_hash(&c_i, &r0, &r1, bit_index);
    e0 + e1 == e_total
}

/// Private tally result — shows totals but not individual vote weights.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PrivateTallyResult {
    pub proposal_id: String,
    pub total_votes: usize,
    pub votes_for: usize,
    pub votes_against: usize,
    pub votes_abstain: usize,
    pub commitment_sum_for: Option<[u8; 32]>,
    pub commitment_sum_against: Option<[u8; 32]>,
    pub all_proofs_valid: bool,
}

/// Tally committed votes, verifying all range proofs.
pub fn tally_private_votes(
    proposal_id: &str,
    votes: &[CommittedVote],
) -> Result<PrivateTallyResult> {
    use super::vote::VoteChoice;

    let mut for_commitments: Vec<[u8; 32]> = Vec::new();
    let mut against_commitments: Vec<[u8; 32]> = Vec::new();
    let mut votes_for = 0usize;
    let mut votes_against = 0usize;
    let mut votes_abstain = 0usize;

    for vote in votes {
        if vote.proposal_id != proposal_id {
            return Err(Error::Governance(format!(
                "vote for wrong proposal: expected {proposal_id}, got {}",
                vote.proposal_id
            )));
        }

        verify_committed_vote(vote)?;

        match vote.choice {
            VoteChoice::For => {
                for_commitments.push(vote.commitment);
                votes_for += 1;
            }
            VoteChoice::Against => {
                against_commitments.push(vote.commitment);
                votes_against += 1;
            }
            VoteChoice::Abstain => {
                votes_abstain += 1;
            }
        }
    }

    // Homomorphically sum commitments per choice.
    let commitment_sum_for = if for_commitments.len() > 1 {
        let mut sum = for_commitments[0];
        for c in &for_commitments[1..] {
            sum = add_commitments(&sum, c)?;
        }
        Some(sum)
    } else {
        for_commitments.first().copied()
    };

    let commitment_sum_against = if against_commitments.len() > 1 {
        let mut sum = against_commitments[0];
        for c in &against_commitments[1..] {
            sum = add_commitments(&sum, c)?;
        }
        Some(sum)
    } else {
        against_commitments.first().copied()
    };

    Ok(PrivateTallyResult {
        proposal_id: proposal_id.to_string(),
        total_votes: votes.len(),
        votes_for,
        votes_against,
        votes_abstain,
        commitment_sum_for,
        commitment_sum_against,
        all_proofs_valid: true,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::vote::VoteChoice;

    #[test]
    fn commit_and_verify_vote() {
        let (vote, _opening) = commit_vote("prop-1", "did:key:zVoter", VoteChoice::For, 42).unwrap();
        assert!(verify_committed_vote(&vote).is_ok());
    }

    #[test]
    fn verify_opening() {
        let (vote, opening) = commit_vote("prop-1", "did:key:zVoter", VoteChoice::For, 100).unwrap();
        assert!(super::verify_opening(
            &vote.commitment,
            opening.weight,
            &opening.blinding
        ));
    }

    #[test]
    fn verify_opening_wrong_weight() {
        let (vote, opening) = commit_vote("prop-1", "did:key:zVoter", VoteChoice::For, 50).unwrap();
        assert!(!super::verify_opening(
            &vote.commitment,
            51,
            &opening.blinding
        ));
    }

    #[test]
    fn verify_rejects_tampered_commitment() {
        let (mut vote, _opening) =
            commit_vote("prop-1", "did:key:zVoter", VoteChoice::For, 10).unwrap();
        // Tamper with the commitment
        vote.commitment[0] ^= 0xFF;
        assert!(verify_committed_vote(&vote).is_err());
    }

    #[test]
    fn zero_weight_proves_correctly() {
        let (vote, opening) = commit_vote("prop-1", "did:key:zVoter", VoteChoice::Abstain, 0).unwrap();
        assert!(verify_committed_vote(&vote).is_ok());
        assert!(super::verify_opening(
            &vote.commitment,
            0,
            &opening.blinding
        ));
    }

    #[test]
    fn max_weight_proves_correctly() {
        let max = (1u64 << RANGE_BITS) - 1;
        let (vote, opening) = commit_vote("prop-1", "did:key:zVoter", VoteChoice::For, max).unwrap();
        assert!(verify_committed_vote(&vote).is_ok());
        assert!(super::verify_opening(
            &vote.commitment,
            max,
            &opening.blinding
        ));
    }

    #[test]
    fn exceeds_max_weight_rejected() {
        let over_max = 1u64 << RANGE_BITS;
        assert!(commit_vote("prop-1", "did:key:zVoter", VoteChoice::For, over_max).is_err());
    }

    #[test]
    fn different_weights_different_commitments() {
        let (v1, _) = commit_vote("prop-1", "did:key:zVoter", VoteChoice::For, 10).unwrap();
        let (v2, _) = commit_vote("prop-1", "did:key:zVoter", VoteChoice::For, 10).unwrap();
        // Hiding property: same value, different commitments
        assert_ne!(v1.commitment, v2.commitment);
    }

    #[test]
    fn homomorphic_addition() {
        let (v1, o1) = commit_vote("prop-1", "did:key:zA", VoteChoice::For, 30).unwrap();
        let (v2, o2) = commit_vote("prop-1", "did:key:zB", VoteChoice::For, 12).unwrap();
        let sum = add_commitments(&v1.commitment, &v2.commitment).unwrap();
        let combined_blinding = o1.blinding + o2.blinding;
        assert!(super::verify_opening(&sum, 42, &combined_blinding));
    }

    #[test]
    fn tally_private_votes_basic() {
        let mut votes = Vec::new();
        for i in 0..5 {
            let (vote, _) = commit_vote(
                "prop-1",
                &format!("did:key:zVoter{i}"),
                VoteChoice::For,
                10,
            )
            .unwrap();
            votes.push(vote);
        }
        for i in 5..8 {
            let (vote, _) = commit_vote(
                "prop-1",
                &format!("did:key:zVoter{i}"),
                VoteChoice::Against,
                5,
            )
            .unwrap();
            votes.push(vote);
        }

        let result = tally_private_votes("prop-1", &votes).unwrap();
        assert!(result.all_proofs_valid);
        assert_eq!(result.votes_for, 5);
        assert_eq!(result.votes_against, 3);
        assert_eq!(result.total_votes, 8);
        assert!(result.commitment_sum_for.is_some());
        assert!(result.commitment_sum_against.is_some());
    }

    #[test]
    fn tally_verifies_homomorphic_sum() {
        let mut votes = Vec::new();
        let mut openings = Vec::new();
        let weights = [10u64, 20, 30];

        for (i, &w) in weights.iter().enumerate() {
            let (vote, opening) = commit_vote(
                "prop-1",
                &format!("did:key:zVoter{i}"),
                VoteChoice::For,
                w,
            )
            .unwrap();
            votes.push(vote);
            openings.push(opening);
        }

        let result = tally_private_votes("prop-1", &votes).unwrap();
        let sum_commitment = result.commitment_sum_for.unwrap();

        // The sum of blindings should open the sum commitment to 60.
        let total_blinding: Scalar = openings.iter().map(|o| o.blinding).sum();
        assert!(super::verify_opening(&sum_commitment, 60, &total_blinding));
    }

    #[test]
    fn tally_rejects_wrong_proposal() {
        let (vote, _) = commit_vote("prop-2", "did:key:zVoter", VoteChoice::For, 10).unwrap();
        let result = tally_private_votes("prop-1", &[vote]);
        assert!(result.is_err());
    }

    #[test]
    fn tally_with_abstains() {
        let (for_vote, _) =
            commit_vote("prop-1", "did:key:zA", VoteChoice::For, 10).unwrap();
        let (abstain_vote, _) =
            commit_vote("prop-1", "did:key:zB", VoteChoice::Abstain, 5).unwrap();

        let result = tally_private_votes("prop-1", &[for_vote, abstain_vote]).unwrap();
        assert_eq!(result.votes_for, 1);
        assert_eq!(result.votes_abstain, 1);
        assert_eq!(result.total_votes, 2);
    }

    #[test]
    fn vote_serializes() {
        let (vote, _) = commit_vote("prop-1", "did:key:zVoter", VoteChoice::For, 42).unwrap();
        let json = serde_json::to_string(&vote).unwrap();
        let deserialized: CommittedVote = serde_json::from_str(&json).unwrap();
        assert!(verify_committed_vote(&deserialized).is_ok());
    }

    #[test]
    fn single_vote_tally() {
        let (vote, _) = commit_vote("prop-1", "did:key:zVoter", VoteChoice::For, 100).unwrap();
        let result = tally_private_votes("prop-1", &[vote]).unwrap();
        assert_eq!(result.votes_for, 1);
        assert!(result.commitment_sum_for.is_some());
    }

    #[test]
    fn empty_tally() {
        let result = tally_private_votes("prop-1", &[]).unwrap();
        assert_eq!(result.total_votes, 0);
        assert!(result.all_proofs_valid);
    }
}
