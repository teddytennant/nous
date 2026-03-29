use chrono::{DateTime, Duration, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use nous_core::Error;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum EscrowStatus {
    Active,
    Released,
    Refunded,
    Disputed,
    Expired,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Escrow {
    pub id: String,
    pub buyer_did: String,
    pub seller_did: String,
    pub arbiter_did: Option<String>,
    pub token: String,
    pub amount: u128,
    pub status: EscrowStatus,
    pub created_at: DateTime<Utc>,
    pub expires_at: DateTime<Utc>,
    pub description: String,
    pub release_conditions: Vec<String>,
}

impl Escrow {
    pub fn new(
        buyer: &str,
        seller: &str,
        token: &str,
        amount: u128,
        description: &str,
        duration_hours: i64,
    ) -> Result<Self, Error> {
        if buyer == seller {
            return Err(Error::InvalidInput("buyer and seller must differ".into()));
        }
        if amount == 0 {
            return Err(Error::InvalidInput("amount must be positive".into()));
        }

        Ok(Self {
            id: format!("escrow:{}", Uuid::new_v4()),
            buyer_did: buyer.to_string(),
            seller_did: seller.to_string(),
            arbiter_did: None,
            token: token.to_string(),
            amount,
            status: EscrowStatus::Active,
            created_at: Utc::now(),
            expires_at: Utc::now() + Duration::hours(duration_hours),
            description: description.to_string(),
            release_conditions: Vec::new(),
        })
    }

    pub fn with_arbiter(mut self, arbiter_did: &str) -> Self {
        self.arbiter_did = Some(arbiter_did.to_string());
        self
    }

    pub fn add_condition(&mut self, condition: impl Into<String>) {
        self.release_conditions.push(condition.into());
    }

    pub fn is_expired(&self) -> bool {
        Utc::now() > self.expires_at
    }

    pub fn release(&mut self, caller_did: &str) -> Result<(), Error> {
        if self.status != EscrowStatus::Active {
            return Err(Error::InvalidInput(format!(
                "escrow is {:?}, not active",
                self.status
            )));
        }

        if self.is_expired() {
            self.status = EscrowStatus::Expired;
            return Err(Error::Expired("escrow has expired".into()));
        }

        // Only buyer or arbiter can release funds to seller
        let authorized =
            caller_did == self.buyer_did || self.arbiter_did.as_deref() == Some(caller_did);

        if !authorized {
            return Err(Error::PermissionDenied(
                "only buyer or arbiter can release escrow".into(),
            ));
        }

        self.status = EscrowStatus::Released;
        Ok(())
    }

    pub fn refund(&mut self, caller_did: &str) -> Result<(), Error> {
        if self.status != EscrowStatus::Active && self.status != EscrowStatus::Disputed {
            return Err(Error::InvalidInput(format!(
                "escrow is {:?}, cannot refund",
                self.status
            )));
        }

        // Only seller or arbiter can refund
        let authorized =
            caller_did == self.seller_did || self.arbiter_did.as_deref() == Some(caller_did);

        if !authorized {
            return Err(Error::PermissionDenied(
                "only seller or arbiter can refund escrow".into(),
            ));
        }

        self.status = EscrowStatus::Refunded;
        Ok(())
    }

    pub fn dispute(&mut self, caller_did: &str) -> Result<(), Error> {
        if self.status != EscrowStatus::Active {
            return Err(Error::InvalidInput("can only dispute active escrow".into()));
        }

        if caller_did != self.buyer_did && caller_did != self.seller_did {
            return Err(Error::PermissionDenied(
                "only buyer or seller can dispute".into(),
            ));
        }

        self.status = EscrowStatus::Disputed;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_escrow() -> Escrow {
        Escrow::new("buyer", "seller", "ETH", 1000, "test purchase", 24).unwrap()
    }

    #[test]
    fn create_escrow() {
        let escrow = test_escrow();
        assert_eq!(escrow.status, EscrowStatus::Active);
        assert_eq!(escrow.amount, 1000);
        assert!(escrow.id.starts_with("escrow:"));
    }

    #[test]
    fn reject_same_buyer_seller() {
        assert!(Escrow::new("alice", "alice", "ETH", 100, "test", 24).is_err());
    }

    #[test]
    fn reject_zero_amount() {
        assert!(Escrow::new("buyer", "seller", "ETH", 0, "test", 24).is_err());
    }

    #[test]
    fn buyer_releases() {
        let mut escrow = test_escrow();
        assert!(escrow.release("buyer").is_ok());
        assert_eq!(escrow.status, EscrowStatus::Released);
    }

    #[test]
    fn seller_cannot_release() {
        let mut escrow = test_escrow();
        assert!(escrow.release("seller").is_err());
    }

    #[test]
    fn arbiter_releases() {
        let mut escrow = test_escrow().with_arbiter("judge");
        assert!(escrow.release("judge").is_ok());
        assert_eq!(escrow.status, EscrowStatus::Released);
    }

    #[test]
    fn seller_refunds() {
        let mut escrow = test_escrow();
        assert!(escrow.refund("seller").is_ok());
        assert_eq!(escrow.status, EscrowStatus::Refunded);
    }

    #[test]
    fn buyer_cannot_refund() {
        let mut escrow = test_escrow();
        assert!(escrow.refund("buyer").is_err());
    }

    #[test]
    fn arbiter_refunds() {
        let mut escrow = test_escrow().with_arbiter("judge");
        assert!(escrow.refund("judge").is_ok());
        assert_eq!(escrow.status, EscrowStatus::Refunded);
    }

    #[test]
    fn dispute_flow() {
        let mut escrow = test_escrow().with_arbiter("judge");
        assert!(escrow.dispute("buyer").is_ok());
        assert_eq!(escrow.status, EscrowStatus::Disputed);

        // Arbiter can refund after dispute
        assert!(escrow.refund("judge").is_ok());
        assert_eq!(escrow.status, EscrowStatus::Refunded);
    }

    #[test]
    fn cannot_release_twice() {
        let mut escrow = test_escrow();
        escrow.release("buyer").unwrap();
        assert!(escrow.release("buyer").is_err());
    }

    #[test]
    fn cannot_dispute_released() {
        let mut escrow = test_escrow();
        escrow.release("buyer").unwrap();
        assert!(escrow.dispute("seller").is_err());
    }

    #[test]
    fn release_conditions() {
        let mut escrow = test_escrow();
        escrow.add_condition("item shipped");
        escrow.add_condition("item received");
        assert_eq!(escrow.release_conditions.len(), 2);
    }

    #[test]
    fn escrow_serializes() {
        let escrow = test_escrow();
        let json = serde_json::to_string(&escrow).unwrap();
        let _: Escrow = serde_json::from_str(&json).unwrap();
    }

    #[test]
    fn not_expired_within_duration() {
        let escrow = test_escrow();
        assert!(!escrow.is_expired());
    }
}
