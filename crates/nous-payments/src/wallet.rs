use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Wallet {
    pub did: String,
    pub balances: std::collections::HashMap<String, u128>,
}

impl Wallet {
    pub fn new(did: impl Into<String>) -> Self {
        Self {
            did: did.into(),
            balances: std::collections::HashMap::new(),
        }
    }

    pub fn balance(&self, token: &str) -> u128 {
        self.balances.get(token).copied().unwrap_or(0)
    }

    pub fn credit(&mut self, token: &str, amount: u128) {
        *self.balances.entry(token.to_string()).or_insert(0) += amount;
    }

    pub fn debit(&mut self, token: &str, amount: u128) -> Result<(), nous_core::Error> {
        let balance = self.balances.entry(token.to_string()).or_insert(0);
        if *balance < amount {
            return Err(nous_core::Error::InvalidInput(
                "insufficient balance".into(),
            ));
        }
        *balance -= amount;
        Ok(())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Transaction {
    pub id: String,
    pub from_did: String,
    pub to_did: String,
    pub token: String,
    pub amount: u128,
    pub timestamp: DateTime<Utc>,
}

impl Transaction {
    pub fn new(from: &str, to: &str, token: &str, amount: u128) -> Self {
        Self {
            id: format!("tx:{}", Uuid::new_v4()),
            from_did: from.to_string(),
            to_did: to.to_string(),
            token: token.to_string(),
            amount,
            timestamp: Utc::now(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wallet_credit_debit() {
        let mut wallet = Wallet::new("did:key:ztest");
        wallet.credit("ETH", 1000);
        assert_eq!(wallet.balance("ETH"), 1000);

        wallet.debit("ETH", 300).unwrap();
        assert_eq!(wallet.balance("ETH"), 700);
    }

    #[test]
    fn wallet_insufficient_balance() {
        let mut wallet = Wallet::new("did:key:ztest");
        wallet.credit("ETH", 100);
        assert!(wallet.debit("ETH", 200).is_err());
    }

    #[test]
    fn wallet_zero_balance_unknown_token() {
        let wallet = Wallet::new("did:key:ztest");
        assert_eq!(wallet.balance("BTC"), 0);
    }
}
