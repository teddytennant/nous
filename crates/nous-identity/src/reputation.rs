use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use nous_core::Result;
use nous_crypto::signing::{Signature, Signer, Verifier};

use crate::did::Identity;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ReputationCategory {
    Governance,
    Messaging,
    Trading,
    Moderation,
    Development,
    General,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReputationEvent {
    pub id: String,
    pub subject: String,
    pub issuer: String,
    pub category: ReputationCategory,
    pub delta: i32,
    pub reason: String,
    pub timestamp: DateTime<Utc>,
    pub signature: Signature,
}

impl ReputationEvent {
    pub fn verify(&self) -> Result<()> {
        let issuer_key = nous_crypto::keys::did_to_public_key(&self.issuer)?;
        let payload = self.signable_bytes()?;
        Verifier::verify(&issuer_key, &payload, &self.signature)
    }

    fn signable_bytes(&self) -> Result<Vec<u8>> {
        let signable = serde_json::json!({
            "id": self.id,
            "subject": self.subject,
            "issuer": self.issuer,
            "category": self.category,
            "delta": self.delta,
            "reason": self.reason,
            "timestamp": self.timestamp,
        });
        Ok(serde_json::to_vec(&signable)?)
    }
}

#[derive(Serialize, Deserialize)]
pub struct Reputation {
    pub did: String,
    scores: std::collections::HashMap<ReputationCategory, i64>,
    events: Vec<ReputationEvent>,
}

impl Reputation {
    pub fn new(did: impl Into<String>) -> Self {
        Self {
            did: did.into(),
            scores: std::collections::HashMap::new(),
            events: Vec::new(),
        }
    }

    pub fn score(&self, category: ReputationCategory) -> i64 {
        self.scores.get(&category).copied().unwrap_or(0)
    }

    pub fn total_score(&self) -> i64 {
        self.scores.values().sum()
    }

    pub fn apply(&mut self, event: &ReputationEvent) -> Result<()> {
        event.verify()?;

        if event.subject != self.did {
            return Err(nous_core::Error::Identity(
                "reputation event subject mismatch".into(),
            ));
        }

        *self.scores.entry(event.category).or_insert(0) += event.delta as i64;
        self.events.push(event.clone());
        Ok(())
    }

    pub fn events(&self) -> &[ReputationEvent] {
        &self.events
    }

    pub fn issue_event(
        issuer: &Identity,
        subject_did: &str,
        category: ReputationCategory,
        delta: i32,
        reason: impl Into<String>,
    ) -> Result<ReputationEvent> {
        let id = format!("urn:uuid:{}", Uuid::new_v4());
        let timestamp = Utc::now();
        let reason = reason.into();

        let signable = serde_json::json!({
            "id": id,
            "subject": subject_did,
            "issuer": issuer.did(),
            "category": category,
            "delta": delta,
            "reason": reason,
            "timestamp": timestamp,
        });
        let payload = serde_json::to_vec(&signable)?;
        let signature = Signer::new(issuer.keypair()).sign(&payload);

        Ok(ReputationEvent {
            id,
            subject: subject_did.to_string(),
            issuer: issuer.did().to_string(),
            category,
            delta,
            reason,
            timestamp,
            signature,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn issue_and_verify_reputation_event() {
        let issuer = Identity::generate();
        let subject = Identity::generate();

        let event = Reputation::issue_event(
            &issuer,
            subject.did(),
            ReputationCategory::Governance,
            10,
            "participated in vote",
        )
        .unwrap();

        assert!(event.verify().is_ok());
    }

    #[test]
    fn apply_reputation_event() {
        let issuer = Identity::generate();
        let subject = Identity::generate();

        let mut rep = Reputation::new(subject.did());
        assert_eq!(rep.total_score(), 0);

        let event = Reputation::issue_event(
            &issuer,
            subject.did(),
            ReputationCategory::Governance,
            10,
            "voted",
        )
        .unwrap();

        rep.apply(&event).unwrap();
        assert_eq!(rep.score(ReputationCategory::Governance), 10);
        assert_eq!(rep.total_score(), 10);
    }

    #[test]
    fn multiple_categories() {
        let issuer = Identity::generate();
        let subject = Identity::generate();
        let mut rep = Reputation::new(subject.did());

        let gov_event = Reputation::issue_event(
            &issuer,
            subject.did(),
            ReputationCategory::Governance,
            10,
            "voted",
        )
        .unwrap();

        let trade_event = Reputation::issue_event(
            &issuer,
            subject.did(),
            ReputationCategory::Trading,
            5,
            "completed trade",
        )
        .unwrap();

        rep.apply(&gov_event).unwrap();
        rep.apply(&trade_event).unwrap();

        assert_eq!(rep.score(ReputationCategory::Governance), 10);
        assert_eq!(rep.score(ReputationCategory::Trading), 5);
        assert_eq!(rep.total_score(), 15);
    }

    #[test]
    fn negative_reputation() {
        let issuer = Identity::generate();
        let subject = Identity::generate();
        let mut rep = Reputation::new(subject.did());

        let event = Reputation::issue_event(
            &issuer,
            subject.did(),
            ReputationCategory::Moderation,
            -5,
            "spam detected",
        )
        .unwrap();

        rep.apply(&event).unwrap();
        assert_eq!(rep.score(ReputationCategory::Moderation), -5);
    }

    #[test]
    fn reject_event_for_wrong_subject() {
        let issuer = Identity::generate();
        let subject_a = Identity::generate();
        let subject_b = Identity::generate();
        let mut rep = Reputation::new(subject_a.did());

        let event = Reputation::issue_event(
            &issuer,
            subject_b.did(),
            ReputationCategory::General,
            1,
            "wrong target",
        )
        .unwrap();

        assert!(rep.apply(&event).is_err());
    }

    #[test]
    fn tampered_event_fails_verification() {
        let issuer = Identity::generate();
        let subject = Identity::generate();

        let mut event = Reputation::issue_event(
            &issuer,
            subject.did(),
            ReputationCategory::Trading,
            5,
            "legit trade",
        )
        .unwrap();

        event.delta = 500; // tamper

        let mut rep = Reputation::new(subject.did());
        assert!(rep.apply(&event).is_err());
    }

    #[test]
    fn events_are_tracked() {
        let issuer = Identity::generate();
        let subject = Identity::generate();
        let mut rep = Reputation::new(subject.did());

        for i in 0..3 {
            let event = Reputation::issue_event(
                &issuer,
                subject.did(),
                ReputationCategory::General,
                1,
                format!("action {i}"),
            )
            .unwrap();
            rep.apply(&event).unwrap();
        }

        assert_eq!(rep.events().len(), 3);
        assert_eq!(rep.total_score(), 3);
    }

    #[test]
    fn reputation_event_serializes() {
        let issuer = Identity::generate();
        let subject = Identity::generate();

        let event = Reputation::issue_event(
            &issuer,
            subject.did(),
            ReputationCategory::Development,
            10,
            "merged PR",
        )
        .unwrap();

        let json = serde_json::to_string(&event).unwrap();
        let deserialized: ReputationEvent = serde_json::from_str(&json).unwrap();
        assert!(deserialized.verify().is_ok());
    }

    #[test]
    fn reputation_serde_roundtrip() {
        let issuer = Identity::generate();
        let subject = Identity::generate();
        let mut rep = Reputation::new(subject.did());

        let gov_event = Reputation::issue_event(
            &issuer,
            subject.did(),
            ReputationCategory::Governance,
            10,
            "voted",
        )
        .unwrap();

        let trade_event = Reputation::issue_event(
            &issuer,
            subject.did(),
            ReputationCategory::Trading,
            5,
            "completed trade",
        )
        .unwrap();

        rep.apply(&gov_event).unwrap();
        rep.apply(&trade_event).unwrap();

        let json = serde_json::to_string(&rep).unwrap();
        let restored: Reputation = serde_json::from_str(&json).unwrap();

        assert_eq!(restored.did, subject.did());
        assert_eq!(restored.score(ReputationCategory::Governance), 10);
        assert_eq!(restored.score(ReputationCategory::Trading), 5);
        assert_eq!(restored.total_score(), 15);
        assert_eq!(restored.events().len(), 2);
    }
}
