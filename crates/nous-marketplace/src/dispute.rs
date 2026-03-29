use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use nous_core::Error;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum DisputeStatus {
    Open,
    UnderReview,
    ResolvedBuyerWins,
    ResolvedSellerWins,
    Escalated,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum DisputeReason {
    ItemNotReceived,
    ItemNotAsDescribed,
    QualityIssue,
    Counterfeit,
    SellerUnresponsive,
    Other,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Evidence {
    pub submitted_by: String,
    pub description: String,
    pub attachments: Vec<String>,
    pub submitted_at: DateTime<Utc>,
}

impl Evidence {
    pub fn new(
        submitted_by: impl Into<String>,
        description: impl Into<String>,
    ) -> Result<Self, Error> {
        let desc = description.into();
        if desc.is_empty() {
            return Err(Error::InvalidInput(
                "evidence description cannot be empty".into(),
            ));
        }
        Ok(Self {
            submitted_by: submitted_by.into(),
            description: desc,
            attachments: Vec::new(),
            submitted_at: Utc::now(),
        })
    }

    pub fn with_attachment(mut self, cid: impl Into<String>) -> Self {
        self.attachments.push(cid.into());
        self
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Dispute {
    pub id: String,
    pub order_id: String,
    pub initiator_did: String,
    pub respondent_did: String,
    pub reason: DisputeReason,
    pub description: String,
    pub evidence: Vec<Evidence>,
    pub status: DisputeStatus,
    pub arbiter_did: Option<String>,
    pub resolution_note: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub resolved_at: Option<DateTime<Utc>>,
}

impl Dispute {
    pub fn new(
        order_id: impl Into<String>,
        initiator_did: impl Into<String>,
        respondent_did: impl Into<String>,
        reason: DisputeReason,
        description: impl Into<String>,
    ) -> Result<Self, Error> {
        let initiator = initiator_did.into();
        let respondent = respondent_did.into();
        let desc = description.into();

        if initiator == respondent {
            return Err(Error::InvalidInput(
                "initiator and respondent must differ".into(),
            ));
        }
        if desc.is_empty() {
            return Err(Error::InvalidInput("description cannot be empty".into()));
        }

        let now = Utc::now();
        Ok(Self {
            id: format!("dispute:{}", Uuid::new_v4()),
            order_id: order_id.into(),
            initiator_did: initiator,
            respondent_did: respondent,
            reason,
            description: desc,
            evidence: Vec::new(),
            status: DisputeStatus::Open,
            arbiter_did: None,
            resolution_note: None,
            created_at: now,
            updated_at: now,
            resolved_at: None,
        })
    }

    pub fn add_evidence(&mut self, evidence: Evidence, caller_did: &str) -> Result<(), Error> {
        if self.is_resolved() {
            return Err(Error::InvalidInput(
                "cannot add evidence to resolved dispute".into(),
            ));
        }
        let is_party = caller_did == self.initiator_did || caller_did == self.respondent_did;
        if !is_party {
            return Err(Error::PermissionDenied(
                "only dispute parties can submit evidence".into(),
            ));
        }
        self.evidence.push(evidence);
        self.updated_at = Utc::now();
        Ok(())
    }

    pub fn assign_arbiter(&mut self, arbiter_did: impl Into<String>) -> Result<(), Error> {
        if self.is_resolved() {
            return Err(Error::InvalidInput(
                "cannot assign arbiter to resolved dispute".into(),
            ));
        }
        let arbiter = arbiter_did.into();
        if arbiter == self.initiator_did || arbiter == self.respondent_did {
            return Err(Error::InvalidInput(
                "arbiter cannot be a dispute party".into(),
            ));
        }
        self.arbiter_did = Some(arbiter);
        self.status = DisputeStatus::UnderReview;
        self.updated_at = Utc::now();
        Ok(())
    }

    pub fn resolve_buyer_wins(
        &mut self,
        caller_did: &str,
        note: impl Into<String>,
    ) -> Result<(), Error> {
        self.resolve(caller_did, DisputeStatus::ResolvedBuyerWins, note)
    }

    pub fn resolve_seller_wins(
        &mut self,
        caller_did: &str,
        note: impl Into<String>,
    ) -> Result<(), Error> {
        self.resolve(caller_did, DisputeStatus::ResolvedSellerWins, note)
    }

    pub fn escalate(&mut self, caller_did: &str) -> Result<(), Error> {
        let is_party = caller_did == self.initiator_did || caller_did == self.respondent_did;
        if !is_party {
            return Err(Error::PermissionDenied("only parties can escalate".into()));
        }
        match self.status {
            DisputeStatus::Open | DisputeStatus::UnderReview => {}
            _ => {
                return Err(Error::InvalidInput(format!(
                    "cannot escalate: dispute is {:?}",
                    self.status
                )));
            }
        }
        self.status = DisputeStatus::Escalated;
        self.updated_at = Utc::now();
        Ok(())
    }

    pub fn is_resolved(&self) -> bool {
        matches!(
            self.status,
            DisputeStatus::ResolvedBuyerWins | DisputeStatus::ResolvedSellerWins
        )
    }

    pub fn evidence_count(&self) -> usize {
        self.evidence.len()
    }

    fn resolve(
        &mut self,
        caller_did: &str,
        outcome: DisputeStatus,
        note: impl Into<String>,
    ) -> Result<(), Error> {
        if self.is_resolved() {
            return Err(Error::InvalidInput("dispute already resolved".into()));
        }

        let is_arbiter = self.arbiter_did.as_deref() == Some(caller_did);
        if !is_arbiter {
            return Err(Error::PermissionDenied(
                "only arbiter can resolve disputes".into(),
            ));
        }

        self.status = outcome;
        self.resolution_note = Some(note.into());
        self.resolved_at = Some(Utc::now());
        self.updated_at = Utc::now();
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_dispute() -> Dispute {
        Dispute::new(
            "order:abc",
            "did:key:buyer",
            "did:key:seller",
            DisputeReason::ItemNotReceived,
            "Item never arrived after 30 days",
        )
        .unwrap()
    }

    #[test]
    fn create_dispute() {
        let d = test_dispute();
        assert_eq!(d.status, DisputeStatus::Open);
        assert!(d.id.starts_with("dispute:"));
        assert_eq!(d.reason, DisputeReason::ItemNotReceived);
        assert!(!d.is_resolved());
    }

    #[test]
    fn reject_same_parties() {
        assert!(
            Dispute::new(
                "order:x",
                "did:key:a",
                "did:key:a",
                DisputeReason::Other,
                "test",
            )
            .is_err()
        );
    }

    #[test]
    fn reject_empty_description() {
        assert!(
            Dispute::new(
                "order:x",
                "did:key:buyer",
                "did:key:seller",
                DisputeReason::Other,
                "",
            )
            .is_err()
        );
    }

    #[test]
    fn add_evidence_by_initiator() {
        let mut d = test_dispute();
        let ev = Evidence::new("did:key:buyer", "Photo of empty package")
            .unwrap()
            .with_attachment("Qm123");
        d.add_evidence(ev, "did:key:buyer").unwrap();
        assert_eq!(d.evidence_count(), 1);
        assert_eq!(d.evidence[0].attachments, vec!["Qm123"]);
    }

    #[test]
    fn add_evidence_by_respondent() {
        let mut d = test_dispute();
        let ev = Evidence::new("did:key:seller", "Shipping receipt").unwrap();
        d.add_evidence(ev, "did:key:seller").unwrap();
        assert_eq!(d.evidence_count(), 1);
    }

    #[test]
    fn third_party_cannot_add_evidence() {
        let mut d = test_dispute();
        let ev = Evidence::new("did:key:random", "I saw something").unwrap();
        assert!(d.add_evidence(ev, "did:key:random").is_err());
    }

    #[test]
    fn reject_empty_evidence_description() {
        assert!(Evidence::new("did:key:buyer", "").is_err());
    }

    #[test]
    fn assign_arbiter() {
        let mut d = test_dispute();
        d.assign_arbiter("did:key:judge").unwrap();
        assert_eq!(d.arbiter_did.as_deref(), Some("did:key:judge"));
        assert_eq!(d.status, DisputeStatus::UnderReview);
    }

    #[test]
    fn party_cannot_be_arbiter() {
        let mut d = test_dispute();
        assert!(d.assign_arbiter("did:key:buyer").is_err());
        assert!(d.assign_arbiter("did:key:seller").is_err());
    }

    #[test]
    fn arbiter_resolves_buyer_wins() {
        let mut d = test_dispute();
        d.assign_arbiter("did:key:judge").unwrap();
        d.resolve_buyer_wins("did:key:judge", "Seller failed to provide tracking")
            .unwrap();
        assert_eq!(d.status, DisputeStatus::ResolvedBuyerWins);
        assert!(d.is_resolved());
        assert!(d.resolved_at.is_some());
        assert!(d.resolution_note.is_some());
    }

    #[test]
    fn arbiter_resolves_seller_wins() {
        let mut d = test_dispute();
        d.assign_arbiter("did:key:judge").unwrap();
        d.resolve_seller_wins("did:key:judge", "Buyer confirmed receipt in messages")
            .unwrap();
        assert_eq!(d.status, DisputeStatus::ResolvedSellerWins);
        assert!(d.is_resolved());
    }

    #[test]
    fn non_arbiter_cannot_resolve() {
        let mut d = test_dispute();
        d.assign_arbiter("did:key:judge").unwrap();
        assert!(d.resolve_buyer_wins("did:key:buyer", "I win").is_err());
        assert!(d.resolve_seller_wins("did:key:seller", "No I win").is_err());
    }

    #[test]
    fn cannot_resolve_twice() {
        let mut d = test_dispute();
        d.assign_arbiter("did:key:judge").unwrap();
        d.resolve_buyer_wins("did:key:judge", "Buyer wins").unwrap();
        assert!(
            d.resolve_seller_wins("did:key:judge", "Changed mind")
                .is_err()
        );
    }

    #[test]
    fn cannot_add_evidence_after_resolution() {
        let mut d = test_dispute();
        d.assign_arbiter("did:key:judge").unwrap();
        d.resolve_buyer_wins("did:key:judge", "Done").unwrap();
        let ev = Evidence::new("did:key:buyer", "Late evidence").unwrap();
        assert!(d.add_evidence(ev, "did:key:buyer").is_err());
    }

    #[test]
    fn escalate_open_dispute() {
        let mut d = test_dispute();
        d.escalate("did:key:buyer").unwrap();
        assert_eq!(d.status, DisputeStatus::Escalated);
    }

    #[test]
    fn escalate_under_review() {
        let mut d = test_dispute();
        d.assign_arbiter("did:key:judge").unwrap();
        d.escalate("did:key:seller").unwrap();
        assert_eq!(d.status, DisputeStatus::Escalated);
    }

    #[test]
    fn third_party_cannot_escalate() {
        let mut d = test_dispute();
        assert!(d.escalate("did:key:random").is_err());
    }

    #[test]
    fn cannot_escalate_resolved() {
        let mut d = test_dispute();
        d.assign_arbiter("did:key:judge").unwrap();
        d.resolve_buyer_wins("did:key:judge", "Done").unwrap();
        assert!(d.escalate("did:key:buyer").is_err());
    }

    #[test]
    fn dispute_serializes() {
        let mut d = test_dispute();
        let ev = Evidence::new("did:key:buyer", "Photo")
            .unwrap()
            .with_attachment("Qmabc");
        d.add_evidence(ev, "did:key:buyer").unwrap();
        let json = serde_json::to_string(&d).unwrap();
        let restored: Dispute = serde_json::from_str(&json).unwrap();
        assert_eq!(restored.reason, DisputeReason::ItemNotReceived);
        assert_eq!(restored.evidence_count(), 1);
    }

    #[test]
    fn cannot_assign_arbiter_to_resolved() {
        let mut d = test_dispute();
        d.assign_arbiter("did:key:judge").unwrap();
        d.resolve_buyer_wins("did:key:judge", "Done").unwrap();
        assert!(d.assign_arbiter("did:key:judge2").is_err());
    }

    #[test]
    fn multiple_evidence_submissions() {
        let mut d = test_dispute();
        for i in 0..5 {
            let ev = Evidence::new("did:key:buyer", format!("Evidence {}", i)).unwrap();
            d.add_evidence(ev, "did:key:buyer").unwrap();
        }
        assert_eq!(d.evidence_count(), 5);
    }
}
