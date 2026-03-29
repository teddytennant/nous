use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

use nous_core::Error;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Wallet {
    pub did: String,
    pub balances: HashMap<String, u128>,
    pub created_at: DateTime<Utc>,
    pub nonce: u64,
}

impl Wallet {
    pub fn new(did: impl Into<String>) -> Self {
        Self {
            did: did.into(),
            balances: HashMap::new(),
            created_at: Utc::now(),
            nonce: 0,
        }
    }

    pub fn balance(&self, token: &str) -> u128 {
        self.balances.get(token).copied().unwrap_or(0)
    }

    pub fn tokens(&self) -> Vec<&str> {
        self.balances.keys().map(|s| s.as_str()).collect()
    }

    pub fn credit(&mut self, token: &str, amount: u128) {
        *self.balances.entry(token.to_string()).or_insert(0) += amount;
    }

    pub fn debit(&mut self, token: &str, amount: u128) -> Result<(), Error> {
        let balance = self.balances.entry(token.to_string()).or_insert(0);
        if *balance < amount {
            return Err(Error::InvalidInput(format!(
                "insufficient {token} balance: have {balance}, need {amount}"
            )));
        }
        *balance -= amount;
        self.nonce += 1;
        Ok(())
    }

    pub fn total_value_usd(&self, prices: &HashMap<String, f64>) -> f64 {
        self.balances
            .iter()
            .map(|(token, &amount)| {
                let price = prices.get(token).copied().unwrap_or(0.0);
                (amount as f64) * price
            })
            .sum()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Transaction {
    pub id: String,
    pub from_did: String,
    pub to_did: String,
    pub token: String,
    pub amount: u128,
    pub fee: u128,
    pub memo: Option<String>,
    pub timestamp: DateTime<Utc>,
    pub status: TxStatus,
    pub signature: Vec<u8>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TxStatus {
    Pending,
    Confirmed,
    Failed,
    Cancelled,
}

impl Transaction {
    pub fn new(from: &str, to: &str, token: &str, amount: u128) -> Self {
        Self {
            id: format!("tx:{}", Uuid::new_v4()),
            from_did: from.to_string(),
            to_did: to.to_string(),
            token: token.to_string(),
            amount,
            fee: 0,
            memo: None,
            timestamp: Utc::now(),
            status: TxStatus::Pending,
            signature: Vec::new(),
        }
    }

    pub fn with_memo(mut self, memo: impl Into<String>) -> Self {
        self.memo = Some(memo.into());
        self
    }

    pub fn with_fee(mut self, fee: u128) -> Self {
        self.fee = fee;
        self
    }

    pub fn signable_bytes(&self) -> Vec<u8> {
        let mut buf = Vec::new();
        buf.extend_from_slice(self.from_did.as_bytes());
        buf.extend_from_slice(self.to_did.as_bytes());
        buf.extend_from_slice(self.token.as_bytes());
        buf.extend_from_slice(&self.amount.to_be_bytes());
        buf.extend_from_slice(&self.fee.to_be_bytes());
        buf.extend_from_slice(&self.timestamp.timestamp().to_be_bytes());
        buf
    }

    pub fn sign(&mut self, keypair: &nous_crypto::KeyPair) {
        let signer = nous_crypto::Signer::new(keypair);
        let sig = signer.sign(&self.signable_bytes());
        self.signature = sig.as_bytes().to_vec();
    }

    pub fn confirm(&mut self) {
        self.status = TxStatus::Confirmed;
    }

    pub fn fail(&mut self) {
        self.status = TxStatus::Failed;
    }

    pub fn total_cost(&self) -> u128 {
        self.amount + self.fee
    }
}

pub fn transfer(
    sender: &mut Wallet,
    receiver: &mut Wallet,
    token: &str,
    amount: u128,
) -> Result<Transaction, Error> {
    if amount == 0 {
        return Err(Error::InvalidInput("amount must be positive".into()));
    }
    if sender.did == receiver.did {
        return Err(Error::InvalidInput("cannot transfer to self".into()));
    }

    sender.debit(token, amount)?;
    receiver.credit(token, amount);

    let mut tx = Transaction::new(&sender.did, &receiver.did, token, amount);
    tx.confirm();
    Ok(tx)
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
        let err = wallet.debit("ETH", 200).unwrap_err();
        assert!(err.to_string().contains("insufficient"));
    }

    #[test]
    fn wallet_zero_balance_unknown_token() {
        let wallet = Wallet::new("did:key:ztest");
        assert_eq!(wallet.balance("BTC"), 0);
    }

    #[test]
    fn wallet_nonce_increments() {
        let mut wallet = Wallet::new("did:key:ztest");
        wallet.credit("ETH", 1000);
        assert_eq!(wallet.nonce, 0);
        wallet.debit("ETH", 100).unwrap();
        assert_eq!(wallet.nonce, 1);
        wallet.debit("ETH", 100).unwrap();
        assert_eq!(wallet.nonce, 2);
    }

    #[test]
    fn wallet_tokens() {
        let mut wallet = Wallet::new("did:key:ztest");
        wallet.credit("ETH", 100);
        wallet.credit("BTC", 200);
        let tokens = wallet.tokens();
        assert!(tokens.contains(&"ETH"));
        assert!(tokens.contains(&"BTC"));
    }

    #[test]
    fn wallet_total_value() {
        let mut wallet = Wallet::new("did:key:ztest");
        wallet.credit("ETH", 10);
        wallet.credit("BTC", 5);

        let mut prices = HashMap::new();
        prices.insert("ETH".to_string(), 2000.0);
        prices.insert("BTC".to_string(), 50000.0);

        let total = wallet.total_value_usd(&prices);
        assert!((total - 270000.0).abs() < 0.01);
    }

    #[test]
    fn transaction_with_memo() {
        let tx = Transaction::new("alice", "bob", "ETH", 100).with_memo("for coffee");
        assert_eq!(tx.memo.as_deref(), Some("for coffee"));
    }

    #[test]
    fn transaction_with_fee() {
        let tx = Transaction::new("alice", "bob", "ETH", 100).with_fee(5);
        assert_eq!(tx.total_cost(), 105);
    }

    #[test]
    fn transaction_sign() {
        let kp = nous_crypto::KeyPair::generate();
        let mut tx = Transaction::new("alice", "bob", "ETH", 100);
        tx.sign(&kp);
        assert!(!tx.signature.is_empty());
    }

    #[test]
    fn transaction_signable_bytes_deterministic() {
        let tx = Transaction {
            id: "tx:1".into(),
            from_did: "alice".into(),
            to_did: "bob".into(),
            token: "ETH".into(),
            amount: 100,
            fee: 5,
            memo: None,
            timestamp: DateTime::from_timestamp(1000, 0).unwrap(),
            status: TxStatus::Pending,
            signature: vec![],
        };
        assert_eq!(tx.signable_bytes(), tx.signable_bytes());
    }

    #[test]
    fn transfer_success() {
        let mut alice = Wallet::new("alice");
        let mut bob = Wallet::new("bob");
        alice.credit("ETH", 1000);

        let tx = transfer(&mut alice, &mut bob, "ETH", 300).unwrap();
        assert_eq!(alice.balance("ETH"), 700);
        assert_eq!(bob.balance("ETH"), 300);
        assert_eq!(tx.status, TxStatus::Confirmed);
    }

    #[test]
    fn transfer_insufficient_funds() {
        let mut alice = Wallet::new("alice");
        let mut bob = Wallet::new("bob");
        alice.credit("ETH", 100);

        assert!(transfer(&mut alice, &mut bob, "ETH", 200).is_err());
        assert_eq!(alice.balance("ETH"), 100); // unchanged
    }

    #[test]
    fn transfer_zero_amount_rejected() {
        let mut alice = Wallet::new("alice");
        let mut bob = Wallet::new("bob");
        alice.credit("ETH", 1000);
        assert!(transfer(&mut alice, &mut bob, "ETH", 0).is_err());
    }

    #[test]
    fn transfer_to_self_rejected() {
        let mut wallet = Wallet::new("alice");
        let mut same = Wallet::new("alice");
        wallet.credit("ETH", 1000);
        assert!(transfer(&mut wallet, &mut same, "ETH", 100).is_err());
    }

    #[test]
    fn transaction_serializes() {
        let tx = Transaction::new("alice", "bob", "ETH", 100);
        let json = serde_json::to_string(&tx).unwrap();
        let _: Transaction = serde_json::from_str(&json).unwrap();
    }
}
