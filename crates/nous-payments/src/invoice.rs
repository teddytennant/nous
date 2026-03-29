use chrono::{DateTime, Duration, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use nous_core::Error;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum InvoiceStatus {
    Pending,
    Paid,
    Expired,
    Cancelled,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LineItem {
    pub description: String,
    pub quantity: u32,
    pub unit_price: u128,
}

impl LineItem {
    pub fn new(description: &str, quantity: u32, unit_price: u128) -> Self {
        Self {
            description: description.to_string(),
            quantity,
            unit_price,
        }
    }

    pub fn total(&self) -> u128 {
        self.unit_price * self.quantity as u128
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Invoice {
    pub id: String,
    pub from_did: String,
    pub to_did: String,
    pub token: String,
    pub items: Vec<LineItem>,
    pub memo: Option<String>,
    pub status: InvoiceStatus,
    pub created_at: DateTime<Utc>,
    pub due_at: DateTime<Utc>,
    pub paid_at: Option<DateTime<Utc>>,
}

impl Invoice {
    pub fn new(from: &str, to: &str, token: &str, days_until_due: i64) -> Self {
        Self {
            id: format!("inv:{}", Uuid::new_v4()),
            from_did: from.to_string(),
            to_did: to.to_string(),
            token: token.to_string(),
            items: Vec::new(),
            memo: None,
            status: InvoiceStatus::Pending,
            created_at: Utc::now(),
            due_at: Utc::now() + Duration::days(days_until_due),
            paid_at: None,
        }
    }

    pub fn add_item(&mut self, item: LineItem) {
        self.items.push(item);
    }

    pub fn with_memo(mut self, memo: impl Into<String>) -> Self {
        self.memo = Some(memo.into());
        self
    }

    pub fn total(&self) -> u128 {
        self.items.iter().map(|i| i.total()).sum()
    }

    pub fn is_overdue(&self) -> bool {
        self.status == InvoiceStatus::Pending && Utc::now() > self.due_at
    }

    pub fn mark_paid(&mut self) -> Result<(), Error> {
        if self.status != InvoiceStatus::Pending {
            return Err(Error::InvalidInput(format!(
                "invoice is {:?}, not pending",
                self.status
            )));
        }
        self.status = InvoiceStatus::Paid;
        self.paid_at = Some(Utc::now());
        Ok(())
    }

    pub fn cancel(&mut self) -> Result<(), Error> {
        if self.status != InvoiceStatus::Pending {
            return Err(Error::InvalidInput("can only cancel pending invoices".into()));
        }
        self.status = InvoiceStatus::Cancelled;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn create_invoice() {
        let inv = Invoice::new("alice", "bob", "ETH", 30);
        assert_eq!(inv.status, InvoiceStatus::Pending);
        assert!(inv.id.starts_with("inv:"));
    }

    #[test]
    fn line_items() {
        let mut inv = Invoice::new("alice", "bob", "ETH", 30);
        inv.add_item(LineItem::new("Widget", 3, 100));
        inv.add_item(LineItem::new("Service", 1, 500));
        assert_eq!(inv.total(), 800);
    }

    #[test]
    fn line_item_total() {
        let item = LineItem::new("test", 5, 200);
        assert_eq!(item.total(), 1000);
    }

    #[test]
    fn mark_paid() {
        let mut inv = Invoice::new("alice", "bob", "ETH", 30);
        assert!(inv.mark_paid().is_ok());
        assert_eq!(inv.status, InvoiceStatus::Paid);
        assert!(inv.paid_at.is_some());
    }

    #[test]
    fn cannot_pay_twice() {
        let mut inv = Invoice::new("alice", "bob", "ETH", 30);
        inv.mark_paid().unwrap();
        assert!(inv.mark_paid().is_err());
    }

    #[test]
    fn cancel_invoice() {
        let mut inv = Invoice::new("alice", "bob", "ETH", 30);
        assert!(inv.cancel().is_ok());
        assert_eq!(inv.status, InvoiceStatus::Cancelled);
    }

    #[test]
    fn cannot_cancel_paid() {
        let mut inv = Invoice::new("alice", "bob", "ETH", 30);
        inv.mark_paid().unwrap();
        assert!(inv.cancel().is_err());
    }

    #[test]
    fn not_overdue_within_duration() {
        let inv = Invoice::new("alice", "bob", "ETH", 30);
        assert!(!inv.is_overdue());
    }

    #[test]
    fn with_memo() {
        let inv = Invoice::new("alice", "bob", "ETH", 30)
            .with_memo("payment for services");
        assert_eq!(inv.memo.as_deref(), Some("payment for services"));
    }

    #[test]
    fn invoice_serializes() {
        let mut inv = Invoice::new("alice", "bob", "ETH", 30);
        inv.add_item(LineItem::new("Widget", 2, 100));
        let json = serde_json::to_string(&inv).unwrap();
        let restored: Invoice = serde_json::from_str(&json).unwrap();
        assert_eq!(restored.total(), 200);
    }
}
