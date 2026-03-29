use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use nous_core::Error;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum OrderStatus {
    Created,
    EscrowFunded,
    Shipped,
    Delivered,
    Completed,
    Disputed,
    Cancelled,
    Refunded,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShippingInfo {
    pub carrier: String,
    pub tracking_id: String,
    pub shipped_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Order {
    pub id: String,
    pub listing_id: String,
    pub buyer_did: String,
    pub seller_did: String,
    pub token: String,
    pub amount: u128,
    pub quantity: u32,
    pub status: OrderStatus,
    pub escrow_id: Option<String>,
    pub shipping: Option<ShippingInfo>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub completed_at: Option<DateTime<Utc>>,
    pub notes: Vec<String>,
}

impl Order {
    pub fn new(
        listing_id: impl Into<String>,
        buyer_did: impl Into<String>,
        seller_did: impl Into<String>,
        token: impl Into<String>,
        amount: u128,
        quantity: u32,
    ) -> Result<Self, Error> {
        let buyer = buyer_did.into();
        let seller = seller_did.into();

        if buyer == seller {
            return Err(Error::InvalidInput("buyer and seller must differ".into()));
        }
        if amount == 0 {
            return Err(Error::InvalidInput("amount must be positive".into()));
        }
        if quantity == 0 {
            return Err(Error::InvalidInput("quantity must be positive".into()));
        }

        let now = Utc::now();
        Ok(Self {
            id: format!("order:{}", Uuid::new_v4()),
            listing_id: listing_id.into(),
            buyer_did: buyer,
            seller_did: seller,
            token: token.into(),
            amount,
            quantity,
            status: OrderStatus::Created,
            escrow_id: None,
            shipping: None,
            created_at: now,
            updated_at: now,
            completed_at: None,
            notes: Vec::new(),
        })
    }

    pub fn with_note(mut self, note: impl Into<String>) -> Self {
        self.notes.push(note.into());
        self
    }

    pub fn fund_escrow(&mut self, escrow_id: impl Into<String>) -> Result<(), Error> {
        if self.status != OrderStatus::Created {
            return Err(Error::InvalidInput(format!(
                "cannot fund escrow: order is {:?}",
                self.status
            )));
        }
        self.escrow_id = Some(escrow_id.into());
        self.status = OrderStatus::EscrowFunded;
        self.updated_at = Utc::now();
        Ok(())
    }

    pub fn ship(
        &mut self,
        caller_did: &str,
        carrier: impl Into<String>,
        tracking_id: impl Into<String>,
    ) -> Result<(), Error> {
        if caller_did != self.seller_did {
            return Err(Error::PermissionDenied("only seller can ship".into()));
        }
        if self.status != OrderStatus::EscrowFunded {
            return Err(Error::InvalidInput(format!(
                "cannot ship: order is {:?}",
                self.status
            )));
        }
        self.shipping = Some(ShippingInfo {
            carrier: carrier.into(),
            tracking_id: tracking_id.into(),
            shipped_at: Utc::now(),
        });
        self.status = OrderStatus::Shipped;
        self.updated_at = Utc::now();
        Ok(())
    }

    pub fn confirm_delivery(&mut self, caller_did: &str) -> Result<(), Error> {
        if caller_did != self.buyer_did {
            return Err(Error::PermissionDenied(
                "only buyer can confirm delivery".into(),
            ));
        }
        if self.status != OrderStatus::Shipped {
            return Err(Error::InvalidInput(format!(
                "cannot confirm delivery: order is {:?}",
                self.status
            )));
        }
        self.status = OrderStatus::Delivered;
        self.updated_at = Utc::now();
        Ok(())
    }

    pub fn complete(&mut self, caller_did: &str) -> Result<(), Error> {
        if caller_did != self.buyer_did {
            return Err(Error::PermissionDenied(
                "only buyer can complete order".into(),
            ));
        }
        if self.status != OrderStatus::Delivered {
            return Err(Error::InvalidInput(format!(
                "cannot complete: order is {:?}",
                self.status
            )));
        }
        self.status = OrderStatus::Completed;
        self.completed_at = Some(Utc::now());
        self.updated_at = Utc::now();
        Ok(())
    }

    pub fn dispute(&mut self, caller_did: &str) -> Result<(), Error> {
        let is_party = caller_did == self.buyer_did || caller_did == self.seller_did;
        if !is_party {
            return Err(Error::PermissionDenied(
                "only buyer or seller can dispute".into(),
            ));
        }
        match self.status {
            OrderStatus::EscrowFunded | OrderStatus::Shipped | OrderStatus::Delivered => {}
            _ => {
                return Err(Error::InvalidInput(format!(
                    "cannot dispute: order is {:?}",
                    self.status
                )));
            }
        }
        self.status = OrderStatus::Disputed;
        self.updated_at = Utc::now();
        Ok(())
    }

    pub fn cancel(&mut self, caller_did: &str) -> Result<(), Error> {
        if caller_did != self.buyer_did {
            return Err(Error::PermissionDenied(
                "only buyer can cancel order".into(),
            ));
        }
        match self.status {
            OrderStatus::Created | OrderStatus::EscrowFunded => {}
            _ => {
                return Err(Error::InvalidInput(format!(
                    "cannot cancel: order is {:?}",
                    self.status
                )));
            }
        }
        self.status = OrderStatus::Cancelled;
        self.updated_at = Utc::now();
        Ok(())
    }

    pub fn refund(&mut self, caller_did: &str) -> Result<(), Error> {
        let is_seller = caller_did == self.seller_did;
        let is_arbiter = !is_seller && caller_did != self.buyer_did;
        if !is_seller && !is_arbiter {
            return Err(Error::PermissionDenied(
                "only seller or arbiter can refund".into(),
            ));
        }
        if self.status != OrderStatus::Disputed && self.status != OrderStatus::EscrowFunded {
            return Err(Error::InvalidInput(format!(
                "cannot refund: order is {:?}",
                self.status
            )));
        }
        self.status = OrderStatus::Refunded;
        self.updated_at = Utc::now();
        Ok(())
    }

    pub fn is_active(&self) -> bool {
        matches!(
            self.status,
            OrderStatus::Created
                | OrderStatus::EscrowFunded
                | OrderStatus::Shipped
                | OrderStatus::Delivered
        )
    }

    pub fn is_terminal(&self) -> bool {
        matches!(
            self.status,
            OrderStatus::Completed | OrderStatus::Cancelled | OrderStatus::Refunded
        )
    }

    pub fn total_cost(&self) -> u128 {
        self.amount * self.quantity as u128
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_order() -> Order {
        Order::new(
            "listing:abc",
            "did:key:buyer",
            "did:key:seller",
            "ETH",
            1000,
            1,
        )
        .unwrap()
    }

    #[test]
    fn create_order() {
        let order = test_order();
        assert_eq!(order.status, OrderStatus::Created);
        assert!(order.id.starts_with("order:"));
        assert_eq!(order.amount, 1000);
        assert_eq!(order.quantity, 1);
        assert!(order.is_active());
        assert!(!order.is_terminal());
    }

    #[test]
    fn reject_same_buyer_seller() {
        assert!(Order::new("listing:x", "did:key:a", "did:key:a", "ETH", 100, 1).is_err());
    }

    #[test]
    fn reject_zero_amount() {
        assert!(Order::new("listing:x", "did:key:buyer", "did:key:seller", "ETH", 0, 1).is_err());
    }

    #[test]
    fn reject_zero_quantity() {
        assert!(
            Order::new(
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
    fn total_cost() {
        let order = Order::new(
            "listing:x",
            "did:key:buyer",
            "did:key:seller",
            "ETH",
            500,
            3,
        )
        .unwrap();
        assert_eq!(order.total_cost(), 1500);
    }

    #[test]
    fn full_happy_path() {
        let mut order = test_order();
        order.fund_escrow("escrow:123").unwrap();
        assert_eq!(order.status, OrderStatus::EscrowFunded);
        assert_eq!(order.escrow_id.as_deref(), Some("escrow:123"));

        order.ship("did:key:seller", "FedEx", "TRACK123").unwrap();
        assert_eq!(order.status, OrderStatus::Shipped);
        assert!(order.shipping.is_some());
        assert_eq!(order.shipping.as_ref().unwrap().carrier, "FedEx");

        order.confirm_delivery("did:key:buyer").unwrap();
        assert_eq!(order.status, OrderStatus::Delivered);

        order.complete("did:key:buyer").unwrap();
        assert_eq!(order.status, OrderStatus::Completed);
        assert!(order.completed_at.is_some());
        assert!(order.is_terminal());
    }

    #[test]
    fn cancel_before_shipping() {
        let mut order = test_order();
        order.cancel("did:key:buyer").unwrap();
        assert_eq!(order.status, OrderStatus::Cancelled);
        assert!(order.is_terminal());
    }

    #[test]
    fn cancel_after_escrow() {
        let mut order = test_order();
        order.fund_escrow("escrow:x").unwrap();
        order.cancel("did:key:buyer").unwrap();
        assert_eq!(order.status, OrderStatus::Cancelled);
    }

    #[test]
    fn cannot_cancel_after_shipping() {
        let mut order = test_order();
        order.fund_escrow("escrow:x").unwrap();
        order.ship("did:key:seller", "UPS", "T1").unwrap();
        assert!(order.cancel("did:key:buyer").is_err());
    }

    #[test]
    fn seller_cannot_cancel() {
        let mut order = test_order();
        assert!(order.cancel("did:key:seller").is_err());
    }

    #[test]
    fn dispute_after_escrow() {
        let mut order = test_order();
        order.fund_escrow("escrow:x").unwrap();
        order.dispute("did:key:buyer").unwrap();
        assert_eq!(order.status, OrderStatus::Disputed);
    }

    #[test]
    fn dispute_after_shipping() {
        let mut order = test_order();
        order.fund_escrow("escrow:x").unwrap();
        order.ship("did:key:seller", "DHL", "T2").unwrap();
        order.dispute("did:key:buyer").unwrap();
        assert_eq!(order.status, OrderStatus::Disputed);
    }

    #[test]
    fn dispute_after_delivery() {
        let mut order = test_order();
        order.fund_escrow("escrow:x").unwrap();
        order.ship("did:key:seller", "DHL", "T2").unwrap();
        order.confirm_delivery("did:key:buyer").unwrap();
        order.dispute("did:key:buyer").unwrap();
        assert_eq!(order.status, OrderStatus::Disputed);
    }

    #[test]
    fn cannot_dispute_created_order() {
        let mut order = test_order();
        assert!(order.dispute("did:key:buyer").is_err());
    }

    #[test]
    fn cannot_dispute_completed_order() {
        let mut order = test_order();
        order.fund_escrow("escrow:x").unwrap();
        order.ship("did:key:seller", "DHL", "T2").unwrap();
        order.confirm_delivery("did:key:buyer").unwrap();
        order.complete("did:key:buyer").unwrap();
        assert!(order.dispute("did:key:buyer").is_err());
    }

    #[test]
    fn third_party_cannot_dispute() {
        let mut order = test_order();
        order.fund_escrow("escrow:x").unwrap();
        assert!(order.dispute("did:key:random").is_err());
    }

    #[test]
    fn refund_disputed_order() {
        let mut order = test_order();
        order.fund_escrow("escrow:x").unwrap();
        order.dispute("did:key:buyer").unwrap();
        order.refund("did:key:seller").unwrap();
        assert_eq!(order.status, OrderStatus::Refunded);
        assert!(order.is_terminal());
    }

    #[test]
    fn arbiter_refund() {
        let mut order = test_order();
        order.fund_escrow("escrow:x").unwrap();
        order.dispute("did:key:buyer").unwrap();
        order.refund("did:key:arbiter").unwrap();
        assert_eq!(order.status, OrderStatus::Refunded);
    }

    #[test]
    fn buyer_cannot_refund() {
        let mut order = test_order();
        order.fund_escrow("escrow:x").unwrap();
        order.dispute("did:key:buyer").unwrap();
        assert!(order.refund("did:key:buyer").is_err());
    }

    #[test]
    fn cannot_ship_without_escrow() {
        let mut order = test_order();
        assert!(order.ship("did:key:seller", "UPS", "T1").is_err());
    }

    #[test]
    fn buyer_cannot_ship() {
        let mut order = test_order();
        order.fund_escrow("escrow:x").unwrap();
        assert!(order.ship("did:key:buyer", "UPS", "T1").is_err());
    }

    #[test]
    fn seller_cannot_confirm_delivery() {
        let mut order = test_order();
        order.fund_escrow("escrow:x").unwrap();
        order.ship("did:key:seller", "UPS", "T1").unwrap();
        assert!(order.confirm_delivery("did:key:seller").is_err());
    }

    #[test]
    fn seller_cannot_complete() {
        let mut order = test_order();
        order.fund_escrow("escrow:x").unwrap();
        order.ship("did:key:seller", "UPS", "T1").unwrap();
        order.confirm_delivery("did:key:buyer").unwrap();
        assert!(order.complete("did:key:seller").is_err());
    }

    #[test]
    fn with_note() {
        let order = test_order().with_note("rush order");
        assert_eq!(order.notes, vec!["rush order"]);
    }

    #[test]
    fn order_serializes() {
        let mut order = test_order();
        order.fund_escrow("escrow:x").unwrap();
        order.ship("did:key:seller", "FedEx", "T1").unwrap();
        let json = serde_json::to_string(&order).unwrap();
        let restored: Order = serde_json::from_str(&json).unwrap();
        assert_eq!(restored.status, OrderStatus::Shipped);
        assert_eq!(restored.shipping.unwrap().carrier, "FedEx");
    }

    #[test]
    fn cannot_double_fund() {
        let mut order = test_order();
        order.fund_escrow("escrow:x").unwrap();
        assert!(order.fund_escrow("escrow:y").is_err());
    }
}
