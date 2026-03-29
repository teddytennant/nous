use crate::listing::{Listing, ListingCategory, ListingStatus};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchQuery {
    pub text: Option<String>,
    pub category: Option<ListingCategory>,
    pub min_price: Option<u128>,
    pub max_price: Option<u128>,
    pub token: Option<String>,
    pub seller_did: Option<String>,
    pub tags: Vec<String>,
    pub sort: SortOrder,
    pub limit: usize,
    pub offset: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SortOrder {
    Newest,
    PriceLow,
    PriceHigh,
}

impl Default for SearchQuery {
    fn default() -> Self {
        Self {
            text: None,
            category: None,
            min_price: None,
            max_price: None,
            token: None,
            seller_did: None,
            tags: Vec::new(),
            sort: SortOrder::Newest,
            limit: 50,
            offset: 0,
        }
    }
}

impl SearchQuery {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn text(mut self, query: impl Into<String>) -> Self {
        self.text = Some(query.into());
        self
    }

    pub fn category(mut self, cat: ListingCategory) -> Self {
        self.category = Some(cat);
        self
    }

    pub fn price_range(mut self, min: u128, max: u128) -> Self {
        self.min_price = Some(min);
        self.max_price = Some(max);
        self
    }

    pub fn tag(mut self, tag: impl Into<String>) -> Self {
        self.tags.push(tag.into());
        self
    }

    pub fn sort_by(mut self, order: SortOrder) -> Self {
        self.sort = order;
        self
    }

    pub fn paginate(mut self, limit: usize, offset: usize) -> Self {
        self.limit = limit;
        self.offset = offset;
        self
    }
}

pub fn search<'a>(listings: &'a [Listing], query: &SearchQuery) -> Vec<&'a Listing> {
    let mut results: Vec<&Listing> = listings
        .iter()
        .filter(|l| l.status == ListingStatus::Active)
        .filter(|l| query.text.as_ref().is_none_or(|q| l.matches_search(q)))
        .filter(|l| query.category.is_none_or(|c| l.category == c))
        .filter(|l| query.min_price.is_none_or(|min| l.price_amount >= min))
        .filter(|l| query.max_price.is_none_or(|max| l.price_amount <= max))
        .filter(|l| query.token.as_ref().is_none_or(|t| l.price_token == *t))
        .filter(|l| query.seller_did.as_ref().is_none_or(|s| l.seller_did == *s))
        .filter(|l| {
            query.tags.is_empty()
                || query
                    .tags
                    .iter()
                    .all(|tag| l.tags.iter().any(|lt| lt.contains(tag.as_str())))
        })
        .collect();

    match query.sort {
        SortOrder::Newest => results.sort_by(|a, b| b.created_at.cmp(&a.created_at)),
        SortOrder::PriceLow => results.sort_by_key(|l| l.price_amount),
        SortOrder::PriceHigh => results.sort_by(|a, b| b.price_amount.cmp(&a.price_amount)),
    }

    results
        .into_iter()
        .skip(query.offset)
        .take(query.limit)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_listings() -> Vec<Listing> {
        vec![
            Listing::new(
                "s1",
                "Laptop",
                "Fast laptop",
                ListingCategory::Physical,
                "ETH",
                1000,
            )
            .unwrap()
            .with_tag("electronics"),
            Listing::new(
                "s1",
                "Ebook",
                "Programming guide",
                ListingCategory::Digital,
                "ETH",
                50,
            )
            .unwrap()
            .with_tag("books"),
            Listing::new(
                "s2",
                "Consulting",
                "1 hour",
                ListingCategory::Service,
                "ETH",
                200,
            )
            .unwrap()
            .with_tag("service"),
            Listing::new(
                "s1",
                "Phone",
                "Smartphone",
                ListingCategory::Physical,
                "BTC",
                500,
            )
            .unwrap()
            .with_tag("electronics"),
        ]
    }

    #[test]
    fn search_all() {
        let listings = make_listings();
        let query = SearchQuery::new();
        let results = search(&listings, &query);
        assert_eq!(results.len(), 4);
    }

    #[test]
    fn search_by_text() {
        let listings = make_listings();
        let query = SearchQuery::new().text("laptop");
        let results = search(&listings, &query);
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].title, "Laptop");
    }

    #[test]
    fn search_by_category() {
        let listings = make_listings();
        let query = SearchQuery::new().category(ListingCategory::Physical);
        let results = search(&listings, &query);
        assert_eq!(results.len(), 2);
    }

    #[test]
    fn search_by_price_range() {
        let listings = make_listings();
        let query = SearchQuery::new().price_range(100, 500);
        let results = search(&listings, &query);
        assert_eq!(results.len(), 2); // Consulting 200, Phone 500
    }

    #[test]
    fn search_by_tag() {
        let listings = make_listings();
        let query = SearchQuery::new().tag("electronics");
        let results = search(&listings, &query);
        assert_eq!(results.len(), 2);
    }

    #[test]
    fn sort_by_price_low() {
        let listings = make_listings();
        let query = SearchQuery::new().sort_by(SortOrder::PriceLow);
        let results = search(&listings, &query);
        assert_eq!(results[0].title, "Ebook"); // 50
    }

    #[test]
    fn sort_by_price_high() {
        let listings = make_listings();
        let query = SearchQuery::new().sort_by(SortOrder::PriceHigh);
        let results = search(&listings, &query);
        assert_eq!(results[0].title, "Laptop"); // 1000
    }

    #[test]
    fn pagination() {
        let listings = make_listings();
        let query = SearchQuery::new().paginate(2, 0);
        let results = search(&listings, &query);
        assert_eq!(results.len(), 2);

        let query = SearchQuery::new().paginate(2, 2);
        let results = search(&listings, &query);
        assert_eq!(results.len(), 2);
    }

    #[test]
    fn excludes_inactive() {
        let mut listings = make_listings();
        listings[0].status = ListingStatus::Sold;
        let query = SearchQuery::new();
        let results = search(&listings, &query);
        assert_eq!(results.len(), 3);
    }

    #[test]
    fn search_query_serializes() {
        let query = SearchQuery::new()
            .text("test")
            .category(ListingCategory::Digital);
        let json = serde_json::to_string(&query).unwrap();
        let _: SearchQuery = serde_json::from_str(&json).unwrap();
    }
}
