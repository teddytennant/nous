use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::wallet::{Transaction, TxStatus};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TxDirection {
    Sent,
    Received,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TxRecord {
    pub tx: Transaction,
    pub direction: TxDirection,
    pub counterparty: String,
    pub note: Option<String>,
}

#[derive(Debug, Default)]
pub struct TxHistory {
    records: Vec<TxRecord>,
}

impl TxHistory {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn record_sent(&mut self, tx: Transaction) {
        let counterparty = tx.to_did.clone();
        self.records.push(TxRecord {
            tx,
            direction: TxDirection::Sent,
            counterparty,
            note: None,
        });
    }

    pub fn record_received(&mut self, tx: Transaction) {
        let counterparty = tx.from_did.clone();
        self.records.push(TxRecord {
            tx,
            direction: TxDirection::Received,
            counterparty,
            note: None,
        });
    }

    pub fn len(&self) -> usize {
        self.records.len()
    }

    pub fn is_empty(&self) -> bool {
        self.records.is_empty()
    }

    pub fn all(&self) -> &[TxRecord] {
        &self.records
    }

    pub fn recent(&self, count: usize) -> Vec<&TxRecord> {
        self.records.iter().rev().take(count).collect()
    }

    pub fn by_token(&self, token: &str) -> Vec<&TxRecord> {
        self.records
            .iter()
            .filter(|r| r.tx.token == token)
            .collect()
    }

    pub fn by_status(&self, status: TxStatus) -> Vec<&TxRecord> {
        self.records
            .iter()
            .filter(|r| r.tx.status == status)
            .collect()
    }

    pub fn by_direction(&self, direction: TxDirection) -> Vec<&TxRecord> {
        self.records
            .iter()
            .filter(|r| r.direction == direction)
            .collect()
    }

    pub fn by_counterparty(&self, counterparty: &str) -> Vec<&TxRecord> {
        self.records
            .iter()
            .filter(|r| r.counterparty == counterparty)
            .collect()
    }

    pub fn since(&self, since: DateTime<Utc>) -> Vec<&TxRecord> {
        self.records
            .iter()
            .filter(|r| r.tx.timestamp >= since)
            .collect()
    }

    pub fn total_sent(&self, token: &str) -> u128 {
        self.records
            .iter()
            .filter(|r| r.direction == TxDirection::Sent && r.tx.token == token)
            .filter(|r| r.tx.status == TxStatus::Confirmed)
            .map(|r| r.tx.amount)
            .sum()
    }

    pub fn total_received(&self, token: &str) -> u128 {
        self.records
            .iter()
            .filter(|r| r.direction == TxDirection::Received && r.tx.token == token)
            .filter(|r| r.tx.status == TxStatus::Confirmed)
            .map(|r| r.tx.amount)
            .sum()
    }

    pub fn total_fees(&self, token: &str) -> u128 {
        self.records
            .iter()
            .filter(|r| r.direction == TxDirection::Sent && r.tx.token == token)
            .filter(|r| r.tx.status == TxStatus::Confirmed)
            .map(|r| r.tx.fee)
            .sum()
    }

    pub fn net_flow(&self, token: &str) -> i128 {
        self.total_received(token) as i128 - self.total_sent(token) as i128
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_tx(from: &str, to: &str, token: &str, amount: u128) -> Transaction {
        let mut tx = Transaction::new(from, to, token, amount);
        tx.confirm();
        tx
    }

    #[test]
    fn record_sent() {
        let mut history = TxHistory::new();
        history.record_sent(make_tx("alice", "bob", "ETH", 100));

        assert_eq!(history.len(), 1);
        assert_eq!(history.all()[0].direction, TxDirection::Sent);
        assert_eq!(history.all()[0].counterparty, "bob");
    }

    #[test]
    fn record_received() {
        let mut history = TxHistory::new();
        history.record_received(make_tx("bob", "alice", "ETH", 200));

        assert_eq!(history.all()[0].direction, TxDirection::Received);
        assert_eq!(history.all()[0].counterparty, "bob");
    }

    #[test]
    fn recent() {
        let mut history = TxHistory::new();
        history.record_sent(make_tx("alice", "bob", "ETH", 100));
        history.record_sent(make_tx("alice", "charlie", "ETH", 200));
        history.record_received(make_tx("dave", "alice", "ETH", 300));

        let recent = history.recent(2);
        assert_eq!(recent.len(), 2);
        assert_eq!(recent[0].tx.amount, 300);
    }

    #[test]
    fn by_token() {
        let mut history = TxHistory::new();
        history.record_sent(make_tx("alice", "bob", "ETH", 100));
        history.record_sent(make_tx("alice", "bob", "USDC", 200));
        history.record_sent(make_tx("alice", "bob", "ETH", 300));

        assert_eq!(history.by_token("ETH").len(), 2);
        assert_eq!(history.by_token("USDC").len(), 1);
        assert_eq!(history.by_token("BTC").len(), 0);
    }

    #[test]
    fn by_direction() {
        let mut history = TxHistory::new();
        history.record_sent(make_tx("alice", "bob", "ETH", 100));
        history.record_received(make_tx("bob", "alice", "ETH", 200));
        history.record_sent(make_tx("alice", "charlie", "ETH", 300));

        assert_eq!(history.by_direction(TxDirection::Sent).len(), 2);
        assert_eq!(history.by_direction(TxDirection::Received).len(), 1);
    }

    #[test]
    fn by_counterparty() {
        let mut history = TxHistory::new();
        history.record_sent(make_tx("alice", "bob", "ETH", 100));
        history.record_sent(make_tx("alice", "bob", "ETH", 200));
        history.record_sent(make_tx("alice", "charlie", "ETH", 300));

        assert_eq!(history.by_counterparty("bob").len(), 2);
        assert_eq!(history.by_counterparty("charlie").len(), 1);
    }

    #[test]
    fn total_sent() {
        let mut history = TxHistory::new();
        history.record_sent(make_tx("alice", "bob", "ETH", 100));
        history.record_sent(make_tx("alice", "charlie", "ETH", 200));
        history.record_received(make_tx("dave", "alice", "ETH", 500));

        assert_eq!(history.total_sent("ETH"), 300);
    }

    #[test]
    fn total_received() {
        let mut history = TxHistory::new();
        history.record_sent(make_tx("alice", "bob", "ETH", 100));
        history.record_received(make_tx("bob", "alice", "ETH", 200));
        history.record_received(make_tx("charlie", "alice", "ETH", 300));

        assert_eq!(history.total_received("ETH"), 500);
    }

    #[test]
    fn total_fees() {
        let mut history = TxHistory::new();
        let mut tx = make_tx("alice", "bob", "ETH", 100);
        tx.fee = 5;
        history.record_sent(tx);

        let mut tx2 = make_tx("alice", "charlie", "ETH", 200);
        tx2.fee = 10;
        history.record_sent(tx2);

        assert_eq!(history.total_fees("ETH"), 15);
    }

    #[test]
    fn net_flow() {
        let mut history = TxHistory::new();
        history.record_sent(make_tx("alice", "bob", "ETH", 100));
        history.record_received(make_tx("charlie", "alice", "ETH", 300));

        assert_eq!(history.net_flow("ETH"), 200);
    }

    #[test]
    fn net_flow_negative() {
        let mut history = TxHistory::new();
        history.record_sent(make_tx("alice", "bob", "ETH", 500));
        history.record_received(make_tx("charlie", "alice", "ETH", 100));

        assert_eq!(history.net_flow("ETH"), -400);
    }

    #[test]
    fn empty_history() {
        let history = TxHistory::new();
        assert!(history.is_empty());
        assert_eq!(history.len(), 0);
        assert_eq!(history.total_sent("ETH"), 0);
        assert_eq!(history.total_received("ETH"), 0);
        assert_eq!(history.net_flow("ETH"), 0);
    }

    #[test]
    fn pending_tx_not_counted_in_totals() {
        let mut history = TxHistory::new();
        let tx = Transaction::new("alice", "bob", "ETH", 1000); // pending, not confirmed
        history.record_sent(tx);

        assert_eq!(history.total_sent("ETH"), 0); // pending not counted
    }
}
