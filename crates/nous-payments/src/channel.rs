use chrono::{DateTime, Duration, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use nous_core::{Error, Result};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ChannelState {
    Opening,
    Open,
    Closing,
    Closed,
    Disputed,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StateUpdate {
    pub sequence: u64,
    pub balance_a: u128,
    pub balance_b: u128,
    pub timestamp: DateTime<Utc>,
    pub signature_a: Vec<u8>,
    pub signature_b: Vec<u8>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PaymentChannel {
    pub id: String,
    pub party_a: String,
    pub party_b: String,
    pub token: String,
    pub deposit_a: u128,
    pub deposit_b: u128,
    pub state: ChannelState,
    pub sequence: u64,
    pub balance_a: u128,
    pub balance_b: u128,
    pub created_at: DateTime<Utc>,
    pub timeout_hours: i64,
    pub updates: Vec<StateUpdate>,
}

impl PaymentChannel {
    pub fn open(
        party_a: &str,
        party_b: &str,
        token: &str,
        deposit_a: u128,
        deposit_b: u128,
    ) -> Result<Self> {
        if party_a == party_b {
            return Err(Error::InvalidInput(
                "channel parties must be different".into(),
            ));
        }
        if deposit_a == 0 && deposit_b == 0 {
            return Err(Error::InvalidInput(
                "at least one party must deposit".into(),
            ));
        }

        Ok(Self {
            id: format!("chan:{}", Uuid::new_v4()),
            party_a: party_a.into(),
            party_b: party_b.into(),
            token: token.into(),
            deposit_a,
            deposit_b,
            state: ChannelState::Opening,
            sequence: 0,
            balance_a: deposit_a,
            balance_b: deposit_b,
            created_at: Utc::now(),
            timeout_hours: 24,
            updates: Vec::new(),
        })
    }

    pub fn with_timeout(mut self, hours: i64) -> Self {
        self.timeout_hours = hours;
        self
    }

    pub fn confirm_open(&mut self) -> Result<()> {
        if self.state != ChannelState::Opening {
            return Err(Error::InvalidInput(
                "channel is not in opening state".into(),
            ));
        }
        self.state = ChannelState::Open;
        Ok(())
    }

    pub fn total_capacity(&self) -> u128 {
        self.deposit_a + self.deposit_b
    }

    pub fn transfer(&mut self, from: &str, amount: u128) -> Result<StateUpdate> {
        if self.state != ChannelState::Open {
            return Err(Error::InvalidInput("channel is not open".into()));
        }
        if amount == 0 {
            return Err(Error::InvalidInput("amount must be positive".into()));
        }

        let (new_a, new_b) = if from == self.party_a {
            if self.balance_a < amount {
                return Err(Error::InvalidInput(
                    "insufficient balance in channel".into(),
                ));
            }
            (self.balance_a - amount, self.balance_b + amount)
        } else if from == self.party_b {
            if self.balance_b < amount {
                return Err(Error::InvalidInput(
                    "insufficient balance in channel".into(),
                ));
            }
            (self.balance_a + amount, self.balance_b - amount)
        } else {
            return Err(Error::PermissionDenied(
                "caller is not a channel party".into(),
            ));
        };

        self.sequence += 1;
        self.balance_a = new_a;
        self.balance_b = new_b;

        let update = StateUpdate {
            sequence: self.sequence,
            balance_a: new_a,
            balance_b: new_b,
            timestamp: Utc::now(),
            signature_a: Vec::new(),
            signature_b: Vec::new(),
        };
        self.updates.push(update.clone());
        Ok(update)
    }

    pub fn initiate_close(&mut self, caller: &str) -> Result<()> {
        if self.state != ChannelState::Open {
            return Err(Error::InvalidInput("channel is not open".into()));
        }
        if caller != self.party_a && caller != self.party_b {
            return Err(Error::PermissionDenied(
                "only channel parties can close".into(),
            ));
        }
        self.state = ChannelState::Closing;
        Ok(())
    }

    pub fn finalize_close(&mut self) -> Result<(u128, u128)> {
        if self.state != ChannelState::Closing {
            return Err(Error::InvalidInput(
                "channel is not in closing state".into(),
            ));
        }
        self.state = ChannelState::Closed;
        Ok((self.balance_a, self.balance_b))
    }

    pub fn dispute(&mut self, caller: &str) -> Result<()> {
        if self.state != ChannelState::Closing {
            return Err(Error::InvalidInput(
                "can only dispute during closing period".into(),
            ));
        }
        if caller != self.party_a && caller != self.party_b {
            return Err(Error::PermissionDenied(
                "only channel parties can dispute".into(),
            ));
        }
        self.state = ChannelState::Disputed;
        Ok(())
    }

    pub fn resolve_dispute(&mut self, final_a: u128, final_b: u128) -> Result<()> {
        if self.state != ChannelState::Disputed {
            return Err(Error::InvalidInput("channel is not disputed".into()));
        }
        if final_a + final_b != self.total_capacity() {
            return Err(Error::InvalidInput(
                "resolved balances must equal total capacity".into(),
            ));
        }
        self.balance_a = final_a;
        self.balance_b = final_b;
        self.state = ChannelState::Closed;
        Ok(())
    }

    pub fn is_timed_out(&self) -> bool {
        Utc::now() > self.created_at + Duration::hours(self.timeout_hours)
    }

    pub fn latest_update(&self) -> Option<&StateUpdate> {
        self.updates.last()
    }

    pub fn update_count(&self) -> usize {
        self.updates.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_channel() -> PaymentChannel {
        let mut ch = PaymentChannel::open("alice", "bob", "ETH", 1000, 500).unwrap();
        ch.confirm_open().unwrap();
        ch
    }

    #[test]
    fn open_channel() {
        let ch = PaymentChannel::open("alice", "bob", "ETH", 1000, 500).unwrap();
        assert_eq!(ch.state, ChannelState::Opening);
        assert_eq!(ch.total_capacity(), 1500);
        assert!(ch.id.starts_with("chan:"));
    }

    #[test]
    fn reject_same_parties() {
        assert!(PaymentChannel::open("alice", "alice", "ETH", 1000, 0).is_err());
    }

    #[test]
    fn reject_zero_deposits() {
        assert!(PaymentChannel::open("alice", "bob", "ETH", 0, 0).is_err());
    }

    #[test]
    fn confirm_open() {
        let mut ch = PaymentChannel::open("alice", "bob", "ETH", 1000, 0).unwrap();
        assert!(ch.confirm_open().is_ok());
        assert_eq!(ch.state, ChannelState::Open);
    }

    #[test]
    fn double_confirm_fails() {
        let mut ch = test_channel();
        assert!(ch.confirm_open().is_err());
    }

    #[test]
    fn transfer_a_to_b() {
        let mut ch = test_channel();
        let update = ch.transfer("alice", 300).unwrap();

        assert_eq!(ch.balance_a, 700);
        assert_eq!(ch.balance_b, 800);
        assert_eq!(update.sequence, 1);
        assert_eq!(ch.update_count(), 1);
    }

    #[test]
    fn transfer_b_to_a() {
        let mut ch = test_channel();
        ch.transfer("bob", 200).unwrap();

        assert_eq!(ch.balance_a, 1200);
        assert_eq!(ch.balance_b, 300);
    }

    #[test]
    fn transfer_insufficient_balance() {
        let mut ch = test_channel();
        assert!(ch.transfer("alice", 2000).is_err());
        assert_eq!(ch.balance_a, 1000); // unchanged
    }

    #[test]
    fn transfer_zero_rejected() {
        let mut ch = test_channel();
        assert!(ch.transfer("alice", 0).is_err());
    }

    #[test]
    fn transfer_unauthorized() {
        let mut ch = test_channel();
        assert!(ch.transfer("charlie", 100).is_err());
    }

    #[test]
    fn multiple_transfers() {
        let mut ch = test_channel();
        ch.transfer("alice", 100).unwrap();
        ch.transfer("bob", 50).unwrap();
        ch.transfer("alice", 200).unwrap();

        assert_eq!(ch.sequence, 3);
        assert_eq!(ch.balance_a, 750);
        assert_eq!(ch.balance_b, 750);
        assert_eq!(ch.total_capacity(), 1500);
    }

    #[test]
    fn close_channel() {
        let mut ch = test_channel();
        ch.transfer("alice", 300).unwrap();
        ch.initiate_close("alice").unwrap();

        assert_eq!(ch.state, ChannelState::Closing);

        let (final_a, final_b) = ch.finalize_close().unwrap();
        assert_eq!(final_a, 700);
        assert_eq!(final_b, 800);
        assert_eq!(ch.state, ChannelState::Closed);
    }

    #[test]
    fn close_unauthorized() {
        let mut ch = test_channel();
        assert!(ch.initiate_close("charlie").is_err());
    }

    #[test]
    fn transfer_on_closed_channel_fails() {
        let mut ch = test_channel();
        ch.initiate_close("alice").unwrap();
        ch.finalize_close().unwrap();
        assert!(ch.transfer("alice", 100).is_err());
    }

    #[test]
    fn dispute_during_closing() {
        let mut ch = test_channel();
        ch.initiate_close("alice").unwrap();
        ch.dispute("bob").unwrap();
        assert_eq!(ch.state, ChannelState::Disputed);
    }

    #[test]
    fn dispute_not_closing_fails() {
        let mut ch = test_channel();
        assert!(ch.dispute("alice").is_err());
    }

    #[test]
    fn resolve_dispute() {
        let mut ch = test_channel();
        ch.initiate_close("alice").unwrap();
        ch.dispute("bob").unwrap();
        ch.resolve_dispute(800, 700).unwrap();

        assert_eq!(ch.state, ChannelState::Closed);
        assert_eq!(ch.balance_a, 800);
        assert_eq!(ch.balance_b, 700);
    }

    #[test]
    fn resolve_dispute_wrong_total_fails() {
        let mut ch = test_channel();
        ch.initiate_close("alice").unwrap();
        ch.dispute("bob").unwrap();
        assert!(ch.resolve_dispute(1000, 1000).is_err());
    }

    #[test]
    fn latest_update() {
        let mut ch = test_channel();
        assert!(ch.latest_update().is_none());

        ch.transfer("alice", 100).unwrap();
        let latest = ch.latest_update().unwrap();
        assert_eq!(latest.sequence, 1);
    }

    #[test]
    fn with_timeout() {
        let ch = PaymentChannel::open("alice", "bob", "ETH", 1000, 0)
            .unwrap()
            .with_timeout(48);
        assert_eq!(ch.timeout_hours, 48);
    }

    #[test]
    fn channel_serializes() {
        let ch = test_channel();
        let json = serde_json::to_string(&ch).unwrap();
        let restored: PaymentChannel = serde_json::from_str(&json).unwrap();
        assert_eq!(restored.party_a, "alice");
        assert_eq!(restored.total_capacity(), 1500);
    }
}
