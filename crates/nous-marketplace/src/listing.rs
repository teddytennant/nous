use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ListingStatus {
    Active,
    Sold,
    Cancelled,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Listing {
    pub id: String,
    pub seller_did: String,
    pub title: String,
    pub description: String,
    pub price_token: String,
    pub price_amount: u128,
    pub status: ListingStatus,
    pub created_at: DateTime<Utc>,
}

impl Listing {
    pub fn new(
        seller_did: impl Into<String>,
        title: impl Into<String>,
        description: impl Into<String>,
        price_token: impl Into<String>,
        price_amount: u128,
    ) -> Self {
        Self {
            id: format!("listing:{}", Uuid::new_v4()),
            seller_did: seller_did.into(),
            title: title.into(),
            description: description.into(),
            price_token: price_token.into(),
            price_amount,
            status: ListingStatus::Active,
            created_at: Utc::now(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn create_listing() {
        let listing = Listing::new("did:key:ztest", "Widget", "A fine widget", "ETH", 100);
        assert_eq!(listing.status, ListingStatus::Active);
        assert!(listing.id.starts_with("listing:"));
    }
}
