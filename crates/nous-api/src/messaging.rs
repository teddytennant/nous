use axum::Json;
use axum::extract::{Path, Query, State};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use utoipa::{IntoParams, ToSchema};

use nous_messaging::message::MessageBuilder;
use nous_messaging::{Channel, ChannelKind, Message, MessageContent};

use crate::error::ApiError;
use crate::state::AppState;

// ── Request / Response types ────────────────────────────────────────

#[derive(Debug, Deserialize, ToSchema)]
pub struct CreateChannelRequest {
    pub creator_did: String,
    pub kind: String,
    pub name: Option<String>,
    pub peer_did: Option<String>,
    pub members: Option<Vec<String>>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct ChannelResponse {
    pub id: String,
    pub kind: String,
    pub name: Option<String>,
    pub members: Vec<String>,
    pub created_at: String,
}

impl From<&Channel> for ChannelResponse {
    fn from(ch: &Channel) -> Self {
        let kind = match ch.kind {
            ChannelKind::DirectMessage => "direct",
            ChannelKind::Group => "group",
            ChannelKind::Public => "public",
        };
        Self {
            id: ch.id.clone(),
            kind: kind.to_string(),
            name: ch.name.clone(),
            members: ch.members.clone(),
            created_at: ch.created_at.to_rfc3339(),
        }
    }
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct SendMessageRequest {
    pub channel_id: String,
    pub sender_did: String,
    pub content: String,
    pub reply_to: Option<String>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct MessageResponse {
    pub id: String,
    pub channel_id: String,
    pub sender: String,
    pub content: String,
    pub reply_to: Option<String>,
    pub timestamp: String,
}

impl From<&Message> for MessageResponse {
    fn from(msg: &Message) -> Self {
        let content = match &msg.content {
            MessageContent::Text(t) => t.clone(),
            MessageContent::File { name, .. } => format!("[file: {name}]"),
            MessageContent::Reaction { emoji, .. } => emoji.clone(),
            MessageContent::System(t) => t.clone(),
        };
        Self {
            id: msg.id.clone(),
            channel_id: msg.channel_id.clone(),
            sender: msg.sender_did.clone(),
            content,
            reply_to: msg.reply_to.clone(),
            timestamp: msg.timestamp.to_rfc3339(),
        }
    }
}

#[derive(Debug, Deserialize, IntoParams)]
pub struct ChannelQuery {
    pub did: String,
}

#[derive(Debug, Deserialize, IntoParams)]
pub struct MessageQuery {
    pub limit: Option<usize>,
    pub before: Option<String>,
}

// ── Handlers ────────────────────────────────────────────────────────

#[utoipa::path(
    post, path = "/api/v1/channels",
    tag = "messaging",
    request_body = CreateChannelRequest,
    responses(
        (status = 200, description = "Channel created", body = ChannelResponse),
        (status = 400, description = "Invalid request")
    )
)]
pub async fn create_channel(
    State(state): State<Arc<AppState>>,
    Json(req): Json<CreateChannelRequest>,
) -> Result<Json<ChannelResponse>, ApiError> {
    let channel = match req.kind.as_str() {
        "direct" => {
            let peer = req
                .peer_did
                .as_deref()
                .ok_or_else(|| ApiError::bad_request("peer_did required for direct channels"))?;
            Channel::direct(&req.creator_did, peer)
        }
        "group" => {
            let name = req
                .name
                .as_deref()
                .ok_or_else(|| ApiError::bad_request("name required for group channels"))?;
            let members = req.members.clone().unwrap_or_default();
            Channel::group(&req.creator_did, name, members)
        }
        "public" => {
            let name = req
                .name
                .as_deref()
                .ok_or_else(|| ApiError::bad_request("name required for public channels"))?;
            Channel::public(&req.creator_did, name)
        }
        _ => {
            return Err(ApiError::bad_request(
                "kind must be direct, group, or public",
            ));
        }
    };

    let resp = ChannelResponse::from(&channel);
    let mut channels = state.channels.write().await;
    channels.insert(channel.id.clone(), channel);
    Ok(Json(resp))
}

#[utoipa::path(
    get, path = "/api/v1/channels",
    tag = "messaging",
    params(ChannelQuery),
    responses((status = 200, description = "User's channels"))
)]
pub async fn list_channels(
    State(state): State<Arc<AppState>>,
    Query(query): Query<ChannelQuery>,
) -> Result<Json<Vec<ChannelResponse>>, ApiError> {
    let channels = state.channels.read().await;
    let result: Vec<ChannelResponse> = channels
        .values()
        .filter(|ch| ch.is_member(&query.did))
        .map(ChannelResponse::from)
        .collect();
    Ok(Json(result))
}

#[utoipa::path(
    get, path = "/api/v1/channels/{channel_id}",
    tag = "messaging",
    params(("channel_id" = String, Path, description = "Channel ID")),
    responses(
        (status = 200, description = "Channel details", body = ChannelResponse),
        (status = 404, description = "Channel not found")
    )
)]
pub async fn get_channel(
    State(state): State<Arc<AppState>>,
    Path(channel_id): Path<String>,
) -> Result<Json<ChannelResponse>, ApiError> {
    let channels = state.channels.read().await;
    channels
        .get(&channel_id)
        .map(|ch| Json(ChannelResponse::from(ch)))
        .ok_or_else(|| ApiError::not_found(format!("channel {channel_id} not found")))
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct AddMemberRequest {
    pub did: String,
}

#[utoipa::path(
    post, path = "/api/v1/channels/{channel_id}/members",
    tag = "messaging",
    params(("channel_id" = String, Path, description = "Channel ID")),
    request_body = AddMemberRequest,
    responses(
        (status = 200, description = "Member added"),
        (status = 404, description = "Channel not found")
    )
)]
pub async fn add_channel_member(
    State(state): State<Arc<AppState>>,
    Path(channel_id): Path<String>,
    Json(req): Json<AddMemberRequest>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let mut channels = state.channels.write().await;
    let channel = channels
        .get_mut(&channel_id)
        .ok_or_else(|| ApiError::not_found(format!("channel {channel_id} not found")))?;
    channel.add_member(&req.did);
    Ok(Json(
        serde_json::json!({"added": req.did, "channel": channel_id}),
    ))
}

#[utoipa::path(
    delete, path = "/api/v1/channels/{channel_id}/members/{did}",
    tag = "messaging",
    params(
        ("channel_id" = String, Path, description = "Channel ID"),
        ("did" = String, Path, description = "Member DID")
    ),
    responses(
        (status = 200, description = "Member removed"),
        (status = 404, description = "Not found")
    )
)]
pub async fn remove_channel_member(
    State(state): State<Arc<AppState>>,
    Path((channel_id, did)): Path<(String, String)>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let mut channels = state.channels.write().await;
    let channel = channels
        .get_mut(&channel_id)
        .ok_or_else(|| ApiError::not_found(format!("channel {channel_id} not found")))?;
    let removed = channel.remove_member(&did);
    if removed {
        Ok(Json(
            serde_json::json!({"removed": did, "channel": channel_id}),
        ))
    } else {
        Err(ApiError::not_found(format!(
            "{did} not a member of {channel_id}"
        )))
    }
}

#[utoipa::path(
    post, path = "/api/v1/messages",
    tag = "messaging",
    request_body = SendMessageRequest,
    responses(
        (status = 200, description = "Message sent", body = MessageResponse),
        (status = 400, description = "Invalid request"),
        (status = 404, description = "Channel not found")
    )
)]
pub async fn send_message(
    State(state): State<Arc<AppState>>,
    Json(req): Json<SendMessageRequest>,
) -> Result<Json<MessageResponse>, ApiError> {
    if req.content.is_empty() {
        return Err(ApiError::bad_request("content cannot be empty"));
    }
    if req.content.len() > 10_000 {
        return Err(ApiError::bad_request("content exceeds maximum length"));
    }

    // Verify channel exists and sender is a member
    {
        let channels = state.channels.read().await;
        let channel = channels
            .get(&req.channel_id)
            .ok_or_else(|| ApiError::not_found(format!("channel {} not found", req.channel_id)))?;
        if !channel.is_member(&req.sender_did) {
            return Err(ApiError::unauthorized("not a member of this channel"));
        }
    }

    let mut builder = MessageBuilder::text(&req.channel_id, &req.content);
    if let Some(ref reply_to) = req.reply_to {
        builder = builder.reply_to(reply_to);
    }

    // Create an identity to sign with (in production this would come from auth)
    let identity = nous_identity::Identity::generate().with_display_name(&req.sender_did);
    let message = builder
        .sign(&identity)
        .map_err(|e: nous_core::Error| ApiError::internal(e.to_string()))?;

    let resp = MessageResponse::from(&message);

    let mut messages = state.messages.write().await;
    messages
        .entry(req.channel_id.clone())
        .or_default()
        .push(message);

    Ok(Json(resp))
}

#[utoipa::path(
    get, path = "/api/v1/channels/{channel_id}/messages",
    tag = "messaging",
    params(
        ("channel_id" = String, Path, description = "Channel ID"),
        MessageQuery,
    ),
    responses(
        (status = 200, description = "Channel messages"),
        (status = 404, description = "Channel not found")
    )
)]
pub async fn get_messages(
    State(state): State<Arc<AppState>>,
    Path(channel_id): Path<String>,
    Query(query): Query<MessageQuery>,
) -> Result<Json<Vec<MessageResponse>>, ApiError> {
    // Verify channel exists
    {
        let channels = state.channels.read().await;
        if !channels.contains_key(&channel_id) {
            return Err(ApiError::not_found(format!(
                "channel {channel_id} not found"
            )));
        }
    }

    let messages = state.messages.read().await;
    let limit = query.limit.unwrap_or(50).min(200);

    let result: Vec<MessageResponse> = messages
        .get(&channel_id)
        .map(|msgs| {
            let iter = msgs.iter().rev();
            let filtered: Vec<_> = if let Some(ref before) = query.before {
                iter.skip_while(|m| m.id != *before)
                    .skip(1)
                    .take(limit)
                    .collect()
            } else {
                iter.take(limit).collect()
            };
            filtered
                .into_iter()
                .rev()
                .map(MessageResponse::from)
                .collect()
        })
        .unwrap_or_default();

    Ok(Json(result))
}

#[utoipa::path(
    delete, path = "/api/v1/messages/{message_id}",
    tag = "messaging",
    params(("message_id" = String, Path, description = "Message ID")),
    responses(
        (status = 200, description = "Message deleted"),
        (status = 404, description = "Message not found")
    )
)]
pub async fn delete_message(
    State(state): State<Arc<AppState>>,
    Path(message_id): Path<String>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let mut messages = state.messages.write().await;
    for msgs in messages.values_mut() {
        if let Some(pos) = msgs.iter().position(|m| m.id == message_id) {
            msgs.remove(pos);
            return Ok(Json(serde_json::json!({"deleted": message_id})));
        }
    }
    Err(ApiError::not_found(format!(
        "message {message_id} not found"
    )))
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

    fn json_body(value: &serde_json::Value) -> Body {
        Body::from(serde_json::to_vec(value).unwrap())
    }

    fn json_request(method: &str, uri: &str, body: &serde_json::Value) -> Request<Body> {
        Request::builder()
            .method(method)
            .uri(uri)
            .header("content-type", "application/json")
            .body(json_body(body))
            .unwrap()
    }

    async fn parse_json(response: axum::http::Response<Body>) -> serde_json::Value {
        let body = response.into_body().collect().await.unwrap().to_bytes();
        serde_json::from_slice(&body).unwrap()
    }

    #[tokio::test]
    async fn create_direct_channel() {
        let app = test_app().await;
        let req = serde_json::json!({
            "creator_did": "did:key:alice",
            "kind": "direct",
            "peer_did": "did:key:bob"
        });

        let resp = app
            .oneshot(json_request("POST", "/api/v1/channels", &req))
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::OK);

        let json = parse_json(resp).await;
        assert_eq!(json["kind"], "direct");
    }

    #[tokio::test]
    async fn create_group_channel() {
        let app = test_app().await;
        let req = serde_json::json!({
            "creator_did": "did:key:alice",
            "kind": "group",
            "name": "project-x",
            "members": ["did:key:bob", "did:key:carol"]
        });

        let resp = app
            .oneshot(json_request("POST", "/api/v1/channels", &req))
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::OK);

        let json = parse_json(resp).await;
        assert_eq!(json["kind"], "group");
        assert_eq!(json["name"], "project-x");
    }

    #[tokio::test]
    async fn create_public_channel() {
        let app = test_app().await;
        let req = serde_json::json!({
            "creator_did": "did:key:alice",
            "kind": "public",
            "name": "general"
        });

        let resp = app
            .oneshot(json_request("POST", "/api/v1/channels", &req))
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::OK);

        let json = parse_json(resp).await;
        assert_eq!(json["kind"], "public");
        assert_eq!(json["name"], "general");
    }

    #[tokio::test]
    async fn invalid_channel_kind_rejected() {
        let app = test_app().await;
        let req = serde_json::json!({
            "creator_did": "did:key:alice",
            "kind": "invalid"
        });

        let resp = app
            .oneshot(json_request("POST", "/api/v1/channels", &req))
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
    }

    #[tokio::test]
    async fn direct_channel_missing_peer_rejected() {
        let app = test_app().await;
        let req = serde_json::json!({
            "creator_did": "did:key:alice",
            "kind": "direct"
        });

        let resp = app
            .oneshot(json_request("POST", "/api/v1/channels", &req))
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
    }

    #[tokio::test]
    async fn send_and_get_messages() {
        let app = test_app().await;

        // Create channel
        let ch_req = serde_json::json!({
            "creator_did": "did:key:alice",
            "kind": "direct",
            "peer_did": "did:key:bob"
        });
        let resp = app
            .clone()
            .oneshot(json_request("POST", "/api/v1/channels", &ch_req))
            .await
            .unwrap();
        let ch: serde_json::Value = parse_json(resp).await;
        let channel_id = ch["id"].as_str().unwrap();

        // Send message
        let msg_req = serde_json::json!({
            "channel_id": channel_id,
            "sender_did": "did:key:alice",
            "content": "hello bob"
        });
        let resp = app
            .clone()
            .oneshot(json_request("POST", "/api/v1/messages", &msg_req))
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::OK);

        let msg = parse_json(resp).await;
        assert_eq!(msg["content"], "hello bob");

        // Get messages
        let uri = format!("/api/v1/channels/{channel_id}/messages?limit=10");
        let resp = app
            .oneshot(Request::builder().uri(&uri).body(Body::empty()).unwrap())
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::OK);

        let msgs: Vec<serde_json::Value> =
            serde_json::from_slice(&resp.into_body().collect().await.unwrap().to_bytes()).unwrap();
        assert_eq!(msgs.len(), 1);
        assert_eq!(msgs[0]["content"], "hello bob");
    }

    #[tokio::test]
    async fn send_message_to_nonexistent_channel() {
        let app = test_app().await;
        let req = serde_json::json!({
            "channel_id": "nonexistent",
            "sender_did": "did:key:alice",
            "content": "hello"
        });

        let resp = app
            .oneshot(json_request("POST", "/api/v1/messages", &req))
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::NOT_FOUND);
    }

    #[tokio::test]
    async fn send_empty_message_rejected() {
        let app = test_app().await;
        let req = serde_json::json!({
            "channel_id": "ch",
            "sender_did": "did:key:alice",
            "content": ""
        });

        let resp = app
            .oneshot(json_request("POST", "/api/v1/messages", &req))
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
    }

    #[tokio::test]
    async fn delete_message_works() {
        let app = test_app().await;

        // Create channel
        let ch_req = serde_json::json!({
            "creator_did": "did:key:alice",
            "kind": "public",
            "name": "test"
        });
        let resp = app
            .clone()
            .oneshot(json_request("POST", "/api/v1/channels", &ch_req))
            .await
            .unwrap();
        let ch = parse_json(resp).await;
        let channel_id = ch["id"].as_str().unwrap();

        // Send message
        let msg_req = serde_json::json!({
            "channel_id": channel_id,
            "sender_did": "did:key:alice",
            "content": "to be deleted"
        });
        let resp = app
            .clone()
            .oneshot(json_request("POST", "/api/v1/messages", &msg_req))
            .await
            .unwrap();
        let msg = parse_json(resp).await;
        let message_id = msg["id"].as_str().unwrap();

        // Delete it
        let uri = format!("/api/v1/messages/{message_id}");
        let resp = app
            .oneshot(
                Request::builder()
                    .method("DELETE")
                    .uri(&uri)
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn list_channels_for_user() {
        let app = test_app().await;

        // Create a group channel (alice is creator/member)
        let ch1 = serde_json::json!({
            "creator_did": "did:key:alice",
            "kind": "group",
            "name": "alpha",
            "members": []
        });
        let _ = app
            .clone()
            .oneshot(json_request("POST", "/api/v1/channels", &ch1))
            .await
            .unwrap();

        // Create a group channel (bob is creator, alice not a member)
        let ch2 = serde_json::json!({
            "creator_did": "did:key:bob",
            "kind": "group",
            "name": "beta",
            "members": []
        });
        let _ = app
            .clone()
            .oneshot(json_request("POST", "/api/v1/channels", &ch2))
            .await
            .unwrap();

        // List alice's channels — should only see alpha
        let resp = app
            .oneshot(
                Request::builder()
                    .uri("/api/v1/channels?did=did:key:alice")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::OK);

        let channels: Vec<serde_json::Value> =
            serde_json::from_slice(&resp.into_body().collect().await.unwrap().to_bytes()).unwrap();
        assert_eq!(channels.len(), 1);
        assert_eq!(channels[0]["name"], "alpha");
    }

    #[tokio::test]
    async fn add_and_remove_channel_member() {
        let app = test_app().await;

        // Create channel
        let ch_req = serde_json::json!({
            "creator_did": "did:key:alice",
            "kind": "group",
            "name": "team",
            "members": []
        });
        let resp = app
            .clone()
            .oneshot(json_request("POST", "/api/v1/channels", &ch_req))
            .await
            .unwrap();
        let ch = parse_json(resp).await;
        let channel_id = ch["id"].as_str().unwrap();

        // Add bob
        let add_req = serde_json::json!({"did": "did:key:bob"});
        let uri = format!("/api/v1/channels/{channel_id}/members");
        let resp = app
            .clone()
            .oneshot(json_request("POST", &uri, &add_req))
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::OK);

        // Remove bob
        let uri = format!("/api/v1/channels/{channel_id}/members/did:key:bob");
        let resp = app
            .oneshot(
                Request::builder()
                    .method("DELETE")
                    .uri(&uri)
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn get_nonexistent_channel() {
        let app = test_app().await;
        let resp = app
            .oneshot(
                Request::builder()
                    .uri("/api/v1/channels/nonexistent")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::NOT_FOUND);
    }

    #[tokio::test]
    async fn non_member_cannot_send() {
        let app = test_app().await;

        // Create DM between alice and bob
        let ch_req = serde_json::json!({
            "creator_did": "did:key:alice",
            "kind": "direct",
            "peer_did": "did:key:bob"
        });
        let resp = app
            .clone()
            .oneshot(json_request("POST", "/api/v1/channels", &ch_req))
            .await
            .unwrap();
        let ch = parse_json(resp).await;
        let channel_id = ch["id"].as_str().unwrap();

        // Carol tries to send
        let msg_req = serde_json::json!({
            "channel_id": channel_id,
            "sender_did": "did:key:carol",
            "content": "infiltrated"
        });
        let resp = app
            .oneshot(json_request("POST", "/api/v1/messages", &msg_req))
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
    }
}
