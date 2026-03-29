use chrono::{DateTime, Duration, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use nous_core::Error;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum OfferStatus {
    Pending,
    Accepted,
    Rejected,
    Expired,
    Withdrawn,
    Countered,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Offer {
    pub id: String,
    pub listing_id: String,
    pub buyer_did: String,
    pub seller_did: String,
    pub token: String,
    pub amount: u128,
    pub message: Option<String>,
    pub status: OfferStatus,
    pub counter_amount: Option<u128>,
    pub created_at: DateTime<Utc>,
    pub expires_at: DateTime<Utc>,
    pub responded_at: Option<DateTime<Utc>>,
}

impl Offer {
    pub fn new(
        listing_id: impl Into<String>,
        buyer_did: impl Into<String>,
        seller_did: impl Into<String>,
        token: impl Into<String>,
        amount: u128,
        duration_hours: i64,
    ) -> Result<Self, Error> {
        let buyer = buyer_did.into();
        let seller = seller_did.into();

        if buyer == seller {
            return Err(Error::InvalidInput("buyer and seller must differ".into()));
        }
        if amount == 0 {
            return Err(Error::InvalidInput("offer amount must be positive".into()));
        }
        if duration_hours <= 0 {
            return Err(Error::InvalidInput("duration must be positive".into()));
        }

        Ok(Self {
            id: format!("offer:{}", Uuid::new_v4()),
            listing_id: listing_id.into(),
            buyer_did: buyer,
            seller_did: seller,
            token: token.into(),
            amount,
            message: None,
            status: OfferStatus::Pending,
            counter_amount: None,
            created_at: Utc::now(),
            expires_at: Utc::now() + Duration::hours(duration_hours),
            responded_at: None,
        })
    }

    pub fn with_message(mut self, msg: impl Into<String>) -> Self {
        self.message = Some(msg.into());
        self
    }

    pub fn accept(&mut self, caller_did: &str) -> Result<(), Error> {
        if caller_did != self.seller_did {
            return Err(Error::PermissionDenied(
                "only seller can accept offers".into(),
            ));
        }
        self.require_pending()?;
        self.status = OfferStatus::Accepted;
        self.responded_at = Some(Utc::now());
        Ok(())
    }

    pub fn reject(&mut self, caller_did: &str) -> Result<(), Error> {
        if caller_did != self.seller_did {
            return Err(Error::PermissionDenied(
                "only seller can reject offers".into(),
            ));
        }
        self.require_pending()?;
        self.status = OfferStatus::Rejected;
        self.responded_at = Some(Utc::now());
        Ok(())
    }

    pub fn counter(&mut self, caller_did: &str, counter_amount: u128) -> Result<(), Error> {
        if caller_did != self.seller_did {
            return Err(Error::PermissionDenied(
                "only seller can counter offers".into(),
            ));
        }
        self.require_pending()?;
        if counter_amount == 0 {
            return Err(Error::InvalidInput(
                "counter amount must be positive".into(),
            ));
        }
        if counter_amount == self.amount {
            return Err(Error::InvalidInput(
                "counter must differ from original amount".into(),
            ));
        }
        self.status = OfferStatus::Countered;
        self.counter_amount = Some(counter_amount);
        self.responded_at = Some(Utc::now());
        Ok(())
    }

    pub fn withdraw(&mut self, caller_did: &str) -> Result<(), Error> {
        if caller_did != self.buyer_did {
            return Err(Error::PermissionDenied(
                "only buyer can withdraw offers".into(),
            ));
        }
        if self.status != OfferStatus::Pending && self.status != OfferStatus::Countered {
            return Err(Error::InvalidInput(format!(
                "cannot withdraw: offer is {:?}",
                self.status
            )));
        }
        self.status = OfferStatus::Withdrawn;
        self.responded_at = Some(Utc::now());
        Ok(())
    }

    pub fn is_expired(&self) -> bool {
        self.status == OfferStatus::Pending && Utc::now() > self.expires_at
    }

    pub fn is_actionable(&self) -> bool {
        self.status == OfferStatus::Pending && !self.is_expired()
    }

    pub fn is_terminal(&self) -> bool {
        matches!(
            self.status,
            OfferStatus::Accepted
                | OfferStatus::Rejected
                | OfferStatus::Expired
                | OfferStatus::Withdrawn
        )
    }

    fn require_pending(&self) -> Result<(), Error> {
        if self.status != OfferStatus::Pending {
            return Err(Error::InvalidInput(format!(
                "offer is {:?}, not pending",
                self.status
            )));
        }
        if Utc::now() > self.expires_at {
            return Err(Error::Expired("offer has expired".into()));
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_offer() -> Offer {
        Offer::new(
            "listing:abc",
            "did:key:buyer",
            "did:key:seller",
            "ETH",
            800,
            48,
        )
        .unwrap()
    }

    #[test]
    fn create_offer() {
        let offer = test_offer();
        assert_eq!(offer.status, OfferStatus::Pending);
        assert!(offer.id.starts_with("offer:"));
        assert_eq!(offer.amount, 800);
        assert!(offer.is_actionable());
        assert!(!offer.is_terminal());
    }

    #[test]
    fn reject_same_buyer_seller() {
        assert!(Offer::new("listing:x", "did:key:a", "did:key:a", "ETH", 100, 24).is_err());
    }

    #[test]
    fn reject_zero_amount() {
        assert!(Offer::new("listing:x", "did:key:buyer", "did:key:seller", "ETH", 0, 24).is_err());
    }

    #[test]
    fn reject_zero_duration() {
        assert!(
            Offer::new(
                "listing:x",
                "did:key:buyer",
                "did:key:seller",
                "ETH",
                100,
                0
            )
            .is_err()
        );
    }

    #[test]
    fn with_message() {
        let offer = test_offer().with_message("I really want this");
        assert_eq!(offer.message.as_deref(), Some("I really want this"));
    }

    #[test]
    fn seller_accepts() {
        let mut offer = test_offer();
        offer.accept("did:key:seller").unwrap();
        assert_eq!(offer.status, OfferStatus::Accepted);
        assert!(offer.responded_at.is_some());
        assert!(offer.is_terminal());
    }

    #[test]
    fn buyer_cannot_accept() {
        let mut offer = test_offer();
        assert!(offer.accept("did:key:buyer").is_err());
    }

    #[test]
    fn seller_rejects() {
        let mut offer = test_offer();
        offer.reject("did:key:seller").unwrap();
        assert_eq!(offer.status, OfferStatus::Rejected);
        assert!(offer.is_terminal());
    }

    #[test]
    fn buyer_cannot_reject() {
        let mut offer = test_offer();
        assert!(offer.reject("did:key:buyer").is_err());
    }

    #[test]
    fn seller_counters() {
        let mut offer = test_offer();
        offer.counter("did:key:seller", 900).unwrap();
        assert_eq!(offer.status, OfferStatus::Countered);
        assert_eq!(offer.counter_amount, Some(900));
        assert!(offer.responded_at.is_some());
    }

    #[test]
    fn counter_must_differ() {
        let mut offer = test_offer();
        assert!(offer.counter("did:key:seller", 800).is_err());
    }

    #[test]
    fn counter_must_be_positive() {
        let mut offer = test_offer();
        assert!(offer.counter("did:key:seller", 0).is_err());
    }

    #[test]
    fn buyer_cannot_counter() {
        let mut offer = test_offer();
        assert!(offer.counter("did:key:buyer", 900).is_err());
    }

    #[test]
    fn buyer_withdraws() {
        let mut offer = test_offer();
        offer.withdraw("did:key:buyer").unwrap();
        assert_eq!(offer.status, OfferStatus::Withdrawn);
        assert!(offer.is_terminal());
    }

    #[test]
    fn seller_cannot_withdraw() {
        let mut offer = test_offer();
        assert!(offer.withdraw("did:key:seller").is_err());
    }

    #[test]
    fn withdraw_countered_offer() {
        let mut offer = test_offer();
        offer.counter("did:key:seller", 900).unwrap();
        offer.withdraw("did:key:buyer").unwrap();
        assert_eq!(offer.status, OfferStatus::Withdrawn);
    }

    #[test]
    fn cannot_accept_after_reject() {
        let mut offer = test_offer();
        offer.reject("did:key:seller").unwrap();
        assert!(offer.accept("did:key:seller").is_err());
    }

    #[test]
    fn cannot_reject_after_accept() {
        let mut offer = test_offer();
        offer.accept("did:key:seller").unwrap();
        assert!(offer.reject("did:key:seller").is_err());
    }

    #[test]
    fn cannot_withdraw_accepted() {
        let mut offer = test_offer();
        offer.accept("did:key:seller").unwrap();
        assert!(offer.withdraw("did:key:buyer").is_err());
    }

    #[test]
    fn not_expired_within_duration() {
        let offer = test_offer();
        assert!(!offer.is_expired());
    }

    #[test]
    fn offer_serializes() {
        let offer = test_offer().with_message("Please");
        let json = serde_json::to_string(&offer).unwrap();
        let restored: Offer = serde_json::from_str(&json).unwrap();
        assert_eq!(restored.amount, 800);
        assert_eq!(restored.message.as_deref(), Some("Please"));
    }

    #[test]
    fn third_party_cannot_act() {
        let mut offer = test_offer();
        assert!(offer.accept("did:key:random").is_err());
        assert!(offer.reject("did:key:random").is_err());
        assert!(offer.counter("did:key:random", 900).is_err());
        assert!(offer.withdraw("did:key:random").is_err());
    }
}
