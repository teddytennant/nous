use chrono::{DateTime, Duration, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use nous_core::{Error, Result};
use nous_crypto::keys::did_to_public_key;
use nous_crypto::signing::{Signature, Verifier};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ProposalStatus {
    Draft,
    Active,
    Passed,
    Rejected,
    Cancelled,
    Executed,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Proposal {
    pub id: String,
    pub dao_id: String,
    pub title: String,
    pub description: String,
    pub proposer_did: String,
    pub status: ProposalStatus,
    pub created_at: DateTime<Utc>,
    pub voting_starts: DateTime<Utc>,
    pub voting_ends: DateTime<Utc>,
    pub quorum: f64,
    pub threshold: f64,
    pub signature: Signature,
}

impl Proposal {
    pub fn verify(&self) -> Result<()> {
        let key = did_to_public_key(&self.proposer_did)?;
        let payload = self.signable_bytes()?;
        Verifier::verify(&key, &payload, &self.signature)
    }

    fn signable_bytes(&self) -> Result<Vec<u8>> {
        let signable = serde_json::json!({
            "id": self.id,
            "dao_id": self.dao_id,
            "title": self.title,
            "description": self.description,
            "proposer_did": self.proposer_did,
            "voting_starts": self.voting_starts,
            "voting_ends": self.voting_ends,
            "quorum": self.quorum,
            "threshold": self.threshold,
        });
        Ok(serde_json::to_vec(&signable)?)
    }

    pub fn is_voting_active(&self) -> bool {
        let now = Utc::now();
        self.status == ProposalStatus::Active
            && now >= self.voting_starts
            && now <= self.voting_ends
    }

    pub fn is_voting_ended(&self) -> bool {
        Utc::now() > self.voting_ends
    }

    pub fn set_status(&mut self, status: ProposalStatus) {
        self.status = status;
    }
}

pub struct ProposalBuilder {
    dao_id: String,
    title: String,
    description: String,
    voting_duration: Duration,
    delay: Duration,
    quorum: f64,
    threshold: f64,
}

impl ProposalBuilder {
    pub fn new(
        dao_id: impl Into<String>,
        title: impl Into<String>,
        description: impl Into<String>,
    ) -> Self {
        Self {
            dao_id: dao_id.into(),
            title: title.into(),
            description: description.into(),
            voting_duration: Duration::days(7),
            delay: Duration::hours(0),
            quorum: 0.1,
            threshold: 0.5,
        }
    }

    pub fn voting_duration(mut self, duration: Duration) -> Self {
        self.voting_duration = duration;
        self
    }

    pub fn delay(mut self, delay: Duration) -> Self {
        self.delay = delay;
        self
    }

    pub fn quorum(mut self, quorum: f64) -> Self {
        self.quorum = quorum;
        self
    }

    pub fn threshold(mut self, threshold: f64) -> Self {
        self.threshold = threshold;
        self
    }

    pub fn submit(self, proposer: &nous_identity::Identity) -> Result<Proposal> {
        if self.quorum < 0.0 || self.quorum > 1.0 {
            return Err(Error::InvalidInput("quorum must be between 0 and 1".into()));
        }
        if self.threshold < 0.0 || self.threshold > 1.0 {
            return Err(Error::InvalidInput(
                "threshold must be between 0 and 1".into(),
            ));
        }

        let id = format!("prop:{}", Uuid::new_v4());
        let now = Utc::now();
        let voting_starts = now + self.delay;
        let voting_ends = voting_starts + self.voting_duration;

        let signable = serde_json::json!({
            "id": id,
            "dao_id": self.dao_id,
            "title": self.title,
            "description": self.description,
            "proposer_did": proposer.did(),
            "voting_starts": voting_starts,
            "voting_ends": voting_ends,
            "quorum": self.quorum,
            "threshold": self.threshold,
        });
        let payload = serde_json::to_vec(&signable)?;
        let signature = proposer.sign(&payload);

        Ok(Proposal {
            id,
            dao_id: self.dao_id,
            title: self.title,
            description: self.description,
            proposer_did: proposer.did().to_string(),
            status: ProposalStatus::Active,
            created_at: now,
            voting_starts,
            voting_ends,
            quorum: self.quorum,
            threshold: self.threshold,
            signature,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use nous_identity::Identity;

    #[test]
    fn create_and_verify_proposal() {
        let proposer = Identity::generate();
        let proposal = ProposalBuilder::new("dao-1", "Fund education", "Allocate 1000 tokens")
            .submit(&proposer)
            .unwrap();

        assert!(proposal.verify().is_ok());
        assert_eq!(proposal.status, ProposalStatus::Active);
        assert!(proposal.id.starts_with("prop:"));
    }

    #[test]
    fn proposal_custom_parameters() {
        let proposer = Identity::generate();
        let proposal = ProposalBuilder::new("dao-1", "Test", "Test proposal")
            .quorum(0.33)
            .threshold(0.67)
            .voting_duration(Duration::days(3))
            .submit(&proposer)
            .unwrap();

        assert!((proposal.quorum - 0.33).abs() < f64::EPSILON);
        assert!((proposal.threshold - 0.67).abs() < f64::EPSILON);
    }

    #[test]
    fn proposal_invalid_quorum() {
        let proposer = Identity::generate();
        let result = ProposalBuilder::new("dao-1", "Bad", "Bad proposal")
            .quorum(1.5)
            .submit(&proposer);
        assert!(result.is_err());
    }

    #[test]
    fn proposal_invalid_threshold() {
        let proposer = Identity::generate();
        let result = ProposalBuilder::new("dao-1", "Bad", "Bad proposal")
            .threshold(-0.1)
            .submit(&proposer);
        assert!(result.is_err());
    }

    #[test]
    fn proposal_tamper_detection() {
        let proposer = Identity::generate();
        let mut proposal = ProposalBuilder::new("dao-1", "Original", "Original description")
            .submit(&proposer)
            .unwrap();

        proposal.title = "Tampered".to_string();
        assert!(proposal.verify().is_err());
    }

    #[test]
    fn proposal_status_transitions() {
        let proposer = Identity::generate();
        let mut proposal = ProposalBuilder::new("dao-1", "Test", "Test")
            .submit(&proposer)
            .unwrap();

        assert_eq!(proposal.status, ProposalStatus::Active);
        proposal.set_status(ProposalStatus::Passed);
        assert_eq!(proposal.status, ProposalStatus::Passed);
        proposal.set_status(ProposalStatus::Executed);
        assert_eq!(proposal.status, ProposalStatus::Executed);
    }

    #[test]
    fn proposal_serializes() {
        let proposer = Identity::generate();
        let proposal = ProposalBuilder::new("dao-1", "Serde", "Test serialization")
            .submit(&proposer)
            .unwrap();

        let json = serde_json::to_string(&proposal).unwrap();
        let deserialized: Proposal = serde_json::from_str(&json).unwrap();
        assert!(deserialized.verify().is_ok());
    }
}
