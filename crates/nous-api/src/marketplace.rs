use axum::Json;
use axum::extract::{Path, Query, State};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use utoipa::{IntoParams, ToSchema};

use nous_marketplace::{
    Dispute, DisputeReason, Evidence, Listing, ListingCategory, Offer, Order, Review, SearchQuery,
    SellerRating, SortOrder, search,
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
    let id = listing.id.clone();

    let mut listings = state.listings.write().await;
    listings.insert(id.clone(), listing);

    // Persist listing to SQLite
    if let Some(l) = listings.get(&id) {
        state.persist_listing(&id, l).await;
    }

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
    if let Some(ref cat) = params.category
        && let Ok(c) = parse_category(cat)
    {
        query = query.category(c);
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

    // Persist listing to SQLite
    state.persist_listing(&listing_id, listing).await;

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

    // Persist listing to SQLite
    state.persist_listing(&listing_id, listing).await;

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
    let id = review.id.clone();

    let mut reviews = state.reviews.write().await;
    reviews.insert(id.clone(), review);

    // Persist review to SQLite
    if let Some(r) = reviews.get(&id) {
        state.persist_review(&id, r).await;
    }

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
            if let Some(ref lid) = query.listing_id
                && &r.listing_id != lid
            {
                return false;
            }
            if let Some(ref sdid) = query.seller_did
                && &r.seller_did != sdid
            {
                return false;
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

// ── Order Types ───────────────────────────────────────────────────────────

#[derive(Debug, Deserialize, ToSchema)]
pub struct CreateOrderRequest {
    pub listing_id: String,
    pub buyer_did: String,
    pub quantity: Option<u32>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct OrderResponse {
    pub id: String,
    pub listing_id: String,
    pub buyer_did: String,
    pub seller_did: String,
    pub token: String,
    pub amount: u128,
    pub quantity: u32,
    pub status: String,
    pub escrow_id: Option<String>,
    pub shipping: Option<ShippingResponse>,
    pub created_at: String,
    pub updated_at: String,
    pub completed_at: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ShippingResponse {
    pub carrier: String,
    pub tracking_id: String,
    pub shipped_at: String,
}

impl From<&Order> for OrderResponse {
    fn from(o: &Order) -> Self {
        Self {
            id: o.id.clone(),
            listing_id: o.listing_id.clone(),
            buyer_did: o.buyer_did.clone(),
            seller_did: o.seller_did.clone(),
            token: o.token.clone(),
            amount: o.amount,
            quantity: o.quantity,
            status: format!("{:?}", o.status),
            escrow_id: o.escrow_id.clone(),
            shipping: o.shipping.as_ref().map(|s| ShippingResponse {
                carrier: s.carrier.clone(),
                tracking_id: s.tracking_id.clone(),
                shipped_at: s.shipped_at.to_rfc3339(),
            }),
            created_at: o.created_at.to_rfc3339(),
            updated_at: o.updated_at.to_rfc3339(),
            completed_at: o.completed_at.map(|t| t.to_rfc3339()),
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct OrderListResponse {
    pub orders: Vec<OrderResponse>,
    pub count: usize,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct FundEscrowRequest {
    pub escrow_id: String,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct ShipOrderRequest {
    pub seller_did: String,
    pub carrier: String,
    pub tracking_id: String,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct CallerRequest {
    pub caller_did: String,
}

#[derive(Debug, Deserialize, IntoParams)]
pub struct OrderQuery {
    pub buyer_did: Option<String>,
    pub seller_did: Option<String>,
    pub status: Option<String>,
}

// ── Order Handlers ────────────────────────────────────────────────────────

pub async fn create_order(
    State(state): State<Arc<AppState>>,
    Json(req): Json<CreateOrderRequest>,
) -> Result<Json<OrderResponse>, ApiError> {
    let listings = state.listings.read().await;
    let listing = listings
        .get(&req.listing_id)
        .ok_or_else(|| ApiError::not_found(format!("listing {} not found", req.listing_id)))?;

    if !listing.is_available() {
        return Err(ApiError::bad_request("listing is not available"));
    }

    let qty = req.quantity.unwrap_or(1);
    let order = Order::new(
        &req.listing_id,
        &req.buyer_did,
        &listing.seller_did,
        &listing.price_token,
        listing.price_amount,
        qty,
    )
    .map_err(ApiError::from)?;

    let response = OrderResponse::from(&order);

    state.emit(crate::state::RealtimeEvent::OrderUpdate {
        id: order.id.clone(),
        status: "Created".into(),
    });

    let oid = order.id.clone();
    let mut orders = state.orders.write().await;
    orders.insert(oid.clone(), order);

    // Persist order to SQLite
    if let Some(o) = orders.get(&oid) {
        state.persist_order(&oid, o).await;
    }

    Ok(Json(response))
}

pub async fn get_order(
    State(state): State<Arc<AppState>>,
    Path(order_id): Path<String>,
) -> Result<Json<OrderResponse>, ApiError> {
    let orders = state.orders.read().await;
    let order = orders
        .get(&order_id)
        .ok_or_else(|| ApiError::not_found(format!("order {order_id} not found")))?;
    Ok(Json(OrderResponse::from(order)))
}

pub async fn list_orders(
    State(state): State<Arc<AppState>>,
    Query(query): Query<OrderQuery>,
) -> Json<OrderListResponse> {
    let orders = state.orders.read().await;
    let filtered: Vec<OrderResponse> = orders
        .values()
        .filter(|o| {
            if let Some(ref buyer) = query.buyer_did
                && &o.buyer_did != buyer
            {
                return false;
            }
            if let Some(ref seller) = query.seller_did
                && &o.seller_did != seller
            {
                return false;
            }
            if let Some(ref status) = query.status
                && format!("{:?}", o.status).to_lowercase() != status.to_lowercase()
            {
                return false;
            }
            true
        })
        .map(OrderResponse::from)
        .collect();

    let count = filtered.len();
    Json(OrderListResponse {
        orders: filtered,
        count,
    })
}

pub async fn fund_order_escrow(
    State(state): State<Arc<AppState>>,
    Path(order_id): Path<String>,
    Json(req): Json<FundEscrowRequest>,
) -> Result<Json<MutationResponse>, ApiError> {
    let mut orders = state.orders.write().await;
    let order = orders
        .get_mut(&order_id)
        .ok_or_else(|| ApiError::not_found(format!("order {order_id} not found")))?;

    order.fund_escrow(&req.escrow_id).map_err(ApiError::from)?;

    // Persist order to SQLite
    state.persist_order(&order_id, order).await;

    state.emit(crate::state::RealtimeEvent::OrderUpdate {
        id: order_id.clone(),
        status: "EscrowFunded".into(),
    });

    Ok(Json(MutationResponse {
        success: true,
        message: format!("order {order_id} escrow funded"),
    }))
}

pub async fn ship_order(
    State(state): State<Arc<AppState>>,
    Path(order_id): Path<String>,
    Json(req): Json<ShipOrderRequest>,
) -> Result<Json<MutationResponse>, ApiError> {
    let mut orders = state.orders.write().await;
    let order = orders
        .get_mut(&order_id)
        .ok_or_else(|| ApiError::not_found(format!("order {order_id} not found")))?;

    order
        .ship(&req.seller_did, &req.carrier, &req.tracking_id)
        .map_err(ApiError::from)?;

    // Persist order to SQLite
    state.persist_order(&order_id, order).await;

    state.emit(crate::state::RealtimeEvent::OrderUpdate {
        id: order_id.clone(),
        status: "Shipped".into(),
    });

    Ok(Json(MutationResponse {
        success: true,
        message: format!("order {order_id} shipped"),
    }))
}

pub async fn confirm_delivery(
    State(state): State<Arc<AppState>>,
    Path(order_id): Path<String>,
    Json(req): Json<CallerRequest>,
) -> Result<Json<MutationResponse>, ApiError> {
    let mut orders = state.orders.write().await;
    let order = orders
        .get_mut(&order_id)
        .ok_or_else(|| ApiError::not_found(format!("order {order_id} not found")))?;

    order
        .confirm_delivery(&req.caller_did)
        .map_err(ApiError::from)?;

    // Persist order to SQLite
    state.persist_order(&order_id, order).await;

    state.emit(crate::state::RealtimeEvent::OrderUpdate {
        id: order_id.clone(),
        status: "Delivered".into(),
    });

    Ok(Json(MutationResponse {
        success: true,
        message: format!("order {order_id} delivery confirmed"),
    }))
}

pub async fn complete_order(
    State(state): State<Arc<AppState>>,
    Path(order_id): Path<String>,
    Json(req): Json<CallerRequest>,
) -> Result<Json<MutationResponse>, ApiError> {
    let mut orders = state.orders.write().await;
    let order = orders
        .get_mut(&order_id)
        .ok_or_else(|| ApiError::not_found(format!("order {order_id} not found")))?;

    order.complete(&req.caller_did).map_err(ApiError::from)?;

    // Persist order to SQLite
    state.persist_order(&order_id, order).await;

    state.emit(crate::state::RealtimeEvent::OrderUpdate {
        id: order_id.clone(),
        status: "Completed".into(),
    });

    Ok(Json(MutationResponse {
        success: true,
        message: format!("order {order_id} completed"),
    }))
}

pub async fn cancel_order(
    State(state): State<Arc<AppState>>,
    Path(order_id): Path<String>,
    Json(req): Json<CallerRequest>,
) -> Result<Json<MutationResponse>, ApiError> {
    let mut orders = state.orders.write().await;
    let order = orders
        .get_mut(&order_id)
        .ok_or_else(|| ApiError::not_found(format!("order {order_id} not found")))?;

    order.cancel(&req.caller_did).map_err(ApiError::from)?;

    // Persist order to SQLite
    state.persist_order(&order_id, order).await;

    state.emit(crate::state::RealtimeEvent::OrderUpdate {
        id: order_id.clone(),
        status: "Cancelled".into(),
    });

    Ok(Json(MutationResponse {
        success: true,
        message: format!("order {order_id} cancelled"),
    }))
}

pub async fn dispute_order(
    State(state): State<Arc<AppState>>,
    Path(order_id): Path<String>,
    Json(req): Json<CallerRequest>,
) -> Result<Json<MutationResponse>, ApiError> {
    let mut orders = state.orders.write().await;
    let order = orders
        .get_mut(&order_id)
        .ok_or_else(|| ApiError::not_found(format!("order {order_id} not found")))?;

    order.dispute(&req.caller_did).map_err(ApiError::from)?;

    // Persist order to SQLite
    state.persist_order(&order_id, order).await;

    state.emit(crate::state::RealtimeEvent::OrderUpdate {
        id: order_id.clone(),
        status: "Disputed".into(),
    });

    Ok(Json(MutationResponse {
        success: true,
        message: format!("order {order_id} disputed"),
    }))
}

// ── Dispute Types ─────────────────────────────────────────────────────────

#[derive(Debug, Deserialize, ToSchema)]
pub struct CreateDisputeRequest {
    pub order_id: String,
    pub initiator_did: String,
    pub respondent_did: String,
    pub reason: String,
    pub description: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct DisputeResponse {
    pub id: String,
    pub order_id: String,
    pub initiator_did: String,
    pub respondent_did: String,
    pub reason: String,
    pub description: String,
    pub evidence_count: usize,
    pub status: String,
    pub arbiter_did: Option<String>,
    pub resolution_note: Option<String>,
    pub created_at: String,
    pub resolved_at: Option<String>,
}

impl From<&Dispute> for DisputeResponse {
    fn from(d: &Dispute) -> Self {
        Self {
            id: d.id.clone(),
            order_id: d.order_id.clone(),
            initiator_did: d.initiator_did.clone(),
            respondent_did: d.respondent_did.clone(),
            reason: format!("{:?}", d.reason),
            description: d.description.clone(),
            evidence_count: d.evidence_count(),
            status: format!("{:?}", d.status),
            arbiter_did: d.arbiter_did.clone(),
            resolution_note: d.resolution_note.clone(),
            created_at: d.created_at.to_rfc3339(),
            resolved_at: d.resolved_at.map(|t| t.to_rfc3339()),
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct DisputeListResponse {
    pub disputes: Vec<DisputeResponse>,
    pub count: usize,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct AddEvidenceRequest {
    pub submitted_by: String,
    pub description: String,
    pub attachments: Option<Vec<String>>,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct AssignArbiterRequest {
    pub arbiter_did: String,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct ResolveDisputeRequest {
    pub caller_did: String,
    pub note: String,
}

#[derive(Debug, Deserialize, IntoParams)]
pub struct DisputeQuery {
    pub order_id: Option<String>,
    pub status: Option<String>,
}

// ── Dispute Handlers ──────────────────────────────────────────────────────

fn parse_dispute_reason(s: &str) -> Result<DisputeReason, ApiError> {
    match s.to_lowercase().replace('_', "").as_str() {
        "itemnotreceived" => Ok(DisputeReason::ItemNotReceived),
        "itemnotasdescribed" => Ok(DisputeReason::ItemNotAsDescribed),
        "qualityissue" => Ok(DisputeReason::QualityIssue),
        "counterfeit" => Ok(DisputeReason::Counterfeit),
        "sellerunresponsive" => Ok(DisputeReason::SellerUnresponsive),
        "other" => Ok(DisputeReason::Other),
        _ => Err(ApiError::bad_request(format!(
            "invalid dispute reason: {s}"
        ))),
    }
}

pub async fn create_dispute(
    State(state): State<Arc<AppState>>,
    Json(req): Json<CreateDisputeRequest>,
) -> Result<Json<DisputeResponse>, ApiError> {
    let reason = parse_dispute_reason(&req.reason)?;
    let dispute = Dispute::new(
        &req.order_id,
        &req.initiator_did,
        &req.respondent_did,
        reason,
        &req.description,
    )
    .map_err(ApiError::from)?;

    let response = DisputeResponse::from(&dispute);

    state.emit(crate::state::RealtimeEvent::DisputeOpened {
        id: dispute.id.clone(),
        order_id: dispute.order_id.clone(),
    });

    let did = dispute.id.clone();
    let mut disputes = state.disputes.write().await;
    disputes.insert(did.clone(), dispute);

    // Persist dispute to SQLite
    if let Some(d) = disputes.get(&did) {
        state.persist_dispute(&did, d).await;
    }

    Ok(Json(response))
}

pub async fn get_dispute(
    State(state): State<Arc<AppState>>,
    Path(dispute_id): Path<String>,
) -> Result<Json<DisputeResponse>, ApiError> {
    let disputes = state.disputes.read().await;
    let dispute = disputes
        .get(&dispute_id)
        .ok_or_else(|| ApiError::not_found(format!("dispute {dispute_id} not found")))?;
    Ok(Json(DisputeResponse::from(dispute)))
}

pub async fn list_disputes(
    State(state): State<Arc<AppState>>,
    Query(query): Query<DisputeQuery>,
) -> Json<DisputeListResponse> {
    let disputes = state.disputes.read().await;
    let filtered: Vec<DisputeResponse> = disputes
        .values()
        .filter(|d| {
            if let Some(ref oid) = query.order_id
                && &d.order_id != oid
            {
                return false;
            }
            if let Some(ref status) = query.status
                && format!("{:?}", d.status).to_lowercase() != status.to_lowercase()
            {
                return false;
            }
            true
        })
        .map(DisputeResponse::from)
        .collect();

    let count = filtered.len();
    Json(DisputeListResponse {
        disputes: filtered,
        count,
    })
}

pub async fn add_dispute_evidence(
    State(state): State<Arc<AppState>>,
    Path(dispute_id): Path<String>,
    Json(req): Json<AddEvidenceRequest>,
) -> Result<Json<MutationResponse>, ApiError> {
    let mut disputes = state.disputes.write().await;
    let dispute = disputes
        .get_mut(&dispute_id)
        .ok_or_else(|| ApiError::not_found(format!("dispute {dispute_id} not found")))?;

    let mut evidence =
        Evidence::new(&req.submitted_by, &req.description).map_err(ApiError::from)?;
    if let Some(ref attachments) = req.attachments {
        for cid in attachments {
            evidence = evidence.with_attachment(cid);
        }
    }

    dispute
        .add_evidence(evidence, &req.submitted_by)
        .map_err(ApiError::from)?;

    // Persist dispute to SQLite
    state.persist_dispute(&dispute_id, dispute).await;

    Ok(Json(MutationResponse {
        success: true,
        message: format!("evidence added to dispute {dispute_id}"),
    }))
}

pub async fn assign_dispute_arbiter(
    State(state): State<Arc<AppState>>,
    Path(dispute_id): Path<String>,
    Json(req): Json<AssignArbiterRequest>,
) -> Result<Json<MutationResponse>, ApiError> {
    let mut disputes = state.disputes.write().await;
    let dispute = disputes
        .get_mut(&dispute_id)
        .ok_or_else(|| ApiError::not_found(format!("dispute {dispute_id} not found")))?;

    dispute
        .assign_arbiter(&req.arbiter_did)
        .map_err(ApiError::from)?;

    // Persist dispute to SQLite
    state.persist_dispute(&dispute_id, dispute).await;

    Ok(Json(MutationResponse {
        success: true,
        message: format!("arbiter assigned to dispute {dispute_id}"),
    }))
}

pub async fn resolve_dispute_buyer(
    State(state): State<Arc<AppState>>,
    Path(dispute_id): Path<String>,
    Json(req): Json<ResolveDisputeRequest>,
) -> Result<Json<MutationResponse>, ApiError> {
    let mut disputes = state.disputes.write().await;
    let dispute = disputes
        .get_mut(&dispute_id)
        .ok_or_else(|| ApiError::not_found(format!("dispute {dispute_id} not found")))?;

    dispute
        .resolve_buyer_wins(&req.caller_did, &req.note)
        .map_err(ApiError::from)?;

    // Persist dispute to SQLite
    state.persist_dispute(&dispute_id, dispute).await;

    Ok(Json(MutationResponse {
        success: true,
        message: format!("dispute {dispute_id} resolved in buyer's favor"),
    }))
}

pub async fn resolve_dispute_seller(
    State(state): State<Arc<AppState>>,
    Path(dispute_id): Path<String>,
    Json(req): Json<ResolveDisputeRequest>,
) -> Result<Json<MutationResponse>, ApiError> {
    let mut disputes = state.disputes.write().await;
    let dispute = disputes
        .get_mut(&dispute_id)
        .ok_or_else(|| ApiError::not_found(format!("dispute {dispute_id} not found")))?;

    dispute
        .resolve_seller_wins(&req.caller_did, &req.note)
        .map_err(ApiError::from)?;

    // Persist dispute to SQLite
    state.persist_dispute(&dispute_id, dispute).await;

    Ok(Json(MutationResponse {
        success: true,
        message: format!("dispute {dispute_id} resolved in seller's favor"),
    }))
}

pub async fn escalate_dispute(
    State(state): State<Arc<AppState>>,
    Path(dispute_id): Path<String>,
    Json(req): Json<CallerRequest>,
) -> Result<Json<MutationResponse>, ApiError> {
    let mut disputes = state.disputes.write().await;
    let dispute = disputes
        .get_mut(&dispute_id)
        .ok_or_else(|| ApiError::not_found(format!("dispute {dispute_id} not found")))?;

    dispute.escalate(&req.caller_did).map_err(ApiError::from)?;

    // Persist dispute to SQLite
    state.persist_dispute(&dispute_id, dispute).await;

    Ok(Json(MutationResponse {
        success: true,
        message: format!("dispute {dispute_id} escalated"),
    }))
}

// ── Offer Types ───────────────────────────────────────────────────────────

#[derive(Debug, Deserialize, ToSchema)]
pub struct CreateOfferRequest {
    pub listing_id: String,
    pub buyer_did: String,
    pub amount: u128,
    pub token: String,
    pub duration_hours: Option<i64>,
    pub message: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct OfferResponse {
    pub id: String,
    pub listing_id: String,
    pub buyer_did: String,
    pub seller_did: String,
    pub token: String,
    pub amount: u128,
    pub message: Option<String>,
    pub status: String,
    pub counter_amount: Option<u128>,
    pub created_at: String,
    pub expires_at: String,
    pub responded_at: Option<String>,
}

impl From<&Offer> for OfferResponse {
    fn from(o: &Offer) -> Self {
        Self {
            id: o.id.clone(),
            listing_id: o.listing_id.clone(),
            buyer_did: o.buyer_did.clone(),
            seller_did: o.seller_did.clone(),
            token: o.token.clone(),
            amount: o.amount,
            message: o.message.clone(),
            status: format!("{:?}", o.status),
            counter_amount: o.counter_amount,
            created_at: o.created_at.to_rfc3339(),
            expires_at: o.expires_at.to_rfc3339(),
            responded_at: o.responded_at.map(|t| t.to_rfc3339()),
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct OfferListResponse {
    pub offers: Vec<OfferResponse>,
    pub count: usize,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct CounterOfferRequest {
    pub seller_did: String,
    pub counter_amount: u128,
}

#[derive(Debug, Deserialize, IntoParams)]
pub struct OfferQuery {
    pub listing_id: Option<String>,
    pub buyer_did: Option<String>,
    pub seller_did: Option<String>,
    pub status: Option<String>,
}

// ── Offer Handlers ────────────────────────────────────────────────────────

pub async fn create_offer(
    State(state): State<Arc<AppState>>,
    Json(req): Json<CreateOfferRequest>,
) -> Result<Json<OfferResponse>, ApiError> {
    let listings = state.listings.read().await;
    let listing = listings
        .get(&req.listing_id)
        .ok_or_else(|| ApiError::not_found(format!("listing {} not found", req.listing_id)))?;

    let duration = req.duration_hours.unwrap_or(48);
    let mut offer = Offer::new(
        &req.listing_id,
        &req.buyer_did,
        &listing.seller_did,
        &req.token,
        req.amount,
        duration,
    )
    .map_err(ApiError::from)?;

    if let Some(ref msg) = req.message {
        offer = offer.with_message(msg);
    }

    let response = OfferResponse::from(&offer);

    state.emit(crate::state::RealtimeEvent::OfferMade {
        id: offer.id.clone(),
        listing_id: offer.listing_id.clone(),
    });

    let oid = offer.id.clone();
    let mut offers = state.offers.write().await;
    offers.insert(oid.clone(), offer);

    // Persist offer to SQLite
    if let Some(o) = offers.get(&oid) {
        state.persist_offer(&oid, o).await;
    }

    Ok(Json(response))
}

pub async fn get_offer(
    State(state): State<Arc<AppState>>,
    Path(offer_id): Path<String>,
) -> Result<Json<OfferResponse>, ApiError> {
    let offers = state.offers.read().await;
    let offer = offers
        .get(&offer_id)
        .ok_or_else(|| ApiError::not_found(format!("offer {offer_id} not found")))?;
    Ok(Json(OfferResponse::from(offer)))
}

pub async fn list_offers(
    State(state): State<Arc<AppState>>,
    Query(query): Query<OfferQuery>,
) -> Json<OfferListResponse> {
    let offers = state.offers.read().await;
    let filtered: Vec<OfferResponse> = offers
        .values()
        .filter(|o| {
            if let Some(ref lid) = query.listing_id
                && &o.listing_id != lid
            {
                return false;
            }
            if let Some(ref buyer) = query.buyer_did
                && &o.buyer_did != buyer
            {
                return false;
            }
            if let Some(ref seller) = query.seller_did
                && &o.seller_did != seller
            {
                return false;
            }
            if let Some(ref status) = query.status
                && format!("{:?}", o.status).to_lowercase() != status.to_lowercase()
            {
                return false;
            }
            true
        })
        .map(OfferResponse::from)
        .collect();

    let count = filtered.len();
    Json(OfferListResponse {
        offers: filtered,
        count,
    })
}

pub async fn accept_offer(
    State(state): State<Arc<AppState>>,
    Path(offer_id): Path<String>,
    Json(req): Json<CallerRequest>,
) -> Result<Json<MutationResponse>, ApiError> {
    let mut offers = state.offers.write().await;
    let offer = offers
        .get_mut(&offer_id)
        .ok_or_else(|| ApiError::not_found(format!("offer {offer_id} not found")))?;

    offer.accept(&req.caller_did).map_err(ApiError::from)?;

    // Persist offer to SQLite
    state.persist_offer(&offer_id, offer).await;

    Ok(Json(MutationResponse {
        success: true,
        message: format!("offer {offer_id} accepted"),
    }))
}

pub async fn reject_offer(
    State(state): State<Arc<AppState>>,
    Path(offer_id): Path<String>,
    Json(req): Json<CallerRequest>,
) -> Result<Json<MutationResponse>, ApiError> {
    let mut offers = state.offers.write().await;
    let offer = offers
        .get_mut(&offer_id)
        .ok_or_else(|| ApiError::not_found(format!("offer {offer_id} not found")))?;

    offer.reject(&req.caller_did).map_err(ApiError::from)?;

    // Persist offer to SQLite
    state.persist_offer(&offer_id, offer).await;

    Ok(Json(MutationResponse {
        success: true,
        message: format!("offer {offer_id} rejected"),
    }))
}

pub async fn counter_offer(
    State(state): State<Arc<AppState>>,
    Path(offer_id): Path<String>,
    Json(req): Json<CounterOfferRequest>,
) -> Result<Json<MutationResponse>, ApiError> {
    let mut offers = state.offers.write().await;
    let offer = offers
        .get_mut(&offer_id)
        .ok_or_else(|| ApiError::not_found(format!("offer {offer_id} not found")))?;

    offer
        .counter(&req.seller_did, req.counter_amount)
        .map_err(ApiError::from)?;

    // Persist offer to SQLite
    state.persist_offer(&offer_id, offer).await;

    Ok(Json(MutationResponse {
        success: true,
        message: format!("offer {offer_id} countered"),
    }))
}

pub async fn withdraw_offer(
    State(state): State<Arc<AppState>>,
    Path(offer_id): Path<String>,
    Json(req): Json<CallerRequest>,
) -> Result<Json<MutationResponse>, ApiError> {
    let mut offers = state.offers.write().await;
    let offer = offers
        .get_mut(&offer_id)
        .ok_or_else(|| ApiError::not_found(format!("offer {offer_id} not found")))?;

    offer.withdraw(&req.caller_did).map_err(ApiError::from)?;

    // Persist offer to SQLite
    state.persist_offer(&offer_id, offer).await;

    Ok(Json(MutationResponse {
        success: true,
        message: format!("offer {offer_id} withdrawn"),
    }))
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

    // ── Order Tests ───────────────────────────────────────────────────────

    async fn create_test_order(app: &axum::Router) -> OrderResponse {
        let listing = create_test_listing(app).await;
        let response = app
            .clone()
            .oneshot(json_request(
                "POST",
                "/api/v1/orders",
                serde_json::json!({
                    "listing_id": listing.id,
                    "buyer_did": "did:key:zBuyer",
                }),
            ))
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
        let bytes = response.into_body().collect().await.unwrap().to_bytes();
        serde_json::from_slice(&bytes).unwrap()
    }

    #[tokio::test]
    async fn create_and_get_order() {
        let app = test_app().await;
        let order = create_test_order(&app).await;

        assert!(order.id.starts_with("order:"));
        assert_eq!(order.buyer_did, "did:key:zBuyer");
        assert_eq!(order.seller_did, "did:key:zSeller");
        assert_eq!(order.status, "Created");
        assert_eq!(order.amount, 500);

        let response = app
            .oneshot(
                Request::builder()
                    .uri(format!("/api/v1/orders/{}", order.id))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn order_full_lifecycle() {
        let app = test_app().await;
        let order = create_test_order(&app).await;

        // Fund escrow
        let response = app
            .clone()
            .oneshot(json_request(
                "POST",
                &format!("/api/v1/orders/{}/fund", order.id),
                serde_json::json!({ "escrow_id": "escrow:abc" }),
            ))
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);

        // Ship
        let response = app
            .clone()
            .oneshot(json_request(
                "POST",
                &format!("/api/v1/orders/{}/ship", order.id),
                serde_json::json!({
                    "seller_did": "did:key:zSeller",
                    "carrier": "FedEx",
                    "tracking_id": "TRACK123"
                }),
            ))
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);

        // Confirm delivery
        let response = app
            .clone()
            .oneshot(json_request(
                "POST",
                &format!("/api/v1/orders/{}/deliver", order.id),
                serde_json::json!({ "caller_did": "did:key:zBuyer" }),
            ))
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);

        // Complete
        let response = app
            .clone()
            .oneshot(json_request(
                "POST",
                &format!("/api/v1/orders/{}/complete", order.id),
                serde_json::json!({ "caller_did": "did:key:zBuyer" }),
            ))
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);

        // Verify final status
        let response = app
            .oneshot(
                Request::builder()
                    .uri(format!("/api/v1/orders/{}", order.id))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        let bytes = response.into_body().collect().await.unwrap().to_bytes();
        let final_order: OrderResponse = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(final_order.status, "Completed");
        assert!(final_order.completed_at.is_some());
        assert!(final_order.shipping.is_some());
    }

    #[tokio::test]
    async fn cancel_order_before_shipping() {
        let app = test_app().await;
        let order = create_test_order(&app).await;

        let response = app
            .clone()
            .oneshot(json_request(
                "POST",
                &format!("/api/v1/orders/{}/cancel", order.id),
                serde_json::json!({ "caller_did": "did:key:zBuyer" }),
            ))
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn dispute_funded_order() {
        let app = test_app().await;
        let order = create_test_order(&app).await;

        // Fund
        let _ = app
            .clone()
            .oneshot(json_request(
                "POST",
                &format!("/api/v1/orders/{}/fund", order.id),
                serde_json::json!({ "escrow_id": "escrow:abc" }),
            ))
            .await
            .unwrap();

        // Dispute
        let response = app
            .clone()
            .oneshot(json_request(
                "POST",
                &format!("/api/v1/orders/{}/dispute", order.id),
                serde_json::json!({ "caller_did": "did:key:zBuyer" }),
            ))
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);

        let response = app
            .oneshot(
                Request::builder()
                    .uri(format!("/api/v1/orders/{}", order.id))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        let bytes = response.into_body().collect().await.unwrap().to_bytes();
        let disputed: OrderResponse = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(disputed.status, "Disputed");
    }

    #[tokio::test]
    async fn list_orders_by_buyer() {
        let app = test_app().await;
        let _ = create_test_order(&app).await;

        let response = app
            .oneshot(
                Request::builder()
                    .uri("/api/v1/orders?buyer_did=did:key:zBuyer")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
        let bytes = response.into_body().collect().await.unwrap().to_bytes();
        let list: OrderListResponse = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(list.count, 1);
    }

    #[tokio::test]
    async fn order_not_found() {
        let app = test_app().await;
        let response = app
            .oneshot(
                Request::builder()
                    .uri("/api/v1/orders/nonexistent")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::NOT_FOUND);
    }

    // ── Dispute Tests ─────────────────────────────────────────────────────

    #[tokio::test]
    async fn create_and_get_dispute() {
        let app = test_app().await;

        let response = app
            .clone()
            .oneshot(json_request(
                "POST",
                "/api/v1/disputes",
                serde_json::json!({
                    "order_id": "order:abc",
                    "initiator_did": "did:key:zBuyer",
                    "respondent_did": "did:key:zSeller",
                    "reason": "item_not_received",
                    "description": "Item never arrived"
                }),
            ))
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
        let bytes = response.into_body().collect().await.unwrap().to_bytes();
        let dispute: DisputeResponse = serde_json::from_slice(&bytes).unwrap();
        assert!(dispute.id.starts_with("dispute:"));
        assert_eq!(dispute.status, "Open");
        assert_eq!(dispute.reason, "ItemNotReceived");

        // Get by ID
        let response = app
            .oneshot(
                Request::builder()
                    .uri(format!("/api/v1/disputes/{}", dispute.id))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn dispute_evidence_and_resolution() {
        let app = test_app().await;

        // Create dispute
        let response = app
            .clone()
            .oneshot(json_request(
                "POST",
                "/api/v1/disputes",
                serde_json::json!({
                    "order_id": "order:abc",
                    "initiator_did": "did:key:zBuyer",
                    "respondent_did": "did:key:zSeller",
                    "reason": "quality_issue",
                    "description": "Item damaged on arrival"
                }),
            ))
            .await
            .unwrap();
        let bytes = response.into_body().collect().await.unwrap().to_bytes();
        let dispute: DisputeResponse = serde_json::from_slice(&bytes).unwrap();

        // Add evidence
        let response = app
            .clone()
            .oneshot(json_request(
                "POST",
                &format!("/api/v1/disputes/{}/evidence", dispute.id),
                serde_json::json!({
                    "submitted_by": "did:key:zBuyer",
                    "description": "Photo of damaged item",
                    "attachments": ["Qm123abc"]
                }),
            ))
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);

        // Assign arbiter
        let response = app
            .clone()
            .oneshot(json_request(
                "POST",
                &format!("/api/v1/disputes/{}/arbiter", dispute.id),
                serde_json::json!({ "arbiter_did": "did:key:zJudge" }),
            ))
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);

        // Resolve in buyer's favor
        let response = app
            .clone()
            .oneshot(json_request(
                "POST",
                &format!("/api/v1/disputes/{}/resolve-buyer", dispute.id),
                serde_json::json!({
                    "caller_did": "did:key:zJudge",
                    "note": "Evidence clearly shows damage"
                }),
            ))
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);

        // Verify final state
        let response = app
            .oneshot(
                Request::builder()
                    .uri(format!("/api/v1/disputes/{}", dispute.id))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        let bytes = response.into_body().collect().await.unwrap().to_bytes();
        let resolved: DisputeResponse = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(resolved.status, "ResolvedBuyerWins");
        assert_eq!(resolved.evidence_count, 1);
        assert!(resolved.resolved_at.is_some());
    }

    #[tokio::test]
    async fn escalate_dispute_api() {
        let app = test_app().await;

        let response = app
            .clone()
            .oneshot(json_request(
                "POST",
                "/api/v1/disputes",
                serde_json::json!({
                    "order_id": "order:xyz",
                    "initiator_did": "did:key:zBuyer",
                    "respondent_did": "did:key:zSeller",
                    "reason": "seller_unresponsive",
                    "description": "No response in 14 days"
                }),
            ))
            .await
            .unwrap();
        let bytes = response.into_body().collect().await.unwrap().to_bytes();
        let dispute: DisputeResponse = serde_json::from_slice(&bytes).unwrap();

        let response = app
            .clone()
            .oneshot(json_request(
                "POST",
                &format!("/api/v1/disputes/{}/escalate", dispute.id),
                serde_json::json!({ "caller_did": "did:key:zBuyer" }),
            ))
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn list_disputes_by_order() {
        let app = test_app().await;

        let _ = app
            .clone()
            .oneshot(json_request(
                "POST",
                "/api/v1/disputes",
                serde_json::json!({
                    "order_id": "order:target",
                    "initiator_did": "did:key:zBuyer",
                    "respondent_did": "did:key:zSeller",
                    "reason": "other",
                    "description": "Issue"
                }),
            ))
            .await
            .unwrap();

        let response = app
            .oneshot(
                Request::builder()
                    .uri("/api/v1/disputes?order_id=order:target")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        let bytes = response.into_body().collect().await.unwrap().to_bytes();
        let list: DisputeListResponse = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(list.count, 1);
    }

    // ── Offer Tests ───────────────────────────────────────────────────────

    async fn create_test_offer(app: &axum::Router) -> OfferResponse {
        let listing = create_test_listing(app).await;
        let response = app
            .clone()
            .oneshot(json_request(
                "POST",
                "/api/v1/offers",
                serde_json::json!({
                    "listing_id": listing.id,
                    "buyer_did": "did:key:zBuyer",
                    "amount": 400,
                    "token": "ETH",
                    "message": "Would you take 400?"
                }),
            ))
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
        let bytes = response.into_body().collect().await.unwrap().to_bytes();
        serde_json::from_slice(&bytes).unwrap()
    }

    #[tokio::test]
    async fn create_and_get_offer() {
        let app = test_app().await;
        let offer = create_test_offer(&app).await;

        assert!(offer.id.starts_with("offer:"));
        assert_eq!(offer.amount, 400);
        assert_eq!(offer.status, "Pending");
        assert_eq!(offer.message.as_deref(), Some("Would you take 400?"));

        let response = app
            .oneshot(
                Request::builder()
                    .uri(format!("/api/v1/offers/{}", offer.id))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn accept_offer_api() {
        let app = test_app().await;
        let offer = create_test_offer(&app).await;

        let response = app
            .clone()
            .oneshot(json_request(
                "POST",
                &format!("/api/v1/offers/{}/accept", offer.id),
                serde_json::json!({ "caller_did": "did:key:zSeller" }),
            ))
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);

        let response = app
            .oneshot(
                Request::builder()
                    .uri(format!("/api/v1/offers/{}", offer.id))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        let bytes = response.into_body().collect().await.unwrap().to_bytes();
        let accepted: OfferResponse = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(accepted.status, "Accepted");
    }

    #[tokio::test]
    async fn reject_offer_api() {
        let app = test_app().await;
        let offer = create_test_offer(&app).await;

        let response = app
            .clone()
            .oneshot(json_request(
                "POST",
                &format!("/api/v1/offers/{}/reject", offer.id),
                serde_json::json!({ "caller_did": "did:key:zSeller" }),
            ))
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn counter_offer_api() {
        let app = test_app().await;
        let offer = create_test_offer(&app).await;

        let response = app
            .clone()
            .oneshot(json_request(
                "POST",
                &format!("/api/v1/offers/{}/counter", offer.id),
                serde_json::json!({
                    "seller_did": "did:key:zSeller",
                    "counter_amount": 450
                }),
            ))
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);

        let response = app
            .oneshot(
                Request::builder()
                    .uri(format!("/api/v1/offers/{}", offer.id))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        let bytes = response.into_body().collect().await.unwrap().to_bytes();
        let countered: OfferResponse = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(countered.status, "Countered");
        assert_eq!(countered.counter_amount, Some(450));
    }

    #[tokio::test]
    async fn withdraw_offer_api() {
        let app = test_app().await;
        let offer = create_test_offer(&app).await;

        let response = app
            .clone()
            .oneshot(json_request(
                "POST",
                &format!("/api/v1/offers/{}/withdraw", offer.id),
                serde_json::json!({ "caller_did": "did:key:zBuyer" }),
            ))
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn list_offers_by_listing() {
        let app = test_app().await;
        let offer = create_test_offer(&app).await;

        let response = app
            .oneshot(
                Request::builder()
                    .uri(format!("/api/v1/offers?listing_id={}", offer.listing_id))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        let bytes = response.into_body().collect().await.unwrap().to_bytes();
        let list: OfferListResponse = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(list.count, 1);
    }

    #[tokio::test]
    async fn buyer_cannot_accept_own_offer() {
        let app = test_app().await;
        let offer = create_test_offer(&app).await;

        let response = app
            .clone()
            .oneshot(json_request(
                "POST",
                &format!("/api/v1/offers/{}/accept", offer.id),
                serde_json::json!({ "caller_did": "did:key:zBuyer" }),
            ))
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    }
}
