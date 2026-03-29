use axum::extract::{Path, Query, State};
use axum::Json;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use utoipa::{IntoParams, ToSchema};

use nous_marketplace::{
    Listing, ListingCategory, Review, SearchQuery, SellerRating, SortOrder, search,
};

use crate::error::ApiError;
use crate::state::AppState;

// ── Request/Response Types ─────────────────────────────────────────────────

#[derive(Debug, Deserialize, ToSchema)]
pub struct CreateListingRequest {
    pub seller_did: String,
    pub title: String,
    pub description: String,
    pub category: String,
    pub price_token: String,
    pub price_amount: u128,
    pub quantity: Option<u32>,
    pub tags: Option<Vec<String>>,
    pub images: Option<Vec<String>>,
    pub min_reputation: Option<f64>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ListingResponse {
    pub id: String,
    pub seller_did: String,
    pub title: String,
    pub description: String,
    pub category: String,
    pub price_token: String,
    pub price_amount: u128,
    pub quantity: u32,
    pub status: String,
    pub created_at: String,
    pub tags: Vec<String>,
    pub images: Vec<String>,
}

impl From<&Listing> for ListingResponse {
    fn from(l: &Listing) -> Self {
        Self {
            id: l.id.clone(),
            seller_did: l.seller_did.clone(),
            title: l.title.clone(),
            description: l.description.clone(),
            category: format!("{:?}", l.category),
            price_token: l.price_token.clone(),
            price_amount: l.price_amount,
            quantity: l.quantity,
            status: format!("{:?}", l.status),
            created_at: l.created_at.to_rfc3339(),
            tags: l.tags.clone(),
            images: l.images.clone(),
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ListingListResponse {
    pub listings: Vec<ListingResponse>,
    pub count: usize,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct CreateReviewRequest {
    pub listing_id: String,
    pub reviewer_did: String,
    pub seller_did: String,
    pub rating: u8,
    pub comment: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ReviewResponse {
    pub id: String,
    pub listing_id: String,
    pub reviewer_did: String,
    pub seller_did: String,
    pub rating: u8,
    pub comment: String,
    pub timestamp: String,
    pub verified_purchase: bool,
}

impl From<&Review> for ReviewResponse {
    fn from(r: &Review) -> Self {
        Self {
            id: r.id.clone(),
            listing_id: r.listing_id.clone(),
            reviewer_did: r.reviewer_did.clone(),
            seller_did: r.seller_did.clone(),
            rating: r.rating,
            comment: r.comment.clone(),
            timestamp: r.timestamp.to_rfc3339(),
            verified_purchase: r.verified_purchase,
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ReviewListResponse {
    pub reviews: Vec<ReviewResponse>,
    pub count: usize,
}

#[derive(Debug, Deserialize, ToSchema, IntoParams)]
pub struct SearchQueryParams {
    pub text: Option<String>,
    pub category: Option<String>,
    pub min_price: Option<u128>,
    pub max_price: Option<u128>,
    pub token: Option<String>,
    pub seller_did: Option<String>,
    pub tags: Option<String>,
    pub sort: Option<String>,
    pub limit: Option<usize>,
    pub offset: Option<usize>,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct PurchaseRequest {
    pub buyer_did: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct MutationResponse {
    pub success: bool,
    pub message: String,
}

// ── Helpers ────────────────────────────────────────────────────────────────

fn parse_category(s: &str) -> Result<ListingCategory, ApiError> {
    match s.to_lowercase().as_str() {
        "physical" => Ok(ListingCategory::Physical),
        "digital" => Ok(ListingCategory::Digital),
        "service" => Ok(ListingCategory::Service),
        "nft" => Ok(ListingCategory::NFT),
        "data" => Ok(ListingCategory::Data),
        "other" => Ok(ListingCategory::Other),
        _ => Err(ApiError::bad_request(format!(
            "invalid category: {s}. Valid: physical, digital, service, nft, data, other"
        ))),
    }
}

fn parse_sort(s: &str) -> SortOrder {
    match s.to_lowercase().as_str() {
        "price_low" => SortOrder::PriceLow,
        "price_high" => SortOrder::PriceHigh,
        _ => SortOrder::Newest,
    }
}

// ── Listing Handlers ───────────────────────────────────────────────────────

#[utoipa::path(
    post, path = "/api/v1/listings",
    tag = "marketplace",
    request_body = CreateListingRequest,
    responses(
        (status = 200, description = "Listing created"),
        (status = 400, description = "Invalid request")
    )
)]
pub async fn create_listing(
    State(state): State<Arc<AppState>>,
    Json(req): Json<CreateListingRequest>,
) -> Result<Json<ListingResponse>, ApiError> {
    let category = parse_category(&req.category)?;

    let mut listing = Listing::new(
        &req.seller_did,
        &req.title,
        &req.description,
        category,
        &req.price_token,
        req.price_amount,
    )
    .map_err(ApiError::from)?;

    if let Some(qty) = req.quantity {
        listing = listing.with_quantity(qty);
    }
    if let Some(ref tags) = req.tags {
        for tag in tags {
            listing = listing.with_tag(tag);
        }
    }
    if let Some(ref images) = req.images {
        for img in images {
            listing = listing.with_image(img);
        }
    }
    if let Some(min_rep) = req.min_reputation {
        listing = listing.with_min_reputation(min_rep);
    }

    let response = ListingResponse::from(&listing);

    let mut listings = state.listings.write().await;
    listings.insert(listing.id.clone(), listing);

    Ok(Json(response))
}

#[utoipa::path(
    get, path = "/api/v1/listings",
    tag = "marketplace",
    params(SearchQueryParams),
    responses((status = 200, description = "Search results"))
)]
pub async fn search_listings(
    State(state): State<Arc<AppState>>,
    Query(params): Query<SearchQueryParams>,
) -> Json<ListingListResponse> {
    let listings = state.listings.read().await;
    let listing_vec: Vec<Listing> = listings.values().cloned().collect();

    let mut query = SearchQuery::new();
    if let Some(ref text) = params.text {
        query = query.text(text);
    }
    if let Some(ref cat) = params.category {
        if let Ok(c) = parse_category(cat) {
            query = query.category(c);
        }
    }
    if let (Some(min), Some(max)) = (params.min_price, params.max_price) {
        query = query.price_range(min, max);
    }
    if let Some(ref tags) = params.tags {
        for tag in tags.split(',') {
            query = query.tag(tag.trim());
        }
    }
    if let Some(ref sort) = params.sort {
        query = query.sort_by(parse_sort(sort));
    }
    if let Some(limit) = params.limit {
        let offset = params.offset.unwrap_or(0);
        query = query.paginate(limit, offset);
    }

    let results: Vec<ListingResponse> = search(&listing_vec, &query)
        .into_iter()
        .map(ListingResponse::from)
        .collect();

    let count = results.len();
    Json(ListingListResponse {
        listings: results,
        count,
    })
}

#[utoipa::path(
    get, path = "/api/v1/listings/{listing_id}",
    tag = "marketplace",
    params(("listing_id" = String, Path, description = "Listing identifier")),
    responses(
        (status = 200, description = "Listing details"),
        (status = 404, description = "Listing not found")
    )
)]
pub async fn get_listing(
    State(state): State<Arc<AppState>>,
    Path(listing_id): Path<String>,
) -> Result<Json<ListingResponse>, ApiError> {
    let listings = state.listings.read().await;
    let listing = listings
        .get(&listing_id)
        .ok_or_else(|| ApiError::not_found(format!("listing {listing_id} not found")))?;

    Ok(Json(ListingResponse::from(listing)))
}

#[utoipa::path(
    post, path = "/api/v1/listings/{listing_id}/purchase",
    tag = "marketplace",
    params(("listing_id" = String, Path, description = "Listing to purchase")),
    request_body = PurchaseRequest,
    responses(
        (status = 200, description = "Purchase successful"),
        (status = 400, description = "Not available"),
        (status = 404, description = "Listing not found")
    )
)]
pub async fn purchase_listing(
    State(state): State<Arc<AppState>>,
    Path(listing_id): Path<String>,
    Json(req): Json<PurchaseRequest>,
) -> Result<Json<MutationResponse>, ApiError> {
    let mut listings = state.listings.write().await;
    let listing = listings
        .get_mut(&listing_id)
        .ok_or_else(|| ApiError::not_found(format!("listing {listing_id} not found")))?;

    if !listing.is_available() {
        return Err(ApiError::bad_request("listing is not available"));
    }

    if let Some(min_rep) = listing.min_reputation {
        // Check buyer reputation
        let reviews = state.reviews.read().await;
        let all_reviews: Vec<Review> = reviews.values().cloned().collect();
        let rating = SellerRating::compute(&req.buyer_did, &all_reviews);
        if !listing.buyer_meets_reputation(rating.average_rating) {
            return Err(ApiError::bad_request(format!(
                "buyer reputation {:.1} below minimum {:.1}",
                rating.average_rating, min_rep
            )));
        }
    }

    listing.purchase().map_err(ApiError::from)?;

    Ok(Json(MutationResponse {
        success: true,
        message: format!("purchased listing {listing_id}"),
    }))
}

#[utoipa::path(
    delete, path = "/api/v1/listings/{listing_id}",
    tag = "marketplace",
    params(("listing_id" = String, Path, description = "Listing to cancel")),
    responses(
        (status = 200, description = "Listing cancelled"),
        (status = 404, description = "Listing not found"),
        (status = 401, description = "Not the seller")
    )
)]
pub async fn cancel_listing(
    State(state): State<Arc<AppState>>,
    Path(listing_id): Path<String>,
    Query(query): Query<SellerQuery>,
) -> Result<Json<MutationResponse>, ApiError> {
    let mut listings = state.listings.write().await;
    let listing = listings
        .get_mut(&listing_id)
        .ok_or_else(|| ApiError::not_found(format!("listing {listing_id} not found")))?;

    listing.cancel(&query.seller_did).map_err(ApiError::from)?;

    Ok(Json(MutationResponse {
        success: true,
        message: format!("listing {listing_id} cancelled"),
    }))
}

#[derive(Debug, Deserialize, IntoParams)]
pub struct SellerQuery {
    pub seller_did: String,
}

// ── Review Handlers ────────────────────────────────────────────────────────

#[utoipa::path(
    post, path = "/api/v1/reviews",
    tag = "marketplace",
    request_body = CreateReviewRequest,
    responses(
        (status = 200, description = "Review submitted"),
        (status = 400, description = "Invalid review")
    )
)]
pub async fn create_review(
    State(state): State<Arc<AppState>>,
    Json(req): Json<CreateReviewRequest>,
) -> Result<Json<ReviewResponse>, ApiError> {
    let review = Review::new(
        &req.listing_id,
        &req.reviewer_did,
        &req.seller_did,
        req.rating,
        &req.comment,
    )
    .map_err(ApiError::from)?;

    let response = ReviewResponse::from(&review);

    let mut reviews = state.reviews.write().await;
    reviews.insert(review.id.clone(), review);

    Ok(Json(response))
}

#[derive(Debug, Deserialize, IntoParams)]
pub struct ReviewQuery {
    pub listing_id: Option<String>,
    pub seller_did: Option<String>,
}

#[utoipa::path(
    get, path = "/api/v1/reviews",
    tag = "marketplace",
    params(ReviewQuery),
    responses((status = 200, description = "Reviews"))
)]
pub async fn list_reviews(
    State(state): State<Arc<AppState>>,
    Query(query): Query<ReviewQuery>,
) -> Json<ReviewListResponse> {
    let reviews = state.reviews.read().await;

    let filtered: Vec<ReviewResponse> = reviews
        .values()
        .filter(|r| {
            if let Some(ref lid) = query.listing_id {
                if &r.listing_id != lid {
                    return false;
                }
            }
            if let Some(ref sdid) = query.seller_did {
                if &r.seller_did != sdid {
                    return false;
                }
            }
            true
        })
        .map(ReviewResponse::from)
        .collect();

    let count = filtered.len();
    Json(ReviewListResponse {
        reviews: filtered,
        count,
    })
}

#[utoipa::path(
    get, path = "/api/v1/sellers/{seller_did}/rating",
    tag = "marketplace",
    params(("seller_did" = String, Path, description = "Seller DID")),
    responses((status = 200, description = "Seller reputation"))
)]
pub async fn get_seller_rating(
    State(state): State<Arc<AppState>>,
    Path(seller_did): Path<String>,
) -> Json<SellerRating> {
    let reviews = state.reviews.read().await;
    let all_reviews: Vec<Review> = reviews.values().cloned().collect();
    let rating = SellerRating::compute(&seller_did, &all_reviews);
    Json(rating)
}

// ── Tests ──────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::ApiConfig;
    use crate::router;
    use axum::body::Body;
    use axum::http::{Request, StatusCode};
    use http_body_util::BodyExt;
    use tower::ServiceExt;

    async fn test_app() -> axum::Router {
        router(ApiConfig::default())
    }

    fn json_request(method: &str, uri: &str, body: serde_json::Value) -> Request<Body> {
        Request::builder()
            .method(method)
            .uri(uri)
            .header("content-type", "application/json")
            .body(Body::from(serde_json::to_vec(&body).unwrap()))
            .unwrap()
    }

    fn listing_body() -> serde_json::Value {
        serde_json::json!({
            "seller_did": "did:key:zSeller",
            "title": "Vintage Camera",
            "description": "A rare 1960s film camera in excellent condition",
            "category": "physical",
            "price_token": "ETH",
            "price_amount": 500,
            "quantity": 1,
            "tags": ["vintage", "camera", "film"]
        })
    }

    async fn create_test_listing(app: &axum::Router) -> ListingResponse {
        let response = app
            .clone()
            .oneshot(json_request("POST", "/api/v1/listings", listing_body()))
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
        let bytes = response.into_body().collect().await.unwrap().to_bytes();
        serde_json::from_slice(&bytes).unwrap()
    }

    #[tokio::test]
    async fn create_and_get_listing() {
        let app = test_app().await;
        let listing = create_test_listing(&app).await;

        assert_eq!(listing.title, "Vintage Camera");
        assert_eq!(listing.price_amount, 500);
        assert!(listing.id.starts_with("listing:"));

        let response = app
            .oneshot(
                Request::builder()
                    .uri(format!("/api/v1/listings/{}", listing.id))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn create_listing_rejects_empty_title() {
        let app = test_app().await;

        let response = app
            .oneshot(json_request(
                "POST",
                "/api/v1/listings",
                serde_json::json!({
                    "seller_did": "did:key:z123",
                    "title": "",
                    "description": "Bad",
                    "category": "digital",
                    "price_token": "ETH",
                    "price_amount": 100
                }),
            ))
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    }

    #[tokio::test]
    async fn search_listings_by_text() {
        let app = test_app().await;
        let _ = create_test_listing(&app).await;

        let response = app
            .oneshot(
                Request::builder()
                    .uri("/api/v1/listings?text=camera")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
        let bytes = response.into_body().collect().await.unwrap().to_bytes();
        let list: ListingListResponse = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(list.count, 1);
    }

    #[tokio::test]
    async fn search_no_results() {
        let app = test_app().await;
        let _ = create_test_listing(&app).await;

        let response = app
            .oneshot(
                Request::builder()
                    .uri("/api/v1/listings?text=spaceship")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
        let bytes = response.into_body().collect().await.unwrap().to_bytes();
        let list: ListingListResponse = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(list.count, 0);
    }

    #[tokio::test]
    async fn purchase_listing_success() {
        let app = test_app().await;
        let listing = create_test_listing(&app).await;

        let response = app
            .clone()
            .oneshot(json_request(
                "POST",
                &format!("/api/v1/listings/{}/purchase", listing.id),
                serde_json::json!({ "buyer_did": "did:key:zBuyer" }),
            ))
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);

        // Verify listing is now sold
        let response = app
            .oneshot(
                Request::builder()
                    .uri(format!("/api/v1/listings/{}", listing.id))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        let bytes = response.into_body().collect().await.unwrap().to_bytes();
        let detail: ListingResponse = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(detail.status, "Sold");
    }

    #[tokio::test]
    async fn cancel_listing_by_seller() {
        let app = test_app().await;
        let listing = create_test_listing(&app).await;

        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("DELETE")
                    .uri(format!(
                        "/api/v1/listings/{}?seller_did=did:key:zSeller",
                        listing.id
                    ))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn create_and_list_reviews() {
        let app = test_app().await;
        let listing = create_test_listing(&app).await;

        let response = app
            .clone()
            .oneshot(json_request(
                "POST",
                "/api/v1/reviews",
                serde_json::json!({
                    "listing_id": listing.id,
                    "reviewer_did": "did:key:zBuyer",
                    "seller_did": "did:key:zSeller",
                    "rating": 5,
                    "comment": "Excellent camera, exactly as described"
                }),
            ))
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);

        // List reviews for listing
        let response = app
            .oneshot(
                Request::builder()
                    .uri(format!("/api/v1/reviews?listing_id={}", listing.id))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
        let bytes = response.into_body().collect().await.unwrap().to_bytes();
        let list: ReviewListResponse = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(list.count, 1);
        assert_eq!(list.reviews[0].rating, 5);
    }

    #[tokio::test]
    async fn review_rejects_self_review() {
        let app = test_app().await;

        let response = app
            .oneshot(json_request(
                "POST",
                "/api/v1/reviews",
                serde_json::json!({
                    "listing_id": "listing:123",
                    "reviewer_did": "did:key:zSame",
                    "seller_did": "did:key:zSame",
                    "rating": 5,
                    "comment": "I love my own stuff"
                }),
            ))
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    }

    #[tokio::test]
    async fn seller_rating_computation() {
        let app = test_app().await;

        // Create multiple reviews
        for i in 1..=3 {
            let _ = app
                .clone()
                .oneshot(json_request(
                    "POST",
                    "/api/v1/reviews",
                    serde_json::json!({
                        "listing_id": format!("listing:{i}"),
                        "reviewer_did": format!("did:key:zBuyer{i}"),
                        "seller_did": "did:key:zSeller",
                        "rating": i + 2,
                        "comment": format!("Review {i}")
                    }),
                ))
                .await
                .unwrap();
        }

        let response = app
            .oneshot(
                Request::builder()
                    .uri("/api/v1/sellers/did:key:zSeller/rating")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
        let bytes = response.into_body().collect().await.unwrap().to_bytes();
        let rating: SellerRating = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(rating.total_reviews, 3);
        assert!((rating.average_rating - 4.0).abs() < 0.01); // (3+4+5)/3 = 4.0
    }

    #[tokio::test]
    async fn listing_not_found() {
        let app = test_app().await;

        let response = app
            .oneshot(
                Request::builder()
                    .uri("/api/v1/listings/nonexistent")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::NOT_FOUND);
    }
}
