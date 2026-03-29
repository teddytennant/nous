use chrono::{DateTime, Duration, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use nous_core::{Error, Result};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SwapStatus {
    Pending,
    Locked,
    Completed,
    Refunded,
    Expired,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SwapOrder {
    pub id: String,
    pub initiator: String,
    pub counterparty: Option<String>,
    pub offer_token: String,
    pub offer_amount: u128,
    pub want_token: String,
    pub want_amount: u128,
    pub status: SwapStatus,
    pub created_at: DateTime<Utc>,
    pub expires_at: DateTime<Utc>,
    pub hashlock: Option<Vec<u8>>,
    pub preimage: Option<Vec<u8>>,
}

impl SwapOrder {
    pub fn new(
        initiator: &str,
        offer_token: &str,
        offer_amount: u128,
        want_token: &str,
        want_amount: u128,
        ttl_hours: i64,
    ) -> Result<Self> {
        if offer_amount == 0 || want_amount == 0 {
            return Err(Error::InvalidInput("amounts must be positive".into()));
        }
        if offer_token == want_token {
            return Err(Error::InvalidInput("cannot swap same token".into()));
        }

        Ok(Self {
            id: format!("swap:{}", Uuid::new_v4()),
            initiator: initiator.into(),
            counterparty: None,
            offer_token: offer_token.into(),
            offer_amount,
            want_token: want_token.into(),
            want_amount,
            status: SwapStatus::Pending,
            created_at: Utc::now(),
            expires_at: Utc::now() + Duration::hours(ttl_hours),
            hashlock: None,
            preimage: None,
        })
    }

    pub fn exchange_rate(&self) -> f64 {
        self.want_amount as f64 / self.offer_amount as f64
    }

    pub fn inverse_rate(&self) -> f64 {
        self.offer_amount as f64 / self.want_amount as f64
    }

    pub fn is_expired(&self) -> bool {
        Utc::now() > self.expires_at
    }

    pub fn accept(&mut self, counterparty: &str) -> Result<()> {
        if self.status != SwapStatus::Pending {
            return Err(Error::InvalidInput("swap is not pending".into()));
        }
        if self.is_expired() {
            self.status = SwapStatus::Expired;
            return Err(Error::Expired("swap has expired".into()));
        }
        if counterparty == self.initiator {
            return Err(Error::InvalidInput("cannot accept own swap".into()));
        }

        self.counterparty = Some(counterparty.into());
        self.status = SwapStatus::Locked;
        Ok(())
    }

    pub fn complete(&mut self, caller: &str) -> Result<()> {
        if self.status != SwapStatus::Locked {
            return Err(Error::InvalidInput("swap is not locked".into()));
        }
        let is_party = caller == self.initiator || self.counterparty.as_deref() == Some(caller);
        if !is_party {
            return Err(Error::PermissionDenied(
                "only swap parties can complete".into(),
            ));
        }

        self.status = SwapStatus::Completed;
        Ok(())
    }

    pub fn refund(&mut self, caller: &str) -> Result<()> {
        if self.status != SwapStatus::Pending && self.status != SwapStatus::Locked {
            return Err(Error::InvalidInput("swap cannot be refunded".into()));
        }
        if caller != self.initiator {
            return Err(Error::PermissionDenied("only initiator can refund".into()));
        }
        if self.status == SwapStatus::Locked && !self.is_expired() {
            return Err(Error::InvalidInput(
                "cannot refund locked swap before expiry".into(),
            ));
        }

        self.status = SwapStatus::Refunded;
        Ok(())
    }

    pub fn with_hashlock(mut self, hashlock: Vec<u8>) -> Self {
        self.hashlock = Some(hashlock);
        self
    }

    pub fn reveal_preimage(&mut self, preimage: Vec<u8>) -> Result<()> {
        let hashlock = self
            .hashlock
            .as_ref()
            .ok_or_else(|| Error::InvalidInput("swap has no hashlock".into()))?;

        let hash = sha256(&preimage);
        if hash != *hashlock {
            return Err(Error::Crypto("preimage does not match hashlock".into()));
        }

        self.preimage = Some(preimage);
        Ok(())
    }
}

fn sha256(data: &[u8]) -> Vec<u8> {
    use sha2::{Digest, Sha256};
    let mut hasher = Sha256::new();
    hasher.update(data);
    hasher.finalize().to_vec()
}

#[derive(Debug, Default)]
pub struct SwapBook {
    orders: Vec<SwapOrder>,
}

impl SwapBook {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn add(&mut self, order: SwapOrder) {
        self.orders.push(order);
    }

    pub fn pending(&self) -> Vec<&SwapOrder> {
        self.orders
            .iter()
            .filter(|o| o.status == SwapStatus::Pending && !o.is_expired())
            .collect()
    }

    pub fn find_match(
        &self,
        want_token: &str,
        offer_token: &str,
        max_rate: f64,
    ) -> Option<&SwapOrder> {
        self.pending().into_iter().find(|o| {
            o.offer_token == want_token
                && o.want_token == offer_token
                && o.exchange_rate() <= max_rate
        })
    }

    pub fn by_initiator(&self, initiator: &str) -> Vec<&SwapOrder> {
        self.orders
            .iter()
            .filter(|o| o.initiator == initiator)
            .collect()
    }

    pub fn prune_expired(&mut self) -> usize {
        let mut count = 0;
        for order in &mut self.orders {
            if order.is_expired() && order.status == SwapStatus::Pending {
                order.status = SwapStatus::Expired;
                count += 1;
            }
        }
        count
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn create_swap() {
        let swap = SwapOrder::new("alice", "ETH", 1000, "USDC", 2000, 24).unwrap();
        assert!(swap.id.starts_with("swap:"));
        assert_eq!(swap.status, SwapStatus::Pending);
        assert_eq!(swap.exchange_rate(), 2.0);
    }

    #[test]
    fn reject_zero_amounts() {
        assert!(SwapOrder::new("alice", "ETH", 0, "USDC", 1000, 24).is_err());
        assert!(SwapOrder::new("alice", "ETH", 1000, "USDC", 0, 24).is_err());
    }

    #[test]
    fn reject_same_token() {
        assert!(SwapOrder::new("alice", "ETH", 1000, "ETH", 2000, 24).is_err());
    }

    #[test]
    fn exchange_rates() {
        let swap = SwapOrder::new("alice", "ETH", 1000, "USDC", 3000, 24).unwrap();
        assert!((swap.exchange_rate() - 3.0).abs() < f64::EPSILON);
        assert!((swap.inverse_rate() - 1.0 / 3.0).abs() < 0.001);
    }

    #[test]
    fn accept_swap() {
        let mut swap = SwapOrder::new("alice", "ETH", 1000, "USDC", 2000, 24).unwrap();
        swap.accept("bob").unwrap();

        assert_eq!(swap.status, SwapStatus::Locked);
        assert_eq!(swap.counterparty.as_deref(), Some("bob"));
    }

    #[test]
    fn accept_own_swap_fails() {
        let mut swap = SwapOrder::new("alice", "ETH", 1000, "USDC", 2000, 24).unwrap();
        assert!(swap.accept("alice").is_err());
    }

    #[test]
    fn accept_twice_fails() {
        let mut swap = SwapOrder::new("alice", "ETH", 1000, "USDC", 2000, 24).unwrap();
        swap.accept("bob").unwrap();
        assert!(swap.accept("charlie").is_err());
    }

    #[test]
    fn complete_swap() {
        let mut swap = SwapOrder::new("alice", "ETH", 1000, "USDC", 2000, 24).unwrap();
        swap.accept("bob").unwrap();
        swap.complete("alice").unwrap();
        assert_eq!(swap.status, SwapStatus::Completed);
    }

    #[test]
    fn complete_by_counterparty() {
        let mut swap = SwapOrder::new("alice", "ETH", 1000, "USDC", 2000, 24).unwrap();
        swap.accept("bob").unwrap();
        swap.complete("bob").unwrap();
        assert_eq!(swap.status, SwapStatus::Completed);
    }

    #[test]
    fn complete_unauthorized_fails() {
        let mut swap = SwapOrder::new("alice", "ETH", 1000, "USDC", 2000, 24).unwrap();
        swap.accept("bob").unwrap();
        assert!(swap.complete("charlie").is_err());
    }

    #[test]
    fn refund_pending() {
        let mut swap = SwapOrder::new("alice", "ETH", 1000, "USDC", 2000, 24).unwrap();
        swap.refund("alice").unwrap();
        assert_eq!(swap.status, SwapStatus::Refunded);
    }

    #[test]
    fn refund_by_non_initiator_fails() {
        let mut swap = SwapOrder::new("alice", "ETH", 1000, "USDC", 2000, 24).unwrap();
        assert!(swap.refund("bob").is_err());
    }

    #[test]
    fn refund_locked_before_expiry_fails() {
        let mut swap = SwapOrder::new("alice", "ETH", 1000, "USDC", 2000, 24).unwrap();
        swap.accept("bob").unwrap();
        assert!(swap.refund("alice").is_err());
    }

    #[test]
    fn hashlock_preimage() {
        let preimage = b"secret_preimage_value".to_vec();
        let hashlock = sha256(&preimage);

        let mut swap = SwapOrder::new("alice", "ETH", 1000, "USDC", 2000, 24)
            .unwrap()
            .with_hashlock(hashlock);

        swap.reveal_preimage(preimage.clone()).unwrap();
        assert_eq!(swap.preimage, Some(preimage));
    }

    #[test]
    fn wrong_preimage_fails() {
        let hashlock = sha256(b"correct");
        let mut swap = SwapOrder::new("alice", "ETH", 1000, "USDC", 2000, 24)
            .unwrap()
            .with_hashlock(hashlock);

        assert!(swap.reveal_preimage(b"wrong".to_vec()).is_err());
    }

    #[test]
    fn swap_book_pending() {
        let mut book = SwapBook::new();
        book.add(SwapOrder::new("alice", "ETH", 1000, "USDC", 2000, 24).unwrap());
        book.add(SwapOrder::new("bob", "BTC", 100, "ETH", 1500, 24).unwrap());

        assert_eq!(book.pending().len(), 2);
    }

    #[test]
    fn swap_book_find_match() {
        let mut book = SwapBook::new();
        book.add(SwapOrder::new("alice", "ETH", 1000, "USDC", 2000, 24).unwrap());

        let found = book.find_match("ETH", "USDC", 3.0);
        assert!(found.is_some());

        let not_found = book.find_match("ETH", "USDC", 1.0);
        assert!(not_found.is_none());
    }

    #[test]
    fn swap_book_by_initiator() {
        let mut book = SwapBook::new();
        book.add(SwapOrder::new("alice", "ETH", 1000, "USDC", 2000, 24).unwrap());
        book.add(SwapOrder::new("alice", "BTC", 100, "ETH", 1500, 24).unwrap());
        book.add(SwapOrder::new("bob", "SOL", 500, "USDC", 100, 24).unwrap());

        assert_eq!(book.by_initiator("alice").len(), 2);
        assert_eq!(book.by_initiator("bob").len(), 1);
    }

    #[test]
    fn swap_serializes() {
        let swap = SwapOrder::new("alice", "ETH", 1000, "USDC", 2000, 24).unwrap();
        let json = serde_json::to_string(&swap).unwrap();
        let restored: SwapOrder = serde_json::from_str(&json).unwrap();
        assert_eq!(restored.initiator, "alice");
        assert_eq!(restored.exchange_rate(), 2.0);
    }
}
