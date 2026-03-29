use axum::Json;
use axum::extract::{Path, Query, State};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use utoipa::{IntoParams, ToSchema};

use nous_social::{EventKind, PostBuilder, SignedEvent};

use crate::error::ApiError;
use crate::state::AppState;

#[derive(Debug, Serialize, ToSchema)]
pub struct HealthResponse {
    pub status: String,
    pub version: String,
    pub uptime_ms: u64,
}

#[utoipa::path(
    get, path = "/api/v1/health",
    tag = "node",
    responses((status = 200, description = "Server health status", body = HealthResponse))
)]
pub async fn health() -> Json<HealthResponse> {
    Json(HealthResponse {
        status: "ok".to_string(),
        version: env!("CARGO_PKG_VERSION").to_string(),
        uptime_ms: 0,
    })
}

#[derive(Debug, Serialize, ToSchema)]
pub struct NodeInfo {
    pub protocol: String,
    pub version: String,
    pub features: Vec<String>,
}

#[utoipa::path(
    get, path = "/api/v1/node",
    tag = "node",
    responses((status = 200, description = "Node information", body = NodeInfo))
)]
pub async fn node_info() -> Json<NodeInfo> {
    Json(NodeInfo {
        protocol: "nous".to_string(),
        version: env!("CARGO_PKG_VERSION").to_string(),
        features: vec![
            "identity".into(),
            "messaging".into(),
            "social".into(),
            "governance".into(),
            "payments".into(),
            "marketplace".into(),
        ],
    })
}

#[derive(Debug, Deserialize, ToSchema, IntoParams)]
pub struct FeedQuery {
    pub limit: Option<usize>,
    pub author: Option<String>,
    pub kind: Option<u32>,
    pub tag: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct FeedResponse {
    pub events: Vec<SignedEvent>,
    pub count: usize,
}

#[utoipa::path(
    get, path = "/api/v1/feed",
    tag = "social",
    params(FeedQuery),
    responses((status = 200, description = "Event feed"))
)]
pub async fn get_feed(
    State(state): State<Arc<AppState>>,
    Query(query): Query<FeedQuery>,
) -> Result<Json<FeedResponse>, ApiError> {
    let feed = state.feed.read().await;
    let limit = query.limit.unwrap_or(50).min(200);

    let events: Vec<SignedEvent> = if let Some(ref author) = query.author {
        feed.by_author(author)
            .into_iter()
            .take(limit)
            .cloned()
            .collect()
    } else if let Some(kind) = query.kind {
        feed.by_kind(EventKind::from(kind))
            .into_iter()
            .take(limit)
            .cloned()
            .collect()
    } else if let Some(ref tag) = query.tag {
        feed.by_hashtag(tag)
            .into_iter()
            .take(limit)
            .cloned()
            .collect()
    } else {
        feed.latest(limit).into_iter().cloned().collect()
    };

    let count = events.len();
    Ok(Json(FeedResponse { events, count }))
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct CreatePostRequest {
    pub author_did: String,
    pub content: String,
    pub reply_to: Option<String>,
    pub hashtags: Option<Vec<String>>,
}

#[utoipa::path(
    post, path = "/api/v1/events",
    tag = "social",
    request_body = CreatePostRequest,
    responses(
        (status = 200, description = "Created event"),
        (status = 400, description = "Invalid request")
    )
)]
pub async fn create_post(
    State(state): State<Arc<AppState>>,
    Json(req): Json<CreatePostRequest>,
) -> Result<Json<SignedEvent>, ApiError> {
    if req.content.is_empty() {
        return Err(ApiError::bad_request("content cannot be empty"));
    }

    if req.content.len() > 10_000 {
        return Err(ApiError::bad_request("content exceeds maximum length"));
    }

    let mut builder = PostBuilder::new(&req.author_did, &req.content);

    if let Some(ref reply_to) = req.reply_to {
        builder = builder.reply_to(reply_to);
    }

    if let Some(ref tags) = req.hashtags {
        for tag in tags {
            builder = builder.hashtag(tag);
        }
    }

    let event = builder.build();

    let mut feed = state.feed.write().await;
    feed.insert(event.clone());

    state.emit(crate::state::RealtimeEvent::NewPost {
        id: event.id.clone(),
        author: event.pubkey.clone(),
        content: req.content.clone(),
    });

    Ok(Json(event))
}

#[utoipa::path(
    get, path = "/api/v1/events/{event_id}",
    tag = "social",
    params(("event_id" = String, Path, description = "Event ID")),
    responses(
        (status = 200, description = "Event found"),
        (status = 404, description = "Event not found")
    )
)]
pub async fn get_event(
    State(state): State<Arc<AppState>>,
    Path(event_id): Path<String>,
) -> Result<Json<SignedEvent>, ApiError> {
    let feed = state.feed.read().await;
    let events = feed.latest(10_000);

    events
        .into_iter()
        .find(|e| e.id == event_id)
        .cloned()
        .map(Json)
        .ok_or_else(|| ApiError::not_found(format!("event {event_id} not found")))
}

#[utoipa::path(
    delete, path = "/api/v1/events/{event_id}",
    tag = "social",
    params(("event_id" = String, Path, description = "Event ID")),
    responses(
        (status = 200, description = "Event deleted"),
        (status = 404, description = "Event not found")
    )
)]
pub async fn delete_event(
    State(state): State<Arc<AppState>>,
    Path(event_id): Path<String>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let mut feed = state.feed.write().await;
    if feed.remove(&event_id) {
        Ok(Json(serde_json::json!({"deleted": event_id})))
    } else {
        Err(ApiError::not_found(format!("event {event_id} not found")))
    }
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct FollowRequest {
    pub follower_did: String,
    pub target_did: String,
}

#[utoipa::path(
    post, path = "/api/v1/follow",
    tag = "social",
    request_body = FollowRequest,
    responses((status = 200, description = "Follow recorded"))
)]
pub async fn follow_user(
    State(state): State<Arc<AppState>>,
    Json(req): Json<FollowRequest>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let mut graph = state.follow_graph.write().await;
    let added = graph.follow(&req.follower_did, &req.target_did);
    Ok(Json(serde_json::json!({
        "followed": added,
        "follower": req.follower_did,
        "target": req.target_did,
    })))
}

#[utoipa::path(
    post, path = "/api/v1/unfollow",
    tag = "social",
    request_body = FollowRequest,
    responses((status = 200, description = "Unfollow recorded"))
)]
pub async fn unfollow_user(
    State(state): State<Arc<AppState>>,
    Json(req): Json<FollowRequest>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let mut graph = state.follow_graph.write().await;
    let removed = graph.unfollow(&req.follower_did, &req.target_did);
    Ok(Json(serde_json::json!({
        "unfollowed": removed,
        "follower": req.follower_did,
        "target": req.target_did,
    })))
}

#[derive(Debug, Deserialize, ToSchema, IntoParams)]
pub struct TimelineQuery {
    pub did: String,
    pub limit: Option<usize>,
}

#[utoipa::path(
    get, path = "/api/v1/timeline",
    tag = "social",
    params(TimelineQuery),
    responses((status = 200, description = "Timeline events"))
)]
pub async fn get_timeline(
    State(state): State<Arc<AppState>>,
    Query(query): Query<TimelineQuery>,
) -> Result<Json<FeedResponse>, ApiError> {
    let graph = state.follow_graph.read().await;
    let following: Vec<String> = graph
        .following_of(&query.did)
        .into_iter()
        .map(|s| s.to_string())
        .collect();
    drop(graph);

    let feed = state.feed.read().await;
    let limit = query.limit.unwrap_or(50).min(200);
    let events: Vec<SignedEvent> = feed
        .timeline(&following, limit)
        .into_iter()
        .cloned()
        .collect();

    let count = events.len();
    Ok(Json(FeedResponse { events, count }))
}

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

    #[tokio::test]
    async fn health_check() {
        let app = test_app().await;
        let response = app
            .oneshot(
                Request::builder()
                    .uri("/api/v1/health")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
        let body = response.into_body().collect().await.unwrap().to_bytes();
        let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(json["status"], "ok");
    }

    #[tokio::test]
    async fn node_info_endpoint() {
        let app = test_app().await;
        let response = app
            .oneshot(
                Request::builder()
                    .uri("/api/v1/node")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
        let body = response.into_body().collect().await.unwrap().to_bytes();
        let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(json["protocol"], "nous");
    }

    #[tokio::test]
    async fn empty_feed() {
        let app = test_app().await;
        let response = app
            .oneshot(
                Request::builder()
                    .uri("/api/v1/feed")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
        let body = response.into_body().collect().await.unwrap().to_bytes();
        let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(json["count"], 0);
    }

    #[tokio::test]
    async fn create_and_get_post() {
        let app = test_app().await;

        let create_req = serde_json::json!({
            "author_did": "did:key:ztest",
            "content": "hello nous",
            "hashtags": ["test"]
        });

        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/v1/events")
                    .header("content-type", "application/json")
                    .body(Body::from(serde_json::to_vec(&create_req).unwrap()))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
        let body = response.into_body().collect().await.unwrap().to_bytes();
        let event: SignedEvent = serde_json::from_slice(&body).unwrap();
        assert_eq!(event.content, "hello nous");

        // Get it back via feed
        let response = app
            .oneshot(
                Request::builder()
                    .uri("/api/v1/feed")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        let body = response.into_body().collect().await.unwrap().to_bytes();
        let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(json["count"], 1);
    }

    #[tokio::test]
    async fn empty_content_rejected() {
        let app = test_app().await;
        let req = serde_json::json!({
            "author_did": "did:key:ztest",
            "content": ""
        });

        let response = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/v1/events")
                    .header("content-type", "application/json")
                    .body(Body::from(serde_json::to_vec(&req).unwrap()))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    }

    #[tokio::test]
    async fn follow_and_timeline() {
        let app = test_app().await;

        // Create a post from bob
        let post_req = serde_json::json!({
            "author_did": "did:key:bob",
            "content": "bob's post"
        });

        let _ = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/v1/events")
                    .header("content-type", "application/json")
                    .body(Body::from(serde_json::to_vec(&post_req).unwrap()))
                    .unwrap(),
            )
            .await
            .unwrap();

        // Alice follows bob
        let follow_req = serde_json::json!({
            "follower_did": "did:key:alice",
            "target_did": "did:key:bob"
        });

        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/v1/follow")
                    .header("content-type", "application/json")
                    .body(Body::from(serde_json::to_vec(&follow_req).unwrap()))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);

        // Alice's timeline should include bob's post
        let response = app
            .oneshot(
                Request::builder()
                    .uri("/api/v1/timeline?did=did:key:alice")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        let body = response.into_body().collect().await.unwrap().to_bytes();
        let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(json["count"], 1);
    }

    #[tokio::test]
    async fn event_not_found() {
        let app = test_app().await;
        let response = app
            .oneshot(
                Request::builder()
                    .uri("/api/v1/events/nonexistent")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::NOT_FOUND);
    }
}
