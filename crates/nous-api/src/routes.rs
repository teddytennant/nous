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

/// Status of a single node subsystem.
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct SubsystemStatus {
    /// Subsystem name (e.g., "networking", "identity").
    pub name: String,
    /// Current status.
    pub status: SubsystemHealth,
    /// Number of active entities managed by this subsystem.
    pub active_count: usize,
    /// Optional status message.
    pub message: Option<String>,
}

/// Health state of a subsystem.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "lowercase")]
pub enum SubsystemHealth {
    /// Fully operational.
    Healthy,
    /// Operational but with warnings.
    Degraded,
    /// Not operational.
    Down,
}

/// Response containing all subsystem statuses.
#[derive(Debug, Serialize, ToSchema)]
pub struct SubsystemsResponse {
    pub subsystems: Vec<SubsystemStatus>,
    pub overall: SubsystemHealth,
}

#[utoipa::path(
    get, path = "/api/v1/node/subsystems",
    tag = "node",
    responses((status = 200, description = "Subsystem health statuses", body = SubsystemsResponse))
)]
pub async fn node_subsystems(State(state): State<Arc<AppState>>) -> Json<SubsystemsResponse> {
    let peers = state.peers.read().await;
    let identities = state.identities.read().await;
    let channels = state.channels.read().await;
    let daos = state.daos.read().await;
    let wallets = state.wallets.read().await;
    let feed = state.feed.read().await;
    let agents = state.agents.read().await;
    let file_store = state.file_store.read().await;

    let subsystems = vec![
        SubsystemStatus {
            name: "networking".into(),
            status: if peers.is_empty() {
                SubsystemHealth::Degraded
            } else {
                SubsystemHealth::Healthy
            },
            active_count: peers.len(),
            message: Some(format!("{} connected peers", peers.len())),
        },
        SubsystemStatus {
            name: "identity".into(),
            status: if identities.is_empty() {
                SubsystemHealth::Degraded
            } else {
                SubsystemHealth::Healthy
            },
            active_count: identities.len(),
            message: None,
        },
        SubsystemStatus {
            name: "messaging".into(),
            status: SubsystemHealth::Healthy,
            active_count: channels.len(),
            message: Some(format!("{} active channels", channels.len())),
        },
        SubsystemStatus {
            name: "governance".into(),
            status: SubsystemHealth::Healthy,
            active_count: daos.len(),
            message: Some(format!("{} DAOs", daos.len())),
        },
        SubsystemStatus {
            name: "payments".into(),
            status: SubsystemHealth::Healthy,
            active_count: wallets.len(),
            message: Some(format!("{} wallets", wallets.len())),
        },
        SubsystemStatus {
            name: "social".into(),
            status: SubsystemHealth::Healthy,
            active_count: feed.len(),
            message: Some(format!("{} events in feed", feed.len())),
        },
        SubsystemStatus {
            name: "ai".into(),
            status: SubsystemHealth::Healthy,
            active_count: agents.len(),
            message: Some(format!("{} agents", agents.len())),
        },
        SubsystemStatus {
            name: "storage".into(),
            status: SubsystemHealth::Healthy,
            active_count: file_store.stats().total_files,
            message: Some(format!("{} files", file_store.stats().total_files)),
        },
    ];

    let overall = if subsystems.iter().any(|s| s.status == SubsystemHealth::Down) {
        SubsystemHealth::Down
    } else if subsystems
        .iter()
        .any(|s| s.status == SubsystemHealth::Degraded)
    {
        SubsystemHealth::Degraded
    } else {
        SubsystemHealth::Healthy
    };

    Json(SubsystemsResponse {
        subsystems,
        overall,
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

    // Persist feed to SQLite
    state.persist_feed(&feed).await;

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
        // Persist feed to SQLite
        state.persist_feed(&feed).await;
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

    // Persist follow graph to SQLite
    state.persist_follow_graph(&graph).await;

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

    // Persist follow graph to SQLite
    state.persist_follow_graph(&graph).await;

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

// ── Peers ────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct PeerInfo {
    pub peer_id: String,
    pub multiaddr: String,
    pub latency_ms: Option<u64>,
    pub bytes_sent: u64,
    pub bytes_recv: u64,
    pub connected_at: String,
    pub protocols: Vec<String>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct PeersResponse {
    pub peers: Vec<PeerInfo>,
    pub count: usize,
}

#[utoipa::path(
    get, path = "/api/v1/peers",
    tag = "node",
    responses((status = 200, description = "Connected peers", body = PeersResponse))
)]
pub async fn list_peers(State(state): State<Arc<AppState>>) -> Json<PeersResponse> {
    let peers = state.peers.read().await;
    let count = peers.len();
    Json(PeersResponse {
        peers: peers.clone(),
        count,
    })
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct ConnectPeerRequest {
    pub multiaddr: String,
}

#[utoipa::path(
    post, path = "/api/v1/peers",
    tag = "node",
    request_body = ConnectPeerRequest,
    responses(
        (status = 200, description = "Peer connected"),
        (status = 400, description = "Invalid multiaddr")
    )
)]
pub async fn connect_peer(
    State(state): State<Arc<AppState>>,
    Json(req): Json<ConnectPeerRequest>,
) -> Result<Json<PeerInfo>, ApiError> {
    if req.multiaddr.is_empty() {
        return Err(ApiError::bad_request("multiaddr cannot be empty"));
    }

    let peer = PeerInfo {
        peer_id: format!("12D3KooW{}", &uuid::Uuid::new_v4().to_string()[..12]),
        multiaddr: req.multiaddr,
        latency_ms: Some(42),
        bytes_sent: 0,
        bytes_recv: 0,
        connected_at: chrono::Utc::now().to_rfc3339(),
        protocols: vec!["nous/1.0".into(), "gossipsub/1.1".into()],
    };

    let mut peers = state.peers.write().await;
    peers.push(peer.clone());

    Ok(Json(peer))
}

#[utoipa::path(
    delete, path = "/api/v1/peers/{peer_id}",
    tag = "node",
    params(("peer_id" = String, Path, description = "Peer ID")),
    responses(
        (status = 200, description = "Peer disconnected"),
        (status = 404, description = "Peer not found")
    )
)]
pub async fn disconnect_peer(
    State(state): State<Arc<AppState>>,
    Path(peer_id): Path<String>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let mut peers = state.peers.write().await;
    let before = peers.len();
    peers.retain(|p| p.peer_id != peer_id);

    if peers.len() < before {
        Ok(Json(serde_json::json!({"disconnected": peer_id})))
    } else {
        Err(ApiError::not_found(format!("peer {peer_id} not found")))
    }
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
    async fn node_subsystems_endpoint() {
        let app = test_app().await;
        let response = app
            .oneshot(
                Request::builder()
                    .uri("/api/v1/node/subsystems")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
        let body = response.into_body().collect().await.unwrap().to_bytes();
        let json: serde_json::Value = serde_json::from_slice(&body).unwrap();

        let subsystems = json["subsystems"].as_array().unwrap();
        assert_eq!(subsystems.len(), 8);

        // Check all expected subsystem names are present
        let names: Vec<&str> = subsystems
            .iter()
            .map(|s| s["name"].as_str().unwrap())
            .collect();
        assert!(names.contains(&"networking"));
        assert!(names.contains(&"identity"));
        assert!(names.contains(&"messaging"));
        assert!(names.contains(&"governance"));
        assert!(names.contains(&"payments"));
        assert!(names.contains(&"social"));
        assert!(names.contains(&"ai"));
        assert!(names.contains(&"storage"));

        // Overall should be degraded (no peers/identities in fresh state)
        assert_eq!(json["overall"], "degraded");
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
    async fn list_peers_empty() {
        let app = test_app().await;
        let response = app
            .oneshot(
                Request::builder()
                    .uri("/api/v1/peers")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
        let body = response.into_body().collect().await.unwrap().to_bytes();
        let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(json["count"], 0);
        assert!(json["peers"].as_array().unwrap().is_empty());
    }

    #[tokio::test]
    async fn connect_and_list_peer() {
        let app = test_app().await;

        let req = serde_json::json!({
            "multiaddr": "/ip4/192.168.1.1/tcp/9000"
        });

        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/v1/peers")
                    .header("content-type", "application/json")
                    .body(Body::from(serde_json::to_vec(&req).unwrap()))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
        let body = response.into_body().collect().await.unwrap().to_bytes();
        let peer: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert!(peer["peer_id"].as_str().unwrap().starts_with("12D3KooW"));
        assert_eq!(peer["multiaddr"], "/ip4/192.168.1.1/tcp/9000");

        // Verify peer appears in list
        let response = app
            .oneshot(
                Request::builder()
                    .uri("/api/v1/peers")
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
    async fn connect_peer_empty_multiaddr_rejected() {
        let app = test_app().await;

        let req = serde_json::json!({ "multiaddr": "" });

        let response = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/v1/peers")
                    .header("content-type", "application/json")
                    .body(Body::from(serde_json::to_vec(&req).unwrap()))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    }

    #[tokio::test]
    async fn disconnect_peer_not_found() {
        let app = test_app().await;

        let response = app
            .oneshot(
                Request::builder()
                    .method("DELETE")
                    .uri("/api/v1/peers/nonexistent")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::NOT_FOUND);
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
