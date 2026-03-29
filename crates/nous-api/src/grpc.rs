use std::sync::Arc;

use tonic::{Request, Response, Status};

use crate::state::AppState;

pub mod pb {
    tonic::include_proto!("nous.v1");
}

use pb::governance_service_server::GovernanceService;
use pb::identity_service_server::IdentityService;
use pb::marketplace_service_server::MarketplaceService;
use pb::node_service_server::NodeService;
use pb::social_service_server::SocialService;

// ---- Node Service ----

pub struct NousNodeService {
    _state: Arc<AppState>,
}

impl NousNodeService {
    pub fn new(state: Arc<AppState>) -> Self {
        Self { _state: state }
    }
}

#[tonic::async_trait]
impl NodeService for NousNodeService {
    async fn health(
        &self,
        _request: Request<pb::HealthRequest>,
    ) -> Result<Response<pb::HealthResponse>, Status> {
        Ok(Response::new(pb::HealthResponse {
            status: "ok".into(),
        }))
    }

    async fn node_info(
        &self,
        _request: Request<pb::NodeInfoRequest>,
    ) -> Result<Response<pb::NodeInfoResponse>, Status> {
        Ok(Response::new(pb::NodeInfoResponse {
            protocol: "nous/0.1".into(),
            version: env!("CARGO_PKG_VERSION").into(),
            features: vec![
                "identity".into(),
                "messaging".into(),
                "governance".into(),
                "social".into(),
                "payments".into(),
                "ai".into(),
                "browser".into(),
                "storage".into(),
            ],
        }))
    }
}

// ---- Social Service ----

pub struct NousSocialService {
    state: Arc<AppState>,
}

impl NousSocialService {
    pub fn new(state: Arc<AppState>) -> Self {
        Self { state }
    }
}

#[tonic::async_trait]
impl SocialService for NousSocialService {
    async fn create_post(
        &self,
        request: Request<pb::CreatePostRequest>,
    ) -> Result<Response<pb::CreatePostResponse>, Status> {
        let req = request.into_inner();

        if req.content.is_empty() {
            return Err(Status::invalid_argument("content must not be empty"));
        }
        if req.content.len() > 10_000 {
            return Err(Status::invalid_argument(
                "content exceeds 10,000 characters",
            ));
        }

        let tags: Vec<nous_social::Tag> = req
            .hashtags
            .iter()
            .map(|t| nous_social::Tag::hashtag(t))
            .collect();

        let event = nous_social::SignedEvent::new(
            &req.author_did,
            nous_social::EventKind::TextNote,
            &req.content,
            tags,
        );
        let event_id = event.id.clone();

        let mut feed = self.state.feed.write().await;
        feed.insert(event);

        Ok(Response::new(pb::CreatePostResponse { event_id }))
    }

    async fn get_feed(
        &self,
        request: Request<pb::GetFeedRequest>,
    ) -> Result<Response<pb::GetFeedResponse>, Status> {
        let req = request.into_inner();
        let limit = if req.limit == 0 {
            50
        } else {
            req.limit as usize
        };
        let feed = self.state.feed.read().await;

        let events: Vec<&nous_social::SignedEvent> = if !req.author.is_empty() {
            feed.by_author(&req.author)
        } else if !req.hashtag.is_empty() {
            feed.by_hashtag(&req.hashtag)
        } else if req.kind > 0 {
            feed.by_kind(nous_social::EventKind::from(req.kind))
        } else {
            feed.latest(limit)
        };

        let event_messages: Vec<pb::EventMessage> = events
            .iter()
            .take(limit)
            .map(|e| event_to_proto(e))
            .collect();

        let count = event_messages.len() as u32;
        Ok(Response::new(pb::GetFeedResponse {
            events: event_messages,
            count,
        }))
    }

    async fn get_event(
        &self,
        request: Request<pb::GetEventRequest>,
    ) -> Result<Response<pb::EventMessage>, Status> {
        let req = request.into_inner();
        let feed = self.state.feed.read().await;

        // Search through all events for the matching ID
        let event = feed
            .latest(usize::MAX)
            .into_iter()
            .find(|e| e.id == req.event_id)
            .ok_or_else(|| Status::not_found("event not found"))?;

        Ok(Response::new(event_to_proto(event)))
    }

    async fn delete_event(
        &self,
        request: Request<pb::DeleteEventRequest>,
    ) -> Result<Response<pb::DeleteEventResponse>, Status> {
        let req = request.into_inner();
        let mut feed = self.state.feed.write().await;
        let deleted = feed.remove(&req.event_id);
        Ok(Response::new(pb::DeleteEventResponse { deleted }))
    }

    async fn follow(
        &self,
        request: Request<pb::FollowRequest>,
    ) -> Result<Response<pb::FollowResponse>, Status> {
        let req = request.into_inner();
        let mut graph = self.state.follow_graph.write().await;
        let success = graph.follow(&req.follower_did, &req.target_did);
        Ok(Response::new(pb::FollowResponse { success }))
    }

    async fn unfollow(
        &self,
        request: Request<pb::UnfollowRequest>,
    ) -> Result<Response<pb::UnfollowResponse>, Status> {
        let req = request.into_inner();
        let mut graph = self.state.follow_graph.write().await;
        let success = graph.unfollow(&req.follower_did, &req.target_did);
        Ok(Response::new(pb::UnfollowResponse { success }))
    }

    async fn get_timeline(
        &self,
        request: Request<pb::GetTimelineRequest>,
    ) -> Result<Response<pb::GetFeedResponse>, Status> {
        let req = request.into_inner();
        let limit = if req.limit == 0 {
            50
        } else {
            req.limit as usize
        };

        let graph = self.state.follow_graph.read().await;
        let following: Vec<String> = graph
            .following_of(&req.did)
            .iter()
            .map(|s| s.to_string())
            .collect();
        drop(graph);

        let feed = self.state.feed.read().await;
        let events: Vec<pb::EventMessage> = feed
            .timeline(&following, limit)
            .iter()
            .map(|e| event_to_proto(e))
            .collect();

        let count = events.len() as u32;
        Ok(Response::new(pb::GetFeedResponse { events, count }))
    }
}

// ---- Identity Service ----

pub struct NousIdentityService {
    _state: Arc<AppState>,
}

impl NousIdentityService {
    pub fn new(state: Arc<AppState>) -> Self {
        Self { _state: state }
    }
}

#[tonic::async_trait]
impl IdentityService for NousIdentityService {
    async fn resolve_document(
        &self,
        request: Request<pb::ResolveDocumentRequest>,
    ) -> Result<Response<pb::ResolveDocumentResponse>, Status> {
        let req = request.into_inner();

        // Validate DID format
        if !req.did.starts_with("did:key:z") {
            return Err(Status::invalid_argument("invalid DID format"));
        }

        // For now, return a minimal document. In production this would
        // resolve from the DHT or local store.
        let doc_json = serde_json::json!({
            "@context": ["https://www.w3.org/ns/did/v1"],
            "id": req.did,
        })
        .to_string();

        Ok(Response::new(pb::ResolveDocumentResponse {
            document_json: doc_json,
        }))
    }
}

// ---- Governance Service ----

pub struct NousGovernanceService {
    state: Arc<AppState>,
}

impl NousGovernanceService {
    pub fn new(state: Arc<AppState>) -> Self {
        Self { state }
    }
}

fn dao_to_proto(dao: &nous_governance::Dao) -> pb::DaoMessage {
    pb::DaoMessage {
        id: dao.id.clone(),
        name: dao.name.clone(),
        description: dao.description.clone(),
        founder_did: dao.founder_did.clone(),
        member_count: dao.member_count() as u32,
        created_at: dao.created_at.to_rfc3339(),
    }
}

fn proposal_to_proto(p: &nous_governance::Proposal) -> pb::ProposalMessage {
    pb::ProposalMessage {
        id: p.id.clone(),
        dao_id: p.dao_id.clone(),
        title: p.title.clone(),
        description: p.description.clone(),
        proposer_did: p.proposer_did.clone(),
        status: format!("{:?}", p.status),
        voting_starts: p.voting_starts.to_rfc3339(),
        voting_ends: p.voting_ends.to_rfc3339(),
        created_at: p.created_at.to_rfc3339(),
    }
}

#[tonic::async_trait]
impl GovernanceService for NousGovernanceService {
    async fn create_dao(
        &self,
        request: Request<pb::CreateDaoRequest>,
    ) -> Result<Response<pb::DaoMessage>, Status> {
        let req = request.into_inner();
        if req.name.is_empty() {
            return Err(Status::invalid_argument("name must not be empty"));
        }
        let dao = nous_governance::Dao::create(&req.founder_did, &req.name, &req.description);
        let msg = dao_to_proto(&dao);
        let mut daos = self.state.daos.write().await;
        daos.insert(dao.id.clone(), dao);
        Ok(Response::new(msg))
    }

    async fn get_dao(
        &self,
        request: Request<pb::GetDaoRequest>,
    ) -> Result<Response<pb::DaoMessage>, Status> {
        let req = request.into_inner();
        let daos = self.state.daos.read().await;
        let dao = daos
            .get(&req.dao_id)
            .ok_or_else(|| Status::not_found("DAO not found"))?;
        Ok(Response::new(dao_to_proto(dao)))
    }

    async fn list_daos(
        &self,
        _request: Request<pb::ListDaosRequest>,
    ) -> Result<Response<pb::ListDaosResponse>, Status> {
        let daos = self.state.daos.read().await;
        let list: Vec<pb::DaoMessage> = daos.values().map(dao_to_proto).collect();
        Ok(Response::new(pb::ListDaosResponse { daos: list }))
    }

    async fn list_proposals(
        &self,
        request: Request<pb::ListProposalsRequest>,
    ) -> Result<Response<pb::ListProposalsResponse>, Status> {
        let req = request.into_inner();
        let proposals = self.state.proposals.read().await;
        let limit = if req.limit == 0 {
            50
        } else {
            req.limit as usize
        };

        let list: Vec<pb::ProposalMessage> = proposals
            .values()
            .filter(|p| req.dao_id.is_empty() || p.dao_id == req.dao_id)
            .take(limit)
            .map(proposal_to_proto)
            .collect();
        Ok(Response::new(pb::ListProposalsResponse { proposals: list }))
    }

    async fn get_proposal(
        &self,
        request: Request<pb::GetProposalRequest>,
    ) -> Result<Response<pb::ProposalMessage>, Status> {
        let req = request.into_inner();
        let proposals = self.state.proposals.read().await;
        let p = proposals
            .get(&req.proposal_id)
            .ok_or_else(|| Status::not_found("proposal not found"))?;
        Ok(Response::new(proposal_to_proto(p)))
    }

    async fn get_tally(
        &self,
        request: Request<pb::GetTallyRequest>,
    ) -> Result<Response<pb::VoteResultMessage>, Status> {
        let req = request.into_inner();
        let tallies = self.state.tallies.read().await;
        let tally = tallies
            .get(&req.proposal_id)
            .ok_or_else(|| Status::not_found("tally not found"))?;

        let result = tally.tally(100);
        Ok(Response::new(pb::VoteResultMessage {
            proposal_id: result.proposal_id,
            votes_for: result.votes_for,
            votes_against: result.votes_against,
            votes_abstain: result.votes_abstain,
            total_voters: result.total_voters as u32,
            passed: result.passed,
        }))
    }
}

// ---- Marketplace Service ----

pub struct NousMarketplaceService {
    state: Arc<AppState>,
}

impl NousMarketplaceService {
    pub fn new(state: Arc<AppState>) -> Self {
        Self { state }
    }
}

fn listing_to_proto(l: &nous_marketplace::Listing) -> pb::ListingMessage {
    pb::ListingMessage {
        id: l.id.clone(),
        seller_did: l.seller_did.clone(),
        title: l.title.clone(),
        description: l.description.clone(),
        category: format!("{:?}", l.category),
        price_token: l.price_token.clone(),
        price_amount: l.price_amount.to_string(),
        status: format!("{:?}", l.status),
        tags: l.tags.clone(),
        created_at: l.created_at.to_rfc3339(),
    }
}

#[tonic::async_trait]
impl MarketplaceService for NousMarketplaceService {
    async fn create_listing(
        &self,
        request: Request<pb::CreateListingRequest>,
    ) -> Result<Response<pb::ListingMessage>, Status> {
        let req = request.into_inner();
        let category = match req.category.to_lowercase().as_str() {
            "physical" => nous_marketplace::ListingCategory::Physical,
            "digital" => nous_marketplace::ListingCategory::Digital,
            "service" => nous_marketplace::ListingCategory::Service,
            "nft" => nous_marketplace::ListingCategory::NFT,
            "data" => nous_marketplace::ListingCategory::Data,
            _ => nous_marketplace::ListingCategory::Other,
        };

        let price: u128 = req
            .price_amount
            .parse()
            .map_err(|_| Status::invalid_argument("invalid price_amount"))?;

        let mut listing = nous_marketplace::Listing::new(
            &req.seller_did,
            &req.title,
            &req.description,
            category,
            &req.price_token,
            price,
        )
        .map_err(|e| Status::invalid_argument(e.to_string()))?;

        for tag in &req.tags {
            listing = listing.with_tag(tag);
        }

        let msg = listing_to_proto(&listing);
        let mut listings = self.state.listings.write().await;
        listings.insert(listing.id.clone(), listing);
        Ok(Response::new(msg))
    }

    async fn get_listing(
        &self,
        request: Request<pb::GetListingRequest>,
    ) -> Result<Response<pb::ListingMessage>, Status> {
        let req = request.into_inner();
        let listings = self.state.listings.read().await;
        let l = listings
            .get(&req.listing_id)
            .ok_or_else(|| Status::not_found("listing not found"))?;
        Ok(Response::new(listing_to_proto(l)))
    }

    async fn search_listings(
        &self,
        request: Request<pb::SearchListingsRequest>,
    ) -> Result<Response<pb::SearchListingsResponse>, Status> {
        let req = request.into_inner();
        let listings = self.state.listings.read().await;
        let limit = if req.limit == 0 {
            50
        } else {
            req.limit as usize
        };

        let results: Vec<pb::ListingMessage> = listings
            .values()
            .filter(|l| {
                let text = req.query.is_empty() || l.matches_search(&req.query);
                let cat = req.category.is_empty()
                    || format!("{:?}", l.category).to_lowercase() == req.category.to_lowercase();
                text && cat
            })
            .take(limit)
            .map(listing_to_proto)
            .collect();

        let count = results.len() as u32;
        Ok(Response::new(pb::SearchListingsResponse {
            listings: results,
            count,
        }))
    }

    async fn create_review(
        &self,
        request: Request<pb::CreateReviewRequest>,
    ) -> Result<Response<pb::CreateReviewResponse>, Status> {
        let req = request.into_inner();
        let review = nous_marketplace::Review::new(
            &req.listing_id,
            &req.reviewer_did,
            &req.seller_did,
            req.rating as u8,
            &req.comment,
        )
        .map_err(|e| Status::invalid_argument(e.to_string()))?;

        let review_id = review.id.clone();
        let mut reviews = self.state.reviews.write().await;
        reviews.insert(review.id.clone(), review);

        Ok(Response::new(pb::CreateReviewResponse {
            success: true,
            review_id,
        }))
    }

    async fn get_seller_rating(
        &self,
        request: Request<pb::GetSellerRatingRequest>,
    ) -> Result<Response<pb::SellerRatingMessage>, Status> {
        let req = request.into_inner();
        let reviews = self.state.reviews.read().await;
        let seller_reviews: Vec<nous_marketplace::Review> = reviews
            .values()
            .filter(|r| r.seller_did == req.seller_did)
            .cloned()
            .collect();

        let rating = nous_marketplace::SellerRating::compute(&req.seller_did, &seller_reviews);
        let trusted = rating.is_trusted();
        Ok(Response::new(pb::SellerRatingMessage {
            seller_did: rating.seller_did,
            total_reviews: rating.total_reviews,
            average_rating: rating.average_rating,
            verified_reviews: rating.verified_reviews,
            trusted,
        }))
    }
}

fn event_to_proto(event: &nous_social::SignedEvent) -> pb::EventMessage {
    pb::EventMessage {
        id: event.id.clone(),
        pubkey: event.pubkey.clone(),
        created_at: event.created_at.timestamp(),
        kind: event.kind.as_u32(),
        content: event.content.clone(),
        hashtags: event.hashtags().iter().map(|s| s.to_string()).collect(),
        referenced_events: event
            .referenced_events()
            .iter()
            .map(|s| s.to_string())
            .collect(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::ApiConfig;

    fn test_state() -> Arc<AppState> {
        AppState::new(ApiConfig::default())
    }

    #[tokio::test]
    async fn grpc_health() {
        let svc = NousNodeService::new(test_state());
        let resp = svc
            .health(Request::new(pb::HealthRequest {}))
            .await
            .unwrap();
        assert_eq!(resp.into_inner().status, "ok");
    }

    #[tokio::test]
    async fn grpc_node_info() {
        let svc = NousNodeService::new(test_state());
        let resp = svc
            .node_info(Request::new(pb::NodeInfoRequest {}))
            .await
            .unwrap();
        let info = resp.into_inner();
        assert_eq!(info.protocol, "nous/0.1");
        assert!(!info.features.is_empty());
    }

    #[tokio::test]
    async fn grpc_create_post() {
        let state = test_state();
        let svc = NousSocialService::new(state.clone());

        let resp = svc
            .create_post(Request::new(pb::CreatePostRequest {
                author_did: "did:key:ztest".into(),
                content: "hello grpc".into(),
                hashtags: vec!["nous".into()],
            }))
            .await
            .unwrap();

        assert!(!resp.into_inner().event_id.is_empty());

        let feed = state.feed.read().await;
        assert_eq!(feed.len(), 1);
    }

    #[tokio::test]
    async fn grpc_create_post_empty_content() {
        let svc = NousSocialService::new(test_state());
        let result = svc
            .create_post(Request::new(pb::CreatePostRequest {
                author_did: "did:key:ztest".into(),
                content: "".into(),
                hashtags: vec![],
            }))
            .await;
        assert!(result.is_err());
        assert_eq!(result.unwrap_err().code(), tonic::Code::InvalidArgument);
    }

    #[tokio::test]
    async fn grpc_get_feed() {
        let state = test_state();
        let svc = NousSocialService::new(state.clone());

        // Insert a post first
        svc.create_post(Request::new(pb::CreatePostRequest {
            author_did: "did:key:ztest".into(),
            content: "feed item".into(),
            hashtags: vec![],
        }))
        .await
        .unwrap();

        let resp = svc
            .get_feed(Request::new(pb::GetFeedRequest {
                author: "".into(),
                kind: 0,
                hashtag: "".into(),
                limit: 10,
            }))
            .await
            .unwrap();

        let feed = resp.into_inner();
        assert_eq!(feed.count, 1);
        assert_eq!(feed.events[0].content, "feed item");
    }

    #[tokio::test]
    async fn grpc_get_feed_by_author() {
        let state = test_state();
        let svc = NousSocialService::new(state.clone());

        svc.create_post(Request::new(pb::CreatePostRequest {
            author_did: "did:key:zalice".into(),
            content: "alice post".into(),
            hashtags: vec![],
        }))
        .await
        .unwrap();

        svc.create_post(Request::new(pb::CreatePostRequest {
            author_did: "did:key:zbob".into(),
            content: "bob post".into(),
            hashtags: vec![],
        }))
        .await
        .unwrap();

        let resp = svc
            .get_feed(Request::new(pb::GetFeedRequest {
                author: "did:key:zalice".into(),
                kind: 0,
                hashtag: "".into(),
                limit: 10,
            }))
            .await
            .unwrap();

        let feed = resp.into_inner();
        assert_eq!(feed.count, 1);
        assert_eq!(feed.events[0].pubkey, "did:key:zalice");
    }

    #[tokio::test]
    async fn grpc_delete_event() {
        let state = test_state();
        let svc = NousSocialService::new(state.clone());

        let resp = svc
            .create_post(Request::new(pb::CreatePostRequest {
                author_did: "did:key:ztest".into(),
                content: "to delete".into(),
                hashtags: vec![],
            }))
            .await
            .unwrap();
        let event_id = resp.into_inner().event_id;

        let del = svc
            .delete_event(Request::new(pb::DeleteEventRequest {
                event_id: event_id.clone(),
            }))
            .await
            .unwrap();
        assert!(del.into_inner().deleted);

        let feed = state.feed.read().await;
        assert_eq!(feed.len(), 0);
    }

    #[tokio::test]
    async fn grpc_follow_unfollow() {
        let state = test_state();
        let svc = NousSocialService::new(state.clone());

        let resp = svc
            .follow(Request::new(pb::FollowRequest {
                follower_did: "did:key:zalice".into(),
                target_did: "did:key:zbob".into(),
            }))
            .await
            .unwrap();
        assert!(resp.into_inner().success);

        let graph = state.follow_graph.read().await;
        assert!(graph.is_following("did:key:zalice", "did:key:zbob"));
        drop(graph);

        let resp = svc
            .unfollow(Request::new(pb::UnfollowRequest {
                follower_did: "did:key:zalice".into(),
                target_did: "did:key:zbob".into(),
            }))
            .await
            .unwrap();
        assert!(resp.into_inner().success);
    }

    #[tokio::test]
    async fn grpc_get_event_not_found() {
        let svc = NousSocialService::new(test_state());
        let result = svc
            .get_event(Request::new(pb::GetEventRequest {
                event_id: "nonexistent".into(),
            }))
            .await;
        assert!(result.is_err());
        assert_eq!(result.unwrap_err().code(), tonic::Code::NotFound);
    }

    #[tokio::test]
    async fn grpc_resolve_document() {
        let svc = NousIdentityService::new(test_state());
        let resp = svc
            .resolve_document(Request::new(pb::ResolveDocumentRequest {
                did: "did:key:ztest123".into(),
            }))
            .await
            .unwrap();
        let doc = resp.into_inner().document_json;
        assert!(doc.contains("did:key:ztest123"));
    }

    #[tokio::test]
    async fn grpc_resolve_document_invalid_did() {
        let svc = NousIdentityService::new(test_state());
        let result = svc
            .resolve_document(Request::new(pb::ResolveDocumentRequest {
                did: "invalid".into(),
            }))
            .await;
        assert!(result.is_err());
    }

    // ── Governance gRPC tests ────────────────────────────────

    #[tokio::test]
    async fn grpc_create_dao() {
        let state = test_state();
        let svc = NousGovernanceService::new(state.clone());

        let resp = svc
            .create_dao(Request::new(pb::CreateDaoRequest {
                founder_did: "did:key:zfounder".into(),
                name: "TestDAO".into(),
                description: "A test DAO".into(),
            }))
            .await
            .unwrap();

        let dao = resp.into_inner();
        assert_eq!(dao.name, "TestDAO");
        assert_eq!(dao.member_count, 1);
    }

    #[tokio::test]
    async fn grpc_list_daos() {
        let state = test_state();
        let svc = NousGovernanceService::new(state.clone());

        svc.create_dao(Request::new(pb::CreateDaoRequest {
            founder_did: "did:key:za".into(),
            name: "Alpha".into(),
            description: "first".into(),
        }))
        .await
        .unwrap();

        let resp = svc
            .list_daos(Request::new(pb::ListDaosRequest {}))
            .await
            .unwrap();
        assert_eq!(resp.into_inner().daos.len(), 1);
    }

    #[tokio::test]
    async fn grpc_get_dao() {
        let state = test_state();
        let svc = NousGovernanceService::new(state.clone());

        let resp = svc
            .create_dao(Request::new(pb::CreateDaoRequest {
                founder_did: "did:key:za".into(),
                name: "GetMe".into(),
                description: "test".into(),
            }))
            .await
            .unwrap();
        let dao_id = resp.into_inner().id;

        let resp = svc
            .get_dao(Request::new(pb::GetDaoRequest {
                dao_id: dao_id.clone(),
            }))
            .await
            .unwrap();
        assert_eq!(resp.into_inner().name, "GetMe");
    }

    #[tokio::test]
    async fn grpc_get_dao_not_found() {
        let svc = NousGovernanceService::new(test_state());
        let result = svc
            .get_dao(Request::new(pb::GetDaoRequest {
                dao_id: "nonexistent".into(),
            }))
            .await;
        assert!(result.is_err());
        assert_eq!(result.unwrap_err().code(), tonic::Code::NotFound);
    }

    #[tokio::test]
    async fn grpc_empty_proposals() {
        let svc = NousGovernanceService::new(test_state());
        let resp = svc
            .list_proposals(Request::new(pb::ListProposalsRequest {
                dao_id: "".into(),
                limit: 0,
            }))
            .await
            .unwrap();
        assert!(resp.into_inner().proposals.is_empty());
    }

    // ── Marketplace gRPC tests ────────────────────────────────

    #[tokio::test]
    async fn grpc_create_listing() {
        let state = test_state();
        let svc = NousMarketplaceService::new(state.clone());

        let resp = svc
            .create_listing(Request::new(pb::CreateListingRequest {
                seller_did: "did:key:zseller".into(),
                title: "Widget".into(),
                description: "A nice widget".into(),
                category: "physical".into(),
                price_token: "ETH".into(),
                price_amount: "1000".into(),
                tags: vec!["widget".into()],
            }))
            .await
            .unwrap();

        let listing = resp.into_inner();
        assert_eq!(listing.title, "Widget");
        assert_eq!(listing.category, "Physical");
    }

    #[tokio::test]
    async fn grpc_get_listing() {
        let state = test_state();
        let svc = NousMarketplaceService::new(state.clone());

        let resp = svc
            .create_listing(Request::new(pb::CreateListingRequest {
                seller_did: "did:key:zs".into(),
                title: "Book".into(),
                description: "A book".into(),
                category: "digital".into(),
                price_token: "USDC".into(),
                price_amount: "500".into(),
                tags: vec![],
            }))
            .await
            .unwrap();
        let listing_id = resp.into_inner().id;

        let resp = svc
            .get_listing(Request::new(pb::GetListingRequest {
                listing_id: listing_id.clone(),
            }))
            .await
            .unwrap();
        assert_eq!(resp.into_inner().title, "Book");
    }

    #[tokio::test]
    async fn grpc_search_listings() {
        let state = test_state();
        let svc = NousMarketplaceService::new(state.clone());

        svc.create_listing(Request::new(pb::CreateListingRequest {
            seller_did: "did:key:zs".into(),
            title: "Rust Handbook".into(),
            description: "Learn Rust programming".into(),
            category: "digital".into(),
            price_token: "USDC".into(),
            price_amount: "2000".into(),
            tags: vec!["rust".into()],
        }))
        .await
        .unwrap();

        let resp = svc
            .search_listings(Request::new(pb::SearchListingsRequest {
                query: "Rust".into(),
                category: "".into(),
                limit: 10,
            }))
            .await
            .unwrap();
        assert_eq!(resp.into_inner().count, 1);
    }

    #[tokio::test]
    async fn grpc_create_review_and_rating() {
        let state = test_state();
        let svc = NousMarketplaceService::new(state.clone());

        let resp = svc
            .create_review(Request::new(pb::CreateReviewRequest {
                listing_id: "listing-1".into(),
                reviewer_did: "did:key:zbuyer".into(),
                seller_did: "did:key:zseller".into(),
                rating: 5,
                comment: "Excellent".into(),
            }))
            .await
            .unwrap();
        assert!(resp.into_inner().success);

        let resp = svc
            .get_seller_rating(Request::new(pb::GetSellerRatingRequest {
                seller_did: "did:key:zseller".into(),
            }))
            .await
            .unwrap();
        let rating = resp.into_inner();
        assert_eq!(rating.total_reviews, 1);
        assert_eq!(rating.average_rating, 5.0);
    }

    #[tokio::test]
    async fn grpc_timeline() {
        let state = test_state();
        let svc = NousSocialService::new(state.clone());

        // Create posts from two users
        svc.create_post(Request::new(pb::CreatePostRequest {
            author_did: "did:key:zalice".into(),
            content: "alice post".into(),
            hashtags: vec![],
        }))
        .await
        .unwrap();

        svc.create_post(Request::new(pb::CreatePostRequest {
            author_did: "did:key:zbob".into(),
            content: "bob post".into(),
            hashtags: vec![],
        }))
        .await
        .unwrap();

        // Follow alice only
        svc.follow(Request::new(pb::FollowRequest {
            follower_did: "did:key:zme".into(),
            target_did: "did:key:zalice".into(),
        }))
        .await
        .unwrap();

        let resp = svc
            .get_timeline(Request::new(pb::GetTimelineRequest {
                did: "did:key:zme".into(),
                limit: 10,
            }))
            .await
            .unwrap();

        let timeline = resp.into_inner();
        assert_eq!(timeline.count, 1);
        assert_eq!(timeline.events[0].pubkey, "did:key:zalice");
    }
}
