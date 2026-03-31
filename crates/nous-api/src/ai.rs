use axum::Json;
use axum::extract::{Path, Query, State};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use utoipa::{IntoParams, ToSchema};

use nous_ai::{Agent, Conversation, ExecutionConfig, Message, Role, run_agent};

use crate::error::ApiError;
use crate::state::AppState;

// ── Request/Response Types ─────────────────────────────────────────────────

#[derive(Debug, Deserialize, ToSchema)]
pub struct CreateAgentRequest {
    pub name: String,
    pub system_prompt: Option<String>,
    pub model: Option<String>,
    pub temperature: Option<f32>,
    pub max_tokens: Option<u32>,
    pub capabilities: Option<Vec<String>>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct AgentResponse {
    pub id: String,
    pub name: String,
    pub system_prompt: String,
    pub model: String,
    pub temperature: f32,
    pub max_tokens: u32,
    pub capabilities: Vec<String>,
}

impl From<&Agent> for AgentResponse {
    fn from(a: &Agent) -> Self {
        Self {
            id: a.id.clone(),
            name: a.name.clone(),
            system_prompt: a.system_prompt.clone(),
            model: a.model.clone(),
            temperature: a.temperature,
            max_tokens: a.max_tokens,
            capabilities: a.capabilities.clone(),
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct AgentListResponse {
    pub agents: Vec<AgentResponse>,
    pub count: usize,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct ChatRequest {
    pub agent_id: String,
    pub message: String,
    pub conversation_id: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ChatResponse {
    pub conversation_id: String,
    pub response: String,
    pub role: String,
    pub message_count: usize,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ConversationResponse {
    pub id: String,
    pub agent_id: String,
    pub message_count: usize,
    pub created_at: String,
    pub updated_at: String,
}

impl From<&Conversation> for ConversationResponse {
    fn from(c: &Conversation) -> Self {
        Self {
            id: c.id.clone(),
            agent_id: c.agent_id.clone(),
            message_count: c.len(),
            created_at: c.created_at.to_rfc3339(),
            updated_at: c.updated_at.to_rfc3339(),
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct MessageResponse {
    pub id: String,
    pub role: String,
    pub content: String,
    pub timestamp: String,
}

#[derive(Debug, Deserialize, IntoParams)]
pub struct ListConversationsQuery {
    pub agent_id: Option<String>,
    pub limit: Option<usize>,
}

// ── Handlers ───────────────────────────────────────────────────────────────

/// Create a new AI agent.
pub async fn create_agent(
    State(state): State<Arc<AppState>>,
    Json(req): Json<CreateAgentRequest>,
) -> Result<Json<AgentResponse>, ApiError> {
    if req.name.trim().is_empty() {
        return Err(ApiError::bad_request("agent name cannot be empty"));
    }

    let id = format!("agent:{}", uuid::Uuid::new_v4());
    let mut agent = Agent::new(&id, &req.name);

    if let Some(prompt) = &req.system_prompt {
        agent = agent.with_system_prompt(prompt);
    }
    if let Some(model) = &req.model {
        agent = agent.with_model(model);
    }
    if let Some(temp) = req.temperature {
        agent = agent.with_temperature(temp);
    }
    if let Some(max) = req.max_tokens {
        agent = agent.with_max_tokens(max);
    }
    if let Some(caps) = &req.capabilities {
        for cap in caps {
            agent = agent.with_capability(cap);
        }
    }

    let response = AgentResponse::from(&agent);
    let aid = id.clone();
    let mut agents = state.agents.write().await;
    agents.insert(id, agent);

    // Persist agent to SQLite
    if let Some(a) = agents.get(&aid) {
        state.persist_agent(&aid, a).await;
    }

    Ok(Json(response))
}

/// List all agents.
pub async fn list_agents(
    State(state): State<Arc<AppState>>,
) -> Result<Json<AgentListResponse>, ApiError> {
    let agents = state.agents.read().await;
    let list: Vec<AgentResponse> = agents.values().map(AgentResponse::from).collect();
    let count = list.len();
    Ok(Json(AgentListResponse {
        agents: list,
        count,
    }))
}

/// Get a specific agent.
pub async fn get_agent(
    State(state): State<Arc<AppState>>,
    Path(agent_id): Path<String>,
) -> Result<Json<AgentResponse>, ApiError> {
    let agents = state.agents.read().await;
    let agent = agents
        .get(&agent_id)
        .ok_or_else(|| ApiError::not_found(format!("agent {agent_id} not found")))?;
    Ok(Json(AgentResponse::from(agent)))
}

/// Delete an agent.
pub async fn delete_agent(
    State(state): State<Arc<AppState>>,
    Path(agent_id): Path<String>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let removed = state.agents.write().await.remove(&agent_id);
    if removed.is_none() {
        return Err(ApiError::not_found(format!("agent {agent_id} not found")));
    }

    // Delete agent from SQLite
    state.delete_agent_entry(&agent_id).await;

    Ok(Json(serde_json::json!({"deleted": true})))
}

/// Send a message and get an AI response.
///
/// Requires an [`InferenceBackend`] to be configured on [`AppState`] via
/// [`AppState::set_inference_backend`]. Returns 503 Service Unavailable if
/// no backend is configured.
pub async fn chat(
    State(state): State<Arc<AppState>>,
    Json(req): Json<ChatRequest>,
) -> Result<Json<ChatResponse>, ApiError> {
    if req.message.trim().is_empty() {
        return Err(ApiError::bad_request("message cannot be empty"));
    }

    let agents = state.agents.read().await;
    let agent = agents
        .get(&req.agent_id)
        .ok_or_else(|| ApiError::not_found(format!("agent {} not found", req.agent_id)))?;

    let mut conversations = state.conversations.write().await;

    let conv_id = req
        .conversation_id
        .unwrap_or_else(|| format!("conv:{}", uuid::Uuid::new_v4()));

    let conv = conversations.entry(conv_id.clone()).or_insert_with(|| {
        let mut c = Conversation::new(&req.agent_id);
        // Inject system prompt.
        if !agent.system_prompt.is_empty() {
            c.add_message(Message::system(&agent.system_prompt));
        }
        c
    });

    // Add the user message.
    conv.add_message(Message::user(&req.message));

    let backend = state.inference_backend.read().await.clone();
    let response_text = if let Some(backend) = backend.as_ref() {
        let agent_clone = agent.clone();
        // Drop the agents read lock before the potentially long inference call.
        drop(agents);
        let config = ExecutionConfig::default();
        match run_agent(&agent_clone, conv, backend.as_ref(), None, &config).await {
            Ok(result) => result.response,
            Err(e) => {
                return Err(ApiError::internal(format!("inference backend error: {e}")));
            }
        }
    } else {
        return Err(ApiError::service_unavailable(
            "no inference backend configured — call AppState::set_inference_backend first",
        ));
    };

    // Persist conversation to SQLite
    state.persist_conversation(&conv_id, conv).await;

    Ok(Json(ChatResponse {
        conversation_id: conv_id,
        response: response_text,
        role: "assistant".to_string(),
        message_count: conv.len(),
    }))
}

/// List conversations.
pub async fn list_conversations(
    State(state): State<Arc<AppState>>,
    Query(query): Query<ListConversationsQuery>,
) -> Result<Json<Vec<ConversationResponse>>, ApiError> {
    let conversations = state.conversations.read().await;
    let limit = query.limit.unwrap_or(50);

    let mut list: Vec<ConversationResponse> = conversations
        .values()
        .filter(|c| query.agent_id.as_ref().is_none_or(|aid| c.agent_id == *aid))
        .map(ConversationResponse::from)
        .collect();

    list.sort_by(|a, b| b.updated_at.cmp(&a.updated_at));
    list.truncate(limit);

    Ok(Json(list))
}

/// Get conversation history.
pub async fn get_conversation(
    State(state): State<Arc<AppState>>,
    Path(conversation_id): Path<String>,
) -> Result<Json<Vec<MessageResponse>>, ApiError> {
    let conversations = state.conversations.read().await;
    let conv = conversations
        .get(&conversation_id)
        .ok_or_else(|| ApiError::not_found(format!("conversation {conversation_id} not found")))?;

    let messages: Vec<MessageResponse> = conv
        .messages
        .iter()
        .map(|m| MessageResponse {
            id: m.id.clone(),
            role: match m.role {
                Role::System => "system".to_string(),
                Role::User => "user".to_string(),
                Role::Assistant => "assistant".to_string(),
                Role::ToolCall => "tool_call".to_string(),
                Role::Tool => "tool".to_string(),
            },
            content: m.content.clone(),
            timestamp: m.timestamp.to_rfc3339(),
        })
        .collect();

    Ok(Json(messages))
}

/// Delete a conversation.
pub async fn delete_conversation(
    State(state): State<Arc<AppState>>,
    Path(conversation_id): Path<String>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let removed = state.conversations.write().await.remove(&conversation_id);
    if removed.is_none() {
        return Err(ApiError::not_found(format!(
            "conversation {conversation_id} not found"
        )));
    }

    // Delete conversation from SQLite
    state.delete_conversation_entry(&conversation_id).await;

    Ok(Json(serde_json::json!({"deleted": true})))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::ApiConfig;
    use axum::Router;
    use axum::body::Body;
    use axum::routing::{delete, get, post};
    use http_body_util::BodyExt;
    use tower::ServiceExt;

    fn app() -> Router {
        let state = AppState::new(ApiConfig::default());
        Router::new()
            .route("/agents", post(create_agent))
            .route("/agents", get(list_agents))
            .route("/agents/{agent_id}", get(get_agent))
            .route("/agents/{agent_id}", delete(delete_agent))
            .route("/chat", post(chat))
            .route("/conversations", get(list_conversations))
            .route("/conversations/{conversation_id}", get(get_conversation))
            .route(
                "/conversations/{conversation_id}",
                delete(delete_conversation),
            )
            .with_state(state)
    }

    fn json_request(method: &str, uri: &str, body: serde_json::Value) -> axum::http::Request<Body> {
        axum::http::Request::builder()
            .method(method)
            .uri(uri)
            .header("content-type", "application/json")
            .body(Body::from(serde_json::to_string(&body).unwrap()))
            .unwrap()
    }

    fn get_request(uri: &str) -> axum::http::Request<Body> {
        axum::http::Request::builder()
            .uri(uri)
            .body(Body::empty())
            .unwrap()
    }

    fn delete_request(uri: &str) -> axum::http::Request<Body> {
        axum::http::Request::builder()
            .method("DELETE")
            .uri(uri)
            .body(Body::empty())
            .unwrap()
    }

    async fn body_json(response: axum::http::Response<Body>) -> serde_json::Value {
        let bytes = response.into_body().collect().await.unwrap().to_bytes();
        serde_json::from_slice(&bytes).unwrap()
    }

    #[tokio::test]
    async fn create_and_get_agent() {
        let app = app();

        let res = app
            .clone()
            .oneshot(json_request(
                "POST",
                "/agents",
                serde_json::json!({
                    "name": "Research Agent",
                    "system_prompt": "You are a research assistant.",
                    "temperature": 0.5,
                    "capabilities": ["search", "summarize"]
                }),
            ))
            .await
            .unwrap();

        assert_eq!(res.status(), 200);
        let body = body_json(res).await;
        assert_eq!(body["name"], "Research Agent");
        assert_eq!(body["temperature"], 0.5);
        let agent_id = body["id"].as_str().unwrap().to_string();

        // Get agent
        let res = app
            .oneshot(get_request(&format!("/agents/{agent_id}")))
            .await
            .unwrap();
        assert_eq!(res.status(), 200);
        let body = body_json(res).await;
        assert_eq!(body["name"], "Research Agent");
    }

    #[tokio::test]
    async fn create_agent_empty_name_fails() {
        let app = app();
        let res = app
            .oneshot(json_request(
                "POST",
                "/agents",
                serde_json::json!({"name": ""}),
            ))
            .await
            .unwrap();
        assert_eq!(res.status(), 400);
    }

    #[tokio::test]
    async fn list_agents_empty() {
        let app = app();
        let res = app.oneshot(get_request("/agents")).await.unwrap();
        assert_eq!(res.status(), 200);
        let body = body_json(res).await;
        assert_eq!(body["count"], 0);
    }

    #[tokio::test]
    async fn get_missing_agent_404() {
        let app = app();
        let res = app
            .oneshot(get_request("/agents/agent:nonexistent"))
            .await
            .unwrap();
        assert_eq!(res.status(), 404);
    }

    #[tokio::test]
    async fn remove_agent() {
        let app = app();

        let res = app
            .clone()
            .oneshot(json_request(
                "POST",
                "/agents",
                serde_json::json!({"name": "Temp Agent"}),
            ))
            .await
            .unwrap();
        let body = body_json(res).await;
        let agent_id = body["id"].as_str().unwrap().to_string();

        let res = app
            .oneshot(delete_request(&format!("/agents/{agent_id}")))
            .await
            .unwrap();
        assert_eq!(res.status(), 200);
    }

    #[tokio::test]
    async fn chat_without_backend_returns_503() {
        let app = app();

        // Create agent first
        let res = app
            .clone()
            .oneshot(json_request(
                "POST",
                "/agents",
                serde_json::json!({
                    "name": "Chat Bot",
                    "system_prompt": "You are helpful."
                }),
            ))
            .await
            .unwrap();
        let body = body_json(res).await;
        let agent_id = body["id"].as_str().unwrap().to_string();

        // Send chat — no backend configured, expect 503
        let res = app
            .oneshot(json_request(
                "POST",
                "/chat",
                serde_json::json!({
                    "agent_id": agent_id,
                    "message": "Hello!"
                }),
            ))
            .await
            .unwrap();
        assert_eq!(res.status(), 503);
        let body = body_json(res).await;
        assert!(body["error"]["message"]
            .as_str()
            .unwrap()
            .contains("no inference backend configured"));
    }

    #[tokio::test]
    async fn chat_with_echo_backend() {
        let state = AppState::new(ApiConfig::default());
        state
            .set_inference_backend(Arc::new(nous_ai::EchoBackend::new()))
            .await;

        let app = Router::new()
            .route("/agents", post(create_agent))
            .route("/chat", post(chat))
            .route("/conversations/{conversation_id}", get(get_conversation))
            .with_state(state);

        // Create agent
        let res = app
            .clone()
            .oneshot(json_request(
                "POST",
                "/agents",
                serde_json::json!({
                    "name": "Echo Bot",
                    "system_prompt": "You are helpful."
                }),
            ))
            .await
            .unwrap();
        let body = body_json(res).await;
        let agent_id = body["id"].as_str().unwrap().to_string();

        // Send chat — echo backend returns the user message
        let res = app
            .clone()
            .oneshot(json_request(
                "POST",
                "/chat",
                serde_json::json!({
                    "agent_id": agent_id,
                    "message": "Hello!"
                }),
            ))
            .await
            .unwrap();
        assert_eq!(res.status(), 200);
        let body = body_json(res).await;
        assert_eq!(body["response"].as_str().unwrap(), "Hello!");
        let conv_id = body["conversation_id"].as_str().unwrap().to_string();

        // Get conversation history
        let res = app
            .oneshot(get_request(&format!("/conversations/{conv_id}")))
            .await
            .unwrap();
        assert_eq!(res.status(), 200);
        let body = body_json(res).await;
        let messages = body.as_array().unwrap();
        // system + user + assistant = 3
        assert_eq!(messages.len(), 3);
    }

    #[tokio::test]
    async fn chat_empty_message_fails() {
        let app = app();

        // Create agent
        let res = app
            .clone()
            .oneshot(json_request(
                "POST",
                "/agents",
                serde_json::json!({"name": "Bot"}),
            ))
            .await
            .unwrap();
        let body = body_json(res).await;
        let agent_id = body["id"].as_str().unwrap().to_string();

        let res = app
            .oneshot(json_request(
                "POST",
                "/chat",
                serde_json::json!({
                    "agent_id": agent_id,
                    "message": ""
                }),
            ))
            .await
            .unwrap();
        assert_eq!(res.status(), 400);
    }

    #[tokio::test]
    async fn chat_missing_agent_fails() {
        let app = app();
        let res = app
            .oneshot(json_request(
                "POST",
                "/chat",
                serde_json::json!({
                    "agent_id": "agent:nonexistent",
                    "message": "hello"
                }),
            ))
            .await
            .unwrap();
        assert_eq!(res.status(), 404);
    }

    #[tokio::test]
    async fn list_all_conversations() {
        let app = app();

        let res = app.oneshot(get_request("/conversations")).await.unwrap();
        assert_eq!(res.status(), 200);
        let body = body_json(res).await;
        assert!(body.as_array().unwrap().is_empty());
    }

    #[tokio::test]
    async fn remove_conversation() {
        let state = AppState::new(ApiConfig::default());
        state
            .set_inference_backend(Arc::new(nous_ai::EchoBackend::new()))
            .await;

        let app = Router::new()
            .route("/agents", post(create_agent))
            .route("/chat", post(chat))
            .route(
                "/conversations/{conversation_id}",
                delete(delete_conversation),
            )
            .with_state(state);

        // Create agent + conversation
        let res = app
            .clone()
            .oneshot(json_request(
                "POST",
                "/agents",
                serde_json::json!({"name": "Bot"}),
            ))
            .await
            .unwrap();
        let body = body_json(res).await;
        let agent_id = body["id"].as_str().unwrap().to_string();

        let res = app
            .clone()
            .oneshot(json_request(
                "POST",
                "/chat",
                serde_json::json!({
                    "agent_id": agent_id,
                    "message": "hi"
                }),
            ))
            .await
            .unwrap();
        assert_eq!(res.status(), 200);
        let body = body_json(res).await;
        let conv_id = body["conversation_id"].as_str().unwrap().to_string();

        let res = app
            .oneshot(delete_request(&format!("/conversations/{conv_id}")))
            .await
            .unwrap();
        assert_eq!(res.status(), 200);
    }

    #[tokio::test]
    async fn get_missing_conversation_404() {
        let app = app();
        let res = app
            .oneshot(get_request("/conversations/conv:none"))
            .await
            .unwrap();
        assert_eq!(res.status(), 404);
    }
}
