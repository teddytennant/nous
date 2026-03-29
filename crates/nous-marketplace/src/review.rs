use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use nous_core::Error;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Review {
    pub id: String,
    pub listing_id: String,
    pub reviewer_did: String,
    pub seller_did: String,
    pub rating: u8,
    pub comment: String,
    pub timestamp: DateTime<Utc>,
    pub verified_purchase: bool,
}

impl Review {
    pub fn new(
        listing_id: &str,
        reviewer_did: &str,
        seller_did: &str,
        rating: u8,
        comment: &str,
    ) -> Result<Self, Error> {
        if rating == 0 || rating > 5 {
            return Err(Error::InvalidInput("rating must be 1-5".into()));
        }
        if reviewer_did == seller_did {
            return Err(Error::InvalidInput("cannot review own listing".into()));
        }

        Ok(Self {
            id: format!("review:{}", Uuid::new_v4()),
            listing_id: listing_id.to_string(),
            reviewer_did: reviewer_did.to_string(),
            seller_did: seller_did.to_string(),
            rating,
            comment: comment.to_string(),
            timestamp: Utc::now(),
            verified_purchase: false,
        })
    }

    pub fn with_verified_purchase(mut self) -> Self {
        self.verified_purchase = true;
        self
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SellerRating {
    pub seller_did: String,
    pub total_reviews: u32,
    pub average_rating: f64,
    pub verified_reviews: u32,
}

impl SellerRating {
    pub fn compute(seller_did: &str, reviews: &[Review]) -> Self {
        let seller_reviews: Vec<&Review> = reviews
            .iter()
            .filter(|r| r.seller_did == seller_did)
            .collect();

        let total = seller_reviews.len() as u32;
        let verified = seller_reviews.iter().filter(|r| r.verified_purchase).count() as u32;
        let avg = if total > 0 {
            seller_reviews.iter().map(|r| r.rating as f64).sum::<f64>() / total as f64
        } else {
            0.0
        };

        Self {
            seller_did: seller_did.to_string(),
            total_reviews: total,
            average_rating: avg,
            verified_reviews: verified,
        }
    }

    pub fn is_trusted(&self) -> bool {
        self.total_reviews >= 5 && self.average_rating >= 3.5
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn create_review() {
        let review = Review::new("listing:1", "buyer", "seller", 5, "Great!").unwrap();
        assert_eq!(review.rating, 5);
        assert!(!review.verified_purchase);
    }

    #[test]
    fn reject_zero_rating() {
        assert!(Review::new("listing:1", "buyer", "seller", 0, "bad").is_err());
    }

    #[test]
    fn reject_rating_over_5() {
        assert!(Review::new("listing:1", "buyer", "seller", 6, "bad").is_err());
    }

    #[test]
    fn reject_self_review() {
        assert!(Review::new("listing:1", "alice", "alice", 5, "great").is_err());
    }

    #[test]
    fn verified_purchase() {
        let review = Review::new("listing:1", "buyer", "seller", 4, "good")
            .unwrap()
            .with_verified_purchase();
        assert!(review.verified_purchase);
    }

    #[test]
    fn seller_rating_computation() {
        let reviews = vec![
            Review::new("l1", "b1", "seller", 5, "great").unwrap(),
            Review::new("l2", "b2", "seller", 4, "good").unwrap(),
            Review::new("l3", "b3", "seller", 3, "ok").unwrap(),
            Review::new("l4", "b4", "other_seller", 1, "bad").unwrap(),
        ];

        let rating = SellerRating::compute("seller", &reviews);
        assert_eq!(rating.total_reviews, 3);
        assert!((rating.average_rating - 4.0).abs() < 0.01);
    }

    #[test]
    fn seller_rating_trusted() {
        let reviews: Vec<Review> = (0..5)
            .map(|i| Review::new(&format!("l{i}"), &format!("b{i}"), "seller", 4, "good").unwrap())
            .collect();

        let rating = SellerRating::compute("seller", &reviews);
        assert!(rating.is_trusted());
    }

    #[test]
    fn seller_rating_not_trusted_few_reviews() {
        let reviews = vec![
            Review::new("l1", "b1", "seller", 5, "great").unwrap(),
        ];
        let rating = SellerRating::compute("seller", &reviews);
        assert!(!rating.is_trusted());
    }

    #[test]
    fn empty_seller_rating() {
        let rating = SellerRating::compute("nobody", &[]);
        assert_eq!(rating.total_reviews, 0);
        assert_eq!(rating.average_rating, 0.0);
    }

    #[test]
    fn review_serializes() {
        let review = Review::new("l1", "buyer", "seller", 5, "great").unwrap();
        let json = serde_json::to_string(&review).unwrap();
        let _: Review = serde_json::from_str(&json).unwrap();
    }
}
