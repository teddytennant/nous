use nous_core::{Error, Result};
use nous_crypto::keys::did_to_public_key;
use nous_crypto::signing::{Signature, Verifier};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum VoteChoice {
    For,
    Against,
    Abstain,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Ballot {
    pub proposal_id: String,
    pub voter_did: String,
    pub choice: VoteChoice,
    pub weight: u64,
    pub signature: Signature,
}

impl Ballot {
    pub fn new(
        proposal_id: &str,
        identity: &nous_identity::Identity,
        choice: VoteChoice,
        credits: u64,
    ) -> Result<Self> {
        let weight = QuadraticVoting::credits_to_votes(credits);

        let signable = serde_json::json!({
            "proposal_id": proposal_id,
            "voter_did": identity.did(),
            "choice": choice,
            "weight": weight,
        });
        let payload = serde_json::to_vec(&signable)?;
        let signature = identity.sign(&payload);

        Ok(Self {
            proposal_id: proposal_id.to_string(),
            voter_did: identity.did().to_string(),
            choice,
            weight,
            signature,
        })
    }

    pub fn verify(&self) -> Result<()> {
        let key = did_to_public_key(&self.voter_did)?;
        let signable = serde_json::json!({
            "proposal_id": self.proposal_id,
            "voter_did": self.voter_did,
            "choice": self.choice,
            "weight": self.weight,
        });
        let payload = serde_json::to_vec(&signable)?;
        Verifier::verify(&key, &payload, &self.signature)
    }
}

pub struct QuadraticVoting;

impl QuadraticVoting {
    pub fn credits_to_votes(credits: u64) -> u64 {
        (credits as f64).sqrt().floor() as u64
    }

    pub fn votes_to_credits(votes: u64) -> u64 {
        votes * votes
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VoteResult {
    pub proposal_id: String,
    pub votes_for: u64,
    pub votes_against: u64,
    pub votes_abstain: u64,
    pub total_voters: usize,
    pub passed: bool,
}

pub struct VoteTally {
    proposal_id: String,
    ballots: HashMap<String, Ballot>,
    quorum: f64,
    threshold: f64,
}

impl VoteTally {
    pub fn new(proposal_id: impl Into<String>, quorum: f64, threshold: f64) -> Self {
        Self {
            proposal_id: proposal_id.into(),
            ballots: HashMap::new(),
            quorum,
            threshold,
        }
    }

    pub fn cast(&mut self, ballot: Ballot) -> Result<()> {
        ballot.verify()?;

        if ballot.proposal_id != self.proposal_id {
            return Err(Error::Governance(
                "ballot is for a different proposal".into(),
            ));
        }

        // One vote per DID — last ballot wins
        self.ballots.insert(ballot.voter_did.clone(), ballot);
        Ok(())
    }

    pub fn tally(&self, total_eligible_voters: usize) -> VoteResult {
        let mut votes_for: u64 = 0;
        let mut votes_against: u64 = 0;
        let mut votes_abstain: u64 = 0;

        for ballot in self.ballots.values() {
            match ballot.choice {
                VoteChoice::For => votes_for += ballot.weight,
                VoteChoice::Against => votes_against += ballot.weight,
                VoteChoice::Abstain => votes_abstain += ballot.weight,
            }
        }

        let participation = if total_eligible_voters > 0 {
            self.ballots.len() as f64 / total_eligible_voters as f64
        } else {
            0.0
        };

        let quorum_met = participation >= self.quorum;
        let total_decisive = votes_for + votes_against;
        let approval_rate = if total_decisive > 0 {
            votes_for as f64 / total_decisive as f64
        } else {
            0.0
        };

        let passed = quorum_met && approval_rate >= self.threshold;

        VoteResult {
            proposal_id: self.proposal_id.clone(),
            votes_for,
            votes_against,
            votes_abstain,
            total_voters: self.ballots.len(),
            passed,
        }
    }

    pub fn voter_count(&self) -> usize {
        self.ballots.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use nous_identity::Identity;

    #[test]
    fn quadratic_voting_math() {
        assert_eq!(QuadraticVoting::credits_to_votes(1), 1);
        assert_eq!(QuadraticVoting::credits_to_votes(4), 2);
        assert_eq!(QuadraticVoting::credits_to_votes(9), 3);
        assert_eq!(QuadraticVoting::credits_to_votes(100), 10);
        assert_eq!(QuadraticVoting::credits_to_votes(0), 0);
    }

    #[test]
    fn quadratic_voting_roundtrip() {
        for votes in 0..20 {
            let credits = QuadraticVoting::votes_to_credits(votes);
            let back = QuadraticVoting::credits_to_votes(credits);
            assert_eq!(back, votes);
        }
    }

    #[test]
    fn quadratic_voting_non_perfect_square() {
        // 7 credits -> sqrt(7) = 2.64 -> 2 votes (floor)
        assert_eq!(QuadraticVoting::credits_to_votes(7), 2);
    }

    #[test]
    fn create_and_verify_ballot() {
        let voter = Identity::generate();
        let ballot = Ballot::new("prop-1", &voter, VoteChoice::For, 9).unwrap();

        assert!(ballot.verify().is_ok());
        assert_eq!(ballot.weight, 3); // sqrt(9) = 3
        assert_eq!(ballot.choice, VoteChoice::For);
    }

    #[test]
    fn tampered_ballot_fails() {
        let voter = Identity::generate();
        let mut ballot = Ballot::new("prop-1", &voter, VoteChoice::For, 4).unwrap();
        ballot.weight = 100;
        assert!(ballot.verify().is_err());
    }

    #[test]
    fn tally_simple_majority() {
        let mut tally = VoteTally::new("prop-1", 0.0, 0.5);

        for _ in 0..3 {
            let voter = Identity::generate();
            let ballot = Ballot::new("prop-1", &voter, VoteChoice::For, 1).unwrap();
            tally.cast(ballot).unwrap();
        }

        for _ in 0..2 {
            let voter = Identity::generate();
            let ballot = Ballot::new("prop-1", &voter, VoteChoice::Against, 1).unwrap();
            tally.cast(ballot).unwrap();
        }

        let result = tally.tally(10);
        assert!(result.passed);
        assert_eq!(result.votes_for, 3);
        assert_eq!(result.votes_against, 2);
        assert_eq!(result.total_voters, 5);
    }

    #[test]
    fn tally_fails_without_quorum() {
        let mut tally = VoteTally::new("prop-1", 0.5, 0.5); // 50% quorum

        let voter = Identity::generate();
        let ballot = Ballot::new("prop-1", &voter, VoteChoice::For, 1).unwrap();
        tally.cast(ballot).unwrap();

        let result = tally.tally(10); // only 10% participation
        assert!(!result.passed);
    }

    #[test]
    fn tally_quadratic_weights() {
        let mut tally = VoteTally::new("prop-1", 0.0, 0.5);

        // Whale: 100 credits = 10 votes
        let whale = Identity::generate();
        let ballot = Ballot::new("prop-1", &whale, VoteChoice::Against, 100).unwrap();
        tally.cast(ballot).unwrap();

        // 11 small voters: 1 credit each = 1 vote each = 11 votes
        for _ in 0..11 {
            let voter = Identity::generate();
            let ballot = Ballot::new("prop-1", &voter, VoteChoice::For, 1).unwrap();
            tally.cast(ballot).unwrap();
        }

        let result = tally.tally(12);
        // 11 for vs 10 against — quadratic voting limits whale power
        assert!(result.passed);
    }

    #[test]
    fn one_vote_per_did() {
        let voter = Identity::generate();
        let mut tally = VoteTally::new("prop-1", 0.0, 0.5);

        let ballot1 = Ballot::new("prop-1", &voter, VoteChoice::For, 4).unwrap();
        tally.cast(ballot1).unwrap();

        let ballot2 = Ballot::new("prop-1", &voter, VoteChoice::Against, 4).unwrap();
        tally.cast(ballot2).unwrap();

        assert_eq!(tally.voter_count(), 1);
        let result = tally.tally(1);
        assert_eq!(result.votes_for, 0);
        assert_eq!(result.votes_against, 2);
    }

    #[test]
    fn wrong_proposal_rejected() {
        let mut tally = VoteTally::new("prop-1", 0.0, 0.5);
        let voter = Identity::generate();
        let ballot = Ballot::new("prop-2", &voter, VoteChoice::For, 1).unwrap();
        assert!(tally.cast(ballot).is_err());
    }

    #[test]
    fn abstain_doesnt_affect_outcome() {
        let mut tally = VoteTally::new("prop-1", 0.0, 0.5);

        let v1 = Identity::generate();
        tally
            .cast(Ballot::new("prop-1", &v1, VoteChoice::For, 4).unwrap())
            .unwrap();

        let v2 = Identity::generate();
        tally
            .cast(Ballot::new("prop-1", &v2, VoteChoice::Abstain, 100).unwrap())
            .unwrap();

        let result = tally.tally(2);
        assert!(result.passed); // 2 for, 0 against (abstain excluded from ratio)
        assert_eq!(result.votes_abstain, 10);
    }

    #[test]
    fn ballot_serializes() {
        let voter = Identity::generate();
        let ballot = Ballot::new("prop-1", &voter, VoteChoice::For, 9).unwrap();

        let json = serde_json::to_string(&ballot).unwrap();
        let deserialized: Ballot = serde_json::from_str(&json).unwrap();
        assert!(deserialized.verify().is_ok());
    }
}
