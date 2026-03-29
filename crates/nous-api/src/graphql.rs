//! GraphQL schema and resolvers for the Nous API.
//!
//! Provides a full GraphQL endpoint alongside the REST API,
//! sharing the same application state.

use std::sync::Arc;

use async_graphql::{Context, EmptySubscription, InputObject, Object, Schema, SimpleObject};
use axum::extract::State;
use axum::Json;

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

// ── Query Root ──────��──────────────────────────────────────────

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
}

// ── Schema ─────────────────────────��───────────────────────────

pub type NousSchema = Schema<QueryRoot, MutationRoot, EmptySubscription>;

pub fn build_schema(state: Arc<AppState>) -> NousSchema {
    Schema::build(QueryRoot, MutationRoot, EmptySubscription)
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
            .execute(
                r#"{ timeline(did: "did:key:alice") { events { content } count } }"#,
            )
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
            .execute(
                r#"{ followInfo(did: "did:key:bob") { followingCount followerCount } }"#,
            )
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
        let res = schema
            .execute("{ __schema { queryType { name } } }")
            .await;
        assert!(res.errors.is_empty());
        let data = res.data.into_json().unwrap();
        assert_eq!(data["__schema"]["queryType"]["name"], "QueryRoot");
    }
}
