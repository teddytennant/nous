use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use nous_core::Error;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ListingStatus {
    Active,
    Sold,
    Cancelled,
    Expired,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ListingCategory {
    Physical,
    Digital,
    Service,
    NFT,
    Data,
    Other,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Listing {
    pub id: String,
    pub seller_did: String,
    pub title: String,
    pub description: String,
    pub category: ListingCategory,
    pub price_token: String,
    pub price_amount: u128,
    pub quantity: u32,
    pub status: ListingStatus,
    pub created_at: DateTime<Utc>,
    pub tags: Vec<String>,
    pub images: Vec<String>,
    pub min_reputation: Option<f64>,
}

impl Listing {
    pub fn new(
        seller_did: impl Into<String>,
        title: impl Into<String>,
        description: impl Into<String>,
        category: ListingCategory,
        price_token: impl Into<String>,
        price_amount: u128,
    ) -> Result<Self, Error> {
        let title = title.into();
        if title.is_empty() {
            return Err(Error::InvalidInput("title cannot be empty".into()));
        }
        if price_amount == 0 {
            return Err(Error::InvalidInput("price must be positive".into()));
        }

        Ok(Self {
            id: format!("listing:{}", Uuid::new_v4()),
            seller_did: seller_did.into(),
            title,
            description: description.into(),
            category,
            price_token: price_token.into(),
            price_amount,
            quantity: 1,
            status: ListingStatus::Active,
            created_at: Utc::now(),
            tags: Vec::new(),
            images: Vec::new(),
            min_reputation: None,
        })
    }

    pub fn with_quantity(mut self, qty: u32) -> Self {
        self.quantity = qty;
        self
    }

    pub fn with_tag(mut self, tag: impl Into<String>) -> Self {
        self.tags.push(tag.into());
        self
    }

    pub fn with_image(mut self, cid: impl Into<String>) -> Self {
        self.images.push(cid.into());
        self
    }

    pub fn with_min_reputation(mut self, min: f64) -> Self {
        self.min_reputation = Some(min);
        self
    }

    pub fn is_available(&self) -> bool {
        self.status == ListingStatus::Active && self.quantity > 0
    }

    pub fn purchase(&mut self) -> Result<(), Error> {
        if !self.is_available() {
            return Err(Error::InvalidInput("listing not available".into()));
        }
        self.quantity -= 1;
        if self.quantity == 0 {
            self.status = ListingStatus::Sold;
        }
        Ok(())
    }

    pub fn cancel(&mut self, caller_did: &str) -> Result<(), Error> {
        if caller_did != self.seller_did {
            return Err(Error::PermissionDenied("only seller can cancel".into()));
        }
        if self.status != ListingStatus::Active {
            return Err(Error::InvalidInput(
                "can only cancel active listings".into(),
            ));
        }
        self.status = ListingStatus::Cancelled;
        Ok(())
    }

    pub fn matches_search(&self, query: &str) -> bool {
        let query_lower = query.to_lowercase();
        self.title.to_lowercase().contains(&query_lower)
            || self.description.to_lowercase().contains(&query_lower)
            || self
                .tags
                .iter()
                .any(|t| t.to_lowercase().contains(&query_lower))
    }

    pub fn buyer_meets_reputation(&self, reputation: f64) -> bool {
        self.min_reputation.is_none_or(|min| reputation >= min)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_listing() -> Listing {
        Listing::new(
            "did:key:seller",
            "Widget",
            "A fine widget",
            ListingCategory::Physical,
            "ETH",
            100,
        )
        .unwrap()
    }

    #[test]
    fn create_listing() {
        let listing = test_listing();
        assert_eq!(listing.status, ListingStatus::Active);
        assert!(listing.id.starts_with("listing:"));
        assert_eq!(listing.quantity, 1);
    }

    #[test]
    fn reject_empty_title() {
        assert!(
            Listing::new(
                "did:key:z",
                "",
                "desc",
                ListingCategory::Physical,
                "ETH",
                100
            )
            .is_err()
        );
    }

    #[test]
    fn reject_zero_price() {
        assert!(
            Listing::new(
                "did:key:z",
                "Title",
                "desc",
                ListingCategory::Physical,
                "ETH",
                0
            )
            .is_err()
        );
    }

    #[test]
    fn builder_methods() {
        let listing = test_listing()
            .with_quantity(5)
            .with_tag("electronics")
            .with_image("Qm123")
            .with_min_reputation(3.0);

        assert_eq!(listing.quantity, 5);
        assert_eq!(listing.tags, vec!["electronics"]);
        assert_eq!(listing.images, vec!["Qm123"]);
        assert_eq!(listing.min_reputation, Some(3.0));
    }

    #[test]
    fn purchase() {
        let mut listing = test_listing();
        assert!(listing.is_available());
        listing.purchase().unwrap();
        assert_eq!(listing.status, ListingStatus::Sold);
        assert!(!listing.is_available());
    }

    #[test]
    fn purchase_with_quantity() {
        let mut listing = test_listing().with_quantity(3);
        listing.purchase().unwrap();
        assert_eq!(listing.quantity, 2);
        assert_eq!(listing.status, ListingStatus::Active);
        listing.purchase().unwrap();
        listing.purchase().unwrap();
        assert_eq!(listing.status, ListingStatus::Sold);
    }

    #[test]
    fn purchase_sold_out() {
        let mut listing = test_listing();
        listing.purchase().unwrap();
        assert!(listing.purchase().is_err());
    }

    #[test]
    fn cancel_by_seller() {
        let mut listing = test_listing();
        listing.cancel("did:key:seller").unwrap();
        assert_eq!(listing.status, ListingStatus::Cancelled);
    }

    #[test]
    fn cancel_by_non_seller() {
        let mut listing = test_listing();
        assert!(listing.cancel("did:key:other").is_err());
    }

    #[test]
    fn search_matches_title() {
        let listing = test_listing();
        assert!(listing.matches_search("widget"));
        assert!(listing.matches_search("WIDGET"));
        assert!(!listing.matches_search("gadget"));
    }

    #[test]
    fn search_matches_tags() {
        let listing = test_listing().with_tag("electronics");
        assert!(listing.matches_search("electronic"));
    }

    #[test]
    fn reputation_gate() {
        let listing = test_listing().with_min_reputation(3.0);
        assert!(listing.buyer_meets_reputation(5.0));
        assert!(!listing.buyer_meets_reputation(2.0));
    }

    #[test]
    fn no_reputation_gate() {
        let listing = test_listing();
        assert!(listing.buyer_meets_reputation(0.0));
    }

    #[test]
    fn listing_serializes() {
        let listing = test_listing().with_tag("test");
        let json = serde_json::to_string(&listing).unwrap();
        let restored: Listing = serde_json::from_str(&json).unwrap();
        assert_eq!(restored.title, "Widget");
    }
}
