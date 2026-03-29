//! GraphQL schema and resolvers for the Nous API.
//!
//! Provides a full GraphQL endpoint alongside the REST API,
//! sharing the same application state.

use std::sync::Arc;

use async_graphql::{Context, InputObject, Object, Schema, SimpleObject, Subscription};
use axum::Json;
use axum::extract::State;
use futures::Stream;
use tokio_stream::StreamExt;

use nous_governance::{Dao, Proposal};
use nous_marketplace::{Listing, ListingCategory, Review, SellerRating};
use nous_social::{EventKind, PostBuilder, SignedEvent};

use crate::state::AppState;

// ── GraphQL Types ──────────────────────────────────────────────

#[derive(SimpleObject)]
pub struct EventNode {
    pub id: String,
    pub pubkey: String,
    pub kind: u32,
    pub content: String,
    pub created_at: String,
    pub hashtags: Vec<String>,
    pub referenced_events: Vec<String>,
}

impl EventNode {
    fn from_signed(event: &SignedEvent) -> Self {
        Self {
            id: event.id.clone(),
            pubkey: event.pubkey.clone(),
            kind: event.kind.as_u32(),
            content: event.content.clone(),
            created_at: event.created_at.to_rfc3339(),
            hashtags: event.hashtags().into_iter().map(String::from).collect(),
            referenced_events: event
                .referenced_events()
                .into_iter()
                .map(String::from)
                .collect(),
        }
    }
}

#[derive(SimpleObject)]
pub struct FollowInfo {
    pub following_count: usize,
    pub follower_count: usize,
    pub following: Vec<String>,
}

#[derive(SimpleObject)]
pub struct NodeInfoGql {
    pub protocol: String,
    pub version: String,
    pub features: Vec<String>,
}

#[derive(SimpleObject)]
pub struct FeedResult {
    pub events: Vec<EventNode>,
    pub count: usize,
}

#[derive(SimpleObject)]
pub struct MutationResult {
    pub success: bool,
    pub message: String,
}

#[derive(InputObject)]
pub struct CreatePostInput {
    pub author_did: String,
    pub content: String,
    pub reply_to: Option<String>,
    pub hashtags: Option<Vec<String>>,
}

#[derive(InputObject)]
pub struct FollowInput {
    pub follower_did: String,
    pub target_did: String,
}

// ── Governance GraphQL Types ──────────────────────────────────

#[derive(SimpleObject)]
pub struct DaoNode {
    pub id: String,
    pub name: String,
    pub description: String,
    pub founder: String,
    pub member_count: usize,
    pub created_at: String,
}

impl DaoNode {
    fn from_dao(dao: &Dao) -> Self {
        Self {
            id: dao.id.clone(),
            name: dao.name.clone(),
            description: dao.description.clone(),
            founder: dao.founder_did.clone(),
            member_count: dao.member_count(),
            created_at: dao.created_at.to_rfc3339(),
        }
    }
}

#[derive(SimpleObject)]
pub struct ProposalNode {
    pub id: String,
    pub dao_id: String,
    pub title: String,
    pub description: String,
    pub proposer: String,
    pub status: String,
    pub voting_starts: String,
    pub voting_ends: String,
    pub created_at: String,
}

impl ProposalNode {
    fn from_proposal(p: &Proposal) -> Self {
        let status = format!("{:?}", p.status);
        Self {
            id: p.id.clone(),
            dao_id: p.dao_id.clone(),
            title: p.title.clone(),
            description: p.description.clone(),
            proposer: p.proposer_did.clone(),
            status,
            voting_starts: p.voting_starts.to_rfc3339(),
            voting_ends: p.voting_ends.to_rfc3339(),
            created_at: p.created_at.to_rfc3339(),
        }
    }
}

#[derive(SimpleObject)]
pub struct VoteResultNode {
    pub proposal_id: String,
    pub votes_for: String,
    pub votes_against: String,
    pub votes_abstain: String,
    pub total_voters: usize,
    pub passed: bool,
}

#[derive(InputObject)]
pub struct CreateDaoInput {
    pub founder_did: String,
    pub name: String,
    pub description: String,
}

// ── Marketplace GraphQL Types ──────────────────────────────────

#[derive(SimpleObject)]
pub struct ListingNode {
    pub id: String,
    pub seller: String,
    pub title: String,
    pub description: String,
    pub category: String,
    pub price_token: String,
    pub price_amount: String,
    pub status: String,
    pub tags: Vec<String>,
    pub created_at: String,
}

impl ListingNode {
    fn from_listing(l: &Listing) -> Self {
        Self {
            id: l.id.clone(),
            seller: l.seller_did.clone(),
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
}

#[derive(SimpleObject)]
pub struct SellerRatingNode {
    pub seller_did: String,
    pub total_reviews: u32,
    pub average_rating: f64,
    pub verified_reviews: u32,
    pub trusted: bool,
}

impl SellerRatingNode {
    fn from_rating(r: &SellerRating) -> Self {
        Self {
            seller_did: r.seller_did.clone(),
            total_reviews: r.total_reviews,
            average_rating: r.average_rating,
            verified_reviews: r.verified_reviews,
            trusted: r.is_trusted(),
        }
    }
}

#[derive(InputObject)]
pub struct CreateListingInput {
    pub seller_did: String,
    pub title: String,
    pub description: String,
    pub category: String,
    pub price_token: String,
    pub price_amount: String,
    pub tags: Option<Vec<String>>,
}

#[derive(InputObject)]
pub struct CreateReviewInput {
    pub listing_id: String,
    pub reviewer_did: String,
    pub seller_did: String,
    pub rating: u8,
    pub comment: String,
}

// ── Query Root ──────────────────────────────────────────────────

pub struct QueryRoot;

#[Object]
impl QueryRoot {
    async fn health(&self) -> &str {
        "ok"
    }

    async fn node_info(&self) -> NodeInfoGql {
        NodeInfoGql {
            protocol: "nous".into(),
            version: env!("CARGO_PKG_VERSION").into(),
            features: vec![
                "identity".into(),
                "messaging".into(),
                "social".into(),
                "governance".into(),
                "payments".into(),
                "marketplace".into(),
            ],
        }
    }

    async fn feed(
        &self,
        ctx: &Context<'_>,
        author: Option<String>,
        kind: Option<u32>,
        tag: Option<String>,
        limit: Option<usize>,
    ) -> async_graphql::Result<FeedResult> {
        let state = ctx.data::<Arc<AppState>>()?;
        let feed = state.feed.read().await;
        let limit = limit.unwrap_or(50).min(200);

        let events: Vec<EventNode> = if let Some(ref author) = author {
            feed.by_author(author)
                .into_iter()
                .take(limit)
                .map(EventNode::from_signed)
                .collect()
        } else if let Some(kind) = kind {
            feed.by_kind(EventKind::from(kind))
                .into_iter()
                .take(limit)
                .map(EventNode::from_signed)
                .collect()
        } else if let Some(ref tag) = tag {
            feed.by_hashtag(tag)
                .into_iter()
                .take(limit)
                .map(EventNode::from_signed)
                .collect()
        } else {
            feed.latest(limit)
                .into_iter()
                .map(EventNode::from_signed)
                .collect()
        };

        let count = events.len();
        Ok(FeedResult { events, count })
    }

    async fn event(
        &self,
        ctx: &Context<'_>,
        id: String,
    ) -> async_graphql::Result<Option<EventNode>> {
        let state = ctx.data::<Arc<AppState>>()?;
        let feed = state.feed.read().await;
        let found = feed
            .latest(10_000)
            .into_iter()
            .find(|e| e.id == id)
            .map(EventNode::from_signed);
        Ok(found)
    }

    async fn timeline(
        &self,
        ctx: &Context<'_>,
        did: String,
        limit: Option<usize>,
    ) -> async_graphql::Result<FeedResult> {
        let state = ctx.data::<Arc<AppState>>()?;
        let graph = state.follow_graph.read().await;
        let following: Vec<String> = graph
            .following_of(&did)
            .into_iter()
            .map(|s| s.to_string())
            .collect();
        drop(graph);

        let feed = state.feed.read().await;
        let limit = limit.unwrap_or(50).min(200);
        let events: Vec<EventNode> = feed
            .timeline(&following, limit)
            .into_iter()
            .map(EventNode::from_signed)
            .collect();

        let count = events.len();
        Ok(FeedResult { events, count })
    }

    async fn follow_info(
        &self,
        ctx: &Context<'_>,
        did: String,
    ) -> async_graphql::Result<FollowInfo> {
        let state = ctx.data::<Arc<AppState>>()?;
        let graph = state.follow_graph.read().await;

        Ok(FollowInfo {
            following_count: graph.following_count(&did),
            follower_count: graph.followers_count(&did),
            following: graph
                .following_of(&did)
                .into_iter()
                .map(|s| s.to_string())
                .collect(),
        })
    }

    // ── Governance Queries ─────────────────────────────────────

    async fn daos(&self, ctx: &Context<'_>) -> async_graphql::Result<Vec<DaoNode>> {
        let state = ctx.data::<Arc<AppState>>()?;
        let daos = state.daos.read().await;
        Ok(daos.values().map(DaoNode::from_dao).collect())
    }

    async fn dao(&self, ctx: &Context<'_>, id: String) -> async_graphql::Result<Option<DaoNode>> {
        let state = ctx.data::<Arc<AppState>>()?;
        let daos = state.daos.read().await;
        Ok(daos.get(&id).map(DaoNode::from_dao))
    }

    async fn proposals(
        &self,
        ctx: &Context<'_>,
        dao_id: Option<String>,
        limit: Option<usize>,
    ) -> async_graphql::Result<Vec<ProposalNode>> {
        let state = ctx.data::<Arc<AppState>>()?;
        let proposals = state.proposals.read().await;
        let limit = limit.unwrap_or(50).min(200);

        let result: Vec<ProposalNode> = proposals
            .values()
            .filter(|p| dao_id.as_ref().is_none_or(|d| &p.dao_id == d))
            .take(limit)
            .map(ProposalNode::from_proposal)
            .collect();
        Ok(result)
    }

    async fn proposal(
        &self,
        ctx: &Context<'_>,
        id: String,
    ) -> async_graphql::Result<Option<ProposalNode>> {
        let state = ctx.data::<Arc<AppState>>()?;
        let proposals = state.proposals.read().await;
        Ok(proposals.get(&id).map(ProposalNode::from_proposal))
    }

    async fn vote_tally(
        &self,
        ctx: &Context<'_>,
        proposal_id: String,
    ) -> async_graphql::Result<Option<VoteResultNode>> {
        let state = ctx.data::<Arc<AppState>>()?;
        let tallies = state.tallies.read().await;
        let result = tallies.get(&proposal_id).map(|t| {
            let r = t.tally(100);
            VoteResultNode {
                proposal_id: r.proposal_id,
                votes_for: r.votes_for.to_string(),
                votes_against: r.votes_against.to_string(),
                votes_abstain: r.votes_abstain.to_string(),
                total_voters: r.total_voters,
                passed: r.passed,
            }
        });
        Ok(result)
    }

    // ── Marketplace Queries ────────────────────────────────────

    async fn listings(
        &self,
        ctx: &Context<'_>,
        search: Option<String>,
        category: Option<String>,
        limit: Option<usize>,
    ) -> async_graphql::Result<Vec<ListingNode>> {
        let state = ctx.data::<Arc<AppState>>()?;
        let listings = state.listings.read().await;
        let limit = limit.unwrap_or(50).min(200);

        let result: Vec<ListingNode> = listings
            .values()
            .filter(|l| {
                let text_match = search.as_ref().is_none_or(|q| l.matches_search(q));
                let cat_match = category
                    .as_ref()
                    .is_none_or(|c| format!("{:?}", l.category).to_lowercase() == c.to_lowercase());
                text_match && cat_match
            })
            .take(limit)
            .map(ListingNode::from_listing)
            .collect();
        Ok(result)
    }

    async fn listing(
        &self,
        ctx: &Context<'_>,
        id: String,
    ) -> async_graphql::Result<Option<ListingNode>> {
        let state = ctx.data::<Arc<AppState>>()?;
        let listings = state.listings.read().await;
        Ok(listings.get(&id).map(ListingNode::from_listing))
    }

    async fn seller_rating(
        &self,
        ctx: &Context<'_>,
        seller_did: String,
    ) -> async_graphql::Result<SellerRatingNode> {
        let state = ctx.data::<Arc<AppState>>()?;
        let reviews = state.reviews.read().await;
        let seller_reviews: Vec<Review> = reviews
            .values()
            .filter(|r| r.seller_did == seller_did)
            .cloned()
            .collect();
        let rating = SellerRating::compute(&seller_did, &seller_reviews);
        Ok(SellerRatingNode::from_rating(&rating))
    }
}

// ── Mutation Root ──────────────────────────────────────────────

pub struct MutationRoot;

#[Object]
impl MutationRoot {
    async fn create_post(
        &self,
        ctx: &Context<'_>,
        input: CreatePostInput,
    ) -> async_graphql::Result<EventNode> {
        if input.content.is_empty() {
            return Err("content cannot be empty".into());
        }
        if input.content.len() > 10_000 {
            return Err("content exceeds maximum length".into());
        }

        let mut builder = PostBuilder::new(&input.author_did, &input.content);

        if let Some(ref reply_to) = input.reply_to {
            builder = builder.reply_to(reply_to);
        }
        if let Some(ref tags) = input.hashtags {
            for tag in tags {
                builder = builder.hashtag(tag);
            }
        }

        let event = builder.build();
        let node = EventNode::from_signed(&event);

        let state = ctx.data::<Arc<AppState>>()?;
        let mut feed = state.feed.write().await;
        feed.insert(event);

        Ok(node)
    }

    async fn follow(
        &self,
        ctx: &Context<'_>,
        input: FollowInput,
    ) -> async_graphql::Result<MutationResult> {
        let state = ctx.data::<Arc<AppState>>()?;
        let mut graph = state.follow_graph.write().await;
        let added = graph.follow(&input.follower_did, &input.target_did);

        Ok(MutationResult {
            success: added,
            message: if added {
                format!("{} now follows {}", input.follower_did, input.target_did)
            } else {
                "already following".into()
            },
        })
    }

    async fn unfollow(
        &self,
        ctx: &Context<'_>,
        input: FollowInput,
    ) -> async_graphql::Result<MutationResult> {
        let state = ctx.data::<Arc<AppState>>()?;
        let mut graph = state.follow_graph.write().await;
        let removed = graph.unfollow(&input.follower_did, &input.target_did);

        Ok(MutationResult {
            success: removed,
            message: if removed {
                format!("{} unfollowed {}", input.follower_did, input.target_did)
            } else {
                "was not following".into()
            },
        })
    }

    async fn delete_event(
        &self,
        ctx: &Context<'_>,
        id: String,
    ) -> async_graphql::Result<MutationResult> {
        let state = ctx.data::<Arc<AppState>>()?;
        let mut feed = state.feed.write().await;
        let removed = feed.remove(&id);

        Ok(MutationResult {
            success: removed,
            message: if removed {
                format!("deleted {id}")
            } else {
                format!("event {id} not found")
            },
        })
    }

    // ── Governance Mutations ───────────────────────────────────

    async fn create_dao(
        &self,
        ctx: &Context<'_>,
        input: CreateDaoInput,
    ) -> async_graphql::Result<DaoNode> {
        let dao = Dao::create(&input.founder_did, &input.name, &input.description);
        let node = DaoNode::from_dao(&dao);

        let state = ctx.data::<Arc<AppState>>()?;
        let mut daos = state.daos.write().await;
        daos.insert(dao.id.clone(), dao);
        Ok(node)
    }

    async fn add_dao_member(
        &self,
        ctx: &Context<'_>,
        dao_id: String,
        did: String,
    ) -> async_graphql::Result<MutationResult> {
        let state = ctx.data::<Arc<AppState>>()?;
        let mut daos = state.daos.write().await;
        let dao = daos
            .get_mut(&dao_id)
            .ok_or_else(|| async_graphql::Error::new(format!("DAO {dao_id} not found")))?;

        match dao.add_member(&did) {
            Ok(()) => Ok(MutationResult {
                success: true,
                message: format!("{did} added to {dao_id}"),
            }),
            Err(e) => Ok(MutationResult {
                success: false,
                message: e.to_string(),
            }),
        }
    }

    // ── Marketplace Mutations ──────────────────────────────────

    async fn create_listing(
        &self,
        ctx: &Context<'_>,
        input: CreateListingInput,
    ) -> async_graphql::Result<ListingNode> {
        let category = match input.category.to_lowercase().as_str() {
            "physical" => ListingCategory::Physical,
            "digital" => ListingCategory::Digital,
            "service" => ListingCategory::Service,
            "nft" => ListingCategory::NFT,
            "data" => ListingCategory::Data,
            _ => ListingCategory::Other,
        };

        let price: u128 = input
            .price_amount
            .parse()
            .map_err(|_| async_graphql::Error::new("invalid price_amount"))?;

        let mut listing = Listing::new(
            &input.seller_did,
            &input.title,
            &input.description,
            category,
            &input.price_token,
            price,
        )
        .map_err(|e| async_graphql::Error::new(e.to_string()))?;

        if let Some(tags) = input.tags {
            for tag in tags {
                listing = listing.with_tag(tag);
            }
        }

        let node = ListingNode::from_listing(&listing);
        let state = ctx.data::<Arc<AppState>>()?;
        let mut listings = state.listings.write().await;
        listings.insert(listing.id.clone(), listing);
        Ok(node)
    }

    async fn create_review(
        &self,
        ctx: &Context<'_>,
        input: CreateReviewInput,
    ) -> async_graphql::Result<MutationResult> {
        let review = Review::new(
            &input.listing_id,
            &input.reviewer_did,
            &input.seller_did,
            input.rating,
            &input.comment,
        )
        .map_err(|e| async_graphql::Error::new(e.to_string()))?;

        let state = ctx.data::<Arc<AppState>>()?;
        let mut reviews = state.reviews.write().await;
        reviews.insert(review.id.clone(), review);

        Ok(MutationResult {
            success: true,
            message: "review created".into(),
        })
    }

    async fn cancel_listing(
        &self,
        ctx: &Context<'_>,
        listing_id: String,
        seller_did: String,
    ) -> async_graphql::Result<MutationResult> {
        let state = ctx.data::<Arc<AppState>>()?;
        let mut listings = state.listings.write().await;
        let listing = listings
            .get_mut(&listing_id)
            .ok_or_else(|| async_graphql::Error::new(format!("listing {listing_id} not found")))?;

        match listing.cancel(&seller_did) {
            Ok(()) => Ok(MutationResult {
                success: true,
                message: format!("listing {listing_id} cancelled"),
            }),
            Err(e) => Ok(MutationResult {
                success: false,
                message: e.to_string(),
            }),
        }
    }
}

// ── Subscriptions ─────────────────────────────────────────────

#[derive(SimpleObject)]
pub struct PostEvent {
    pub id: String,
    pub author: String,
    pub content: String,
}

#[derive(SimpleObject)]
pub struct MessageEvent {
    pub channel_id: String,
    pub sender: String,
    pub content: String,
}

#[derive(SimpleObject)]
pub struct VoteEvent {
    pub proposal_id: String,
    pub voter: String,
}

pub struct SubscriptionRoot;

#[Subscription]
impl SubscriptionRoot {
    /// Subscribe to new posts in the social feed.
    async fn on_new_post(&self, ctx: &Context<'_>) -> impl Stream<Item = PostEvent> {
        let state = ctx.data_unchecked::<Arc<AppState>>();
        let rx = state.events.subscribe();
        tokio_stream::wrappers::BroadcastStream::new(rx).filter_map(|result| match result {
            Ok(crate::state::RealtimeEvent::NewPost {
                id,
                author,
                content,
            }) => Some(PostEvent {
                id,
                author,
                content,
            }),
            _ => None,
        })
    }

    /// Subscribe to new messages in any channel.
    async fn on_new_message(&self, ctx: &Context<'_>) -> impl Stream<Item = MessageEvent> {
        let state = ctx.data_unchecked::<Arc<AppState>>();
        let rx = state.events.subscribe();
        tokio_stream::wrappers::BroadcastStream::new(rx).filter_map(|result| match result {
            Ok(crate::state::RealtimeEvent::NewMessage {
                channel_id,
                sender,
                content,
            }) => Some(MessageEvent {
                channel_id,
                sender,
                content,
            }),
            _ => None,
        })
    }

    /// Subscribe to votes cast on proposals.
    async fn on_vote_cast(&self, ctx: &Context<'_>) -> impl Stream<Item = VoteEvent> {
        let state = ctx.data_unchecked::<Arc<AppState>>();
        let rx = state.events.subscribe();
        tokio_stream::wrappers::BroadcastStream::new(rx).filter_map(|result| match result {
            Ok(crate::state::RealtimeEvent::VoteCast { proposal_id, voter }) => {
                Some(VoteEvent { proposal_id, voter })
            }
            _ => None,
        })
    }
}

// ── Schema ────────────────────────────────────────────────────

pub type NousSchema = Schema<QueryRoot, MutationRoot, SubscriptionRoot>;

pub fn build_schema(state: Arc<AppState>) -> NousSchema {
    Schema::build(QueryRoot, MutationRoot, SubscriptionRoot)
        .data(state)
        .finish()
}

/// Axum handler for GraphQL requests.
pub async fn graphql_handler(
    State(state): State<Arc<AppState>>,
    Json(request): Json<async_graphql::Request>,
) -> Json<async_graphql::Response> {
    let schema = build_schema(state);
    Json(schema.execute(request).await)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::ApiConfig;

    fn test_schema() -> NousSchema {
        let state = AppState::new(ApiConfig::default());
        build_schema(state)
    }

    #[tokio::test]
    async fn health_query() {
        let schema = test_schema();
        let res = schema.execute("{ health }").await;
        assert!(res.errors.is_empty());
        assert_eq!(res.data.to_string(), r#"{health: "ok"}"#);
    }

    #[tokio::test]
    async fn node_info_query() {
        let schema = test_schema();
        let res = schema
            .execute("{ nodeInfo { protocol version features } }")
            .await;
        assert!(res.errors.is_empty());
        let data = res.data.into_json().unwrap();
        assert_eq!(data["nodeInfo"]["protocol"], "nous");
    }

    #[tokio::test]
    async fn empty_feed_query() {
        let schema = test_schema();
        let res = schema
            .execute("{ feed { events { id content } count } }")
            .await;
        assert!(res.errors.is_empty());
        let data = res.data.into_json().unwrap();
        assert_eq!(data["feed"]["count"], 0);
    }

    #[tokio::test]
    async fn create_post_mutation() {
        let schema = test_schema();
        let res = schema
            .execute(
                r#"mutation {
                    createPost(input: {
                        authorDid: "did:key:ztest",
                        content: "hello from graphql"
                    }) {
                        id
                        content
                        pubkey
                    }
                }"#,
            )
            .await;
        assert!(res.errors.is_empty(), "errors: {:?}", res.errors);
        let data = res.data.into_json().unwrap();
        assert_eq!(data["createPost"]["content"], "hello from graphql");
    }

    #[tokio::test]
    async fn create_post_then_query_feed() {
        let state = AppState::new(ApiConfig::default());
        let schema = build_schema(state);

        let _ = schema
            .execute(
                r#"mutation {
                    createPost(input: {
                        authorDid: "did:key:zgql",
                        content: "graphql post",
                        hashtags: ["test"]
                    }) { id }
                }"#,
            )
            .await;

        let res = schema
            .execute("{ feed { events { id content hashtags } count } }")
            .await;
        assert!(res.errors.is_empty());
        let data = res.data.into_json().unwrap();
        assert_eq!(data["feed"]["count"], 1);
        assert_eq!(data["feed"]["events"][0]["content"], "graphql post");
    }

    #[tokio::test]
    async fn follow_and_timeline() {
        let state = AppState::new(ApiConfig::default());
        let schema = build_schema(state);

        // Bob posts
        let _ = schema
            .execute(
                r#"mutation {
                    createPost(input: {
                        authorDid: "did:key:bob",
                        content: "bob's graphql post"
                    }) { id }
                }"#,
            )
            .await;

        // Alice follows Bob
        let res = schema
            .execute(
                r#"mutation {
                    follow(input: {
                        followerDid: "did:key:alice",
                        targetDid: "did:key:bob"
                    }) { success message }
                }"#,
            )
            .await;
        assert!(res.errors.is_empty());

        // Alice's timeline
        let res = schema
            .execute(r#"{ timeline(did: "did:key:alice") { events { content } count } }"#)
            .await;
        assert!(res.errors.is_empty());
        let data = res.data.into_json().unwrap();
        assert_eq!(data["timeline"]["count"], 1);
    }

    #[tokio::test]
    async fn follow_info_query() {
        let state = AppState::new(ApiConfig::default());
        let schema = build_schema(state);

        let _ = schema
            .execute(
                r#"mutation {
                    follow(input: {
                        followerDid: "did:key:alice",
                        targetDid: "did:key:bob"
                    }) { success }
                }"#,
            )
            .await;

        let res = schema
            .execute(r#"{ followInfo(did: "did:key:bob") { followingCount followerCount } }"#)
            .await;
        assert!(res.errors.is_empty());
        let data = res.data.into_json().unwrap();
        assert_eq!(data["followInfo"]["followerCount"], 1);
    }

    #[tokio::test]
    async fn delete_event_mutation() {
        let state = AppState::new(ApiConfig::default());
        let schema = build_schema(state);

        let res = schema
            .execute(
                r#"mutation {
                    createPost(input: {
                        authorDid: "did:key:ztest",
                        content: "to be deleted"
                    }) { id }
                }"#,
            )
            .await;
        let data = res.data.into_json().unwrap();
        let event_id = data["createPost"]["id"].as_str().unwrap();

        let res = schema
            .execute(format!(
                r#"mutation {{ deleteEvent(id: "{event_id}") {{ success }} }}"#,
            ))
            .await;
        assert!(res.errors.is_empty());
        let data = res.data.into_json().unwrap();
        assert!(data["deleteEvent"]["success"].as_bool().unwrap());
    }

    #[tokio::test]
    async fn empty_content_rejected() {
        let schema = test_schema();
        let res = schema
            .execute(
                r#"mutation {
                    createPost(input: {
                        authorDid: "did:key:ztest",
                        content: ""
                    }) { id }
                }"#,
            )
            .await;
        assert!(!res.errors.is_empty());
    }

    #[tokio::test]
    async fn schema_introspection() {
        let schema = test_schema();
        let res = schema.execute("{ __schema { queryType { name } } }").await;
        assert!(res.errors.is_empty());
        let data = res.data.into_json().unwrap();
        assert_eq!(data["__schema"]["queryType"]["name"], "QueryRoot");
    }

    // ── Governance GraphQL tests ──────────────────────────────

    #[tokio::test]
    async fn create_dao_mutation() {
        let schema = test_schema();
        let res = schema
            .execute(
                r#"mutation {
                    createDao(input: {
                        founderDid: "did:key:zfounder",
                        name: "TestDAO",
                        description: "A test DAO"
                    }) { id name founder memberCount }
                }"#,
            )
            .await;
        assert!(res.errors.is_empty(), "errors: {:?}", res.errors);
        let data = res.data.into_json().unwrap();
        assert_eq!(data["createDao"]["name"], "TestDAO");
        assert_eq!(data["createDao"]["memberCount"], 1);
    }

    #[tokio::test]
    async fn query_daos() {
        let state = AppState::new(ApiConfig::default());
        let schema = build_schema(state);

        let _ = schema
            .execute(
                r#"mutation {
                    createDao(input: {
                        founderDid: "did:key:za",
                        name: "Alpha",
                        description: "first"
                    }) { id }
                }"#,
            )
            .await;

        let res = schema.execute("{ daos { id name founder } }").await;
        assert!(res.errors.is_empty());
        let data = res.data.into_json().unwrap();
        assert_eq!(data["daos"].as_array().unwrap().len(), 1);
        assert_eq!(data["daos"][0]["name"], "Alpha");
    }

    #[tokio::test]
    async fn add_dao_member_mutation() {
        let state = AppState::new(ApiConfig::default());
        let schema = build_schema(state);

        let res = schema
            .execute(
                r#"mutation {
                    createDao(input: {
                        founderDid: "did:key:zf",
                        name: "GovDAO",
                        description: "test"
                    }) { id }
                }"#,
            )
            .await;
        let data = res.data.into_json().unwrap();
        let dao_id = data["createDao"]["id"].as_str().unwrap();

        let res = schema
            .execute(format!(
                r#"mutation {{ addDaoMember(daoId: "{dao_id}", did: "did:key:znew") {{ success message }} }}"#,
            ))
            .await;
        assert!(res.errors.is_empty(), "errors: {:?}", res.errors);
        let data = res.data.into_json().unwrap();
        assert!(data["addDaoMember"]["success"].as_bool().unwrap());
    }

    #[tokio::test]
    async fn empty_proposals_query() {
        let schema = test_schema();
        let res = schema.execute("{ proposals { id title status } }").await;
        assert!(res.errors.is_empty());
        let data = res.data.into_json().unwrap();
        assert!(data["proposals"].as_array().unwrap().is_empty());
    }

    // ── Marketplace GraphQL tests ─────────────────────────────

    #[tokio::test]
    async fn create_listing_mutation() {
        let schema = test_schema();
        let res = schema
            .execute(
                r#"mutation {
                    createListing(input: {
                        sellerDid: "did:key:zseller",
                        title: "Rust Book",
                        description: "Learn Rust",
                        category: "digital",
                        priceToken: "USDC",
                        priceAmount: "1000",
                        tags: ["rust", "book"]
                    }) { id title seller category status tags }
                }"#,
            )
            .await;
        assert!(res.errors.is_empty(), "errors: {:?}", res.errors);
        let data = res.data.into_json().unwrap();
        assert_eq!(data["createListing"]["title"], "Rust Book");
        assert_eq!(data["createListing"]["category"], "Digital");
    }

    #[tokio::test]
    async fn query_listings() {
        let state = AppState::new(ApiConfig::default());
        let schema = build_schema(state);

        let _ = schema
            .execute(
                r#"mutation {
                    createListing(input: {
                        sellerDid: "did:key:zs",
                        title: "Widget",
                        description: "A widget",
                        category: "physical",
                        priceToken: "ETH",
                        priceAmount: "500"
                    }) { id }
                }"#,
            )
            .await;

        let res = schema
            .execute("{ listings { id title seller category } }")
            .await;
        assert!(res.errors.is_empty());
        let data = res.data.into_json().unwrap();
        assert_eq!(data["listings"].as_array().unwrap().len(), 1);
        assert_eq!(data["listings"][0]["title"], "Widget");
    }

    #[tokio::test]
    async fn create_review_mutation() {
        let state = AppState::new(ApiConfig::default());
        let schema = build_schema(state);

        let res = schema
            .execute(
                r#"mutation {
                    createReview(input: {
                        listingId: "listing-1",
                        reviewerDid: "did:key:zbuyer",
                        sellerDid: "did:key:zseller",
                        rating: 5,
                        comment: "Excellent"
                    }) { success message }
                }"#,
            )
            .await;
        assert!(res.errors.is_empty(), "errors: {:?}", res.errors);
        let data = res.data.into_json().unwrap();
        assert!(data["createReview"]["success"].as_bool().unwrap());
    }

    #[tokio::test]
    async fn seller_rating_query() {
        let state = AppState::new(ApiConfig::default());
        let schema = build_schema(state);

        // Create some reviews
        let _ = schema
            .execute(
                r#"mutation {
                    createReview(input: {
                        listingId: "l1",
                        reviewerDid: "did:key:zr1",
                        sellerDid: "did:key:zseller",
                        rating: 4,
                        comment: "Good"
                    }) { success }
                }"#,
            )
            .await;

        let _ = schema
            .execute(
                r#"mutation {
                    createReview(input: {
                        listingId: "l2",
                        reviewerDid: "did:key:zr2",
                        sellerDid: "did:key:zseller",
                        rating: 5,
                        comment: "Great"
                    }) { success }
                }"#,
            )
            .await;

        let res = schema
            .execute(
                r#"{ sellerRating(sellerDid: "did:key:zseller") { totalReviews averageRating trusted } }"#,
            )
            .await;
        assert!(res.errors.is_empty(), "errors: {:?}", res.errors);
        let data = res.data.into_json().unwrap();
        assert_eq!(data["sellerRating"]["totalReviews"], 2);
    }

    #[tokio::test]
    async fn cancel_listing_mutation() {
        let state = AppState::new(ApiConfig::default());
        let schema = build_schema(state);

        let res = schema
            .execute(
                r#"mutation {
                    createListing(input: {
                        sellerDid: "did:key:zseller",
                        title: "Cancel Me",
                        description: "test",
                        category: "digital",
                        priceToken: "ETH",
                        priceAmount: "100"
                    }) { id }
                }"#,
            )
            .await;
        let data = res.data.into_json().unwrap();
        let listing_id = data["createListing"]["id"].as_str().unwrap();

        let res = schema
            .execute(format!(
                r#"mutation {{ cancelListing(listingId: "{listing_id}", sellerDid: "did:key:zseller") {{ success }} }}"#,
            ))
            .await;
        assert!(res.errors.is_empty(), "errors: {:?}", res.errors);
        let data = res.data.into_json().unwrap();
        assert!(data["cancelListing"]["success"].as_bool().unwrap());
    }
}
