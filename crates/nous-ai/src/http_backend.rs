//! HTTP inference backend for OpenAI-compatible APIs.
//!
//! Connects the agent framework to real LLMs via any API that implements
//! the OpenAI chat completions format (OpenAI, Anthropic via proxy,
//! ollama, vLLM, LM Studio, etc.).

use async_trait::async_trait;
use serde::{Deserialize, Serialize};

use nous_core::Error;

use crate::conversation::Role;
use crate::pipeline::{
    CompletionRequest, CompletionResponse, FinishReason, InferenceBackend, TokenUsage,
};

/// An inference backend that talks to OpenAI-compatible `/v1/chat/completions`.
///
/// # Example
///
/// ```no_run
/// use nous_ai::HttpInferenceBackend;
///
/// let backend = HttpInferenceBackend::new(
///     "https://api.openai.com/v1/chat/completions",
///     "sk-...",
///     "gpt-4o",
/// );
/// ```
pub struct HttpInferenceBackend {
    endpoint: String,
    api_key: String,
    model: String,
    client: reqwest::Client,
}

impl HttpInferenceBackend {
    /// Create a backend pointing at a specific completions endpoint.
    ///
    /// - `endpoint`: Full URL, e.g. `https://api.openai.com/v1/chat/completions`
    /// - `api_key`: Bearer token for authentication (can be empty for local models)
    /// - `model`: Model ID to request, e.g. `gpt-4o`, `claude-sonnet-4-20250514`, `llama3`
    pub fn new(
        endpoint: impl Into<String>,
        api_key: impl Into<String>,
        model: impl Into<String>,
    ) -> Self {
        Self {
            endpoint: endpoint.into(),
            api_key: api_key.into(),
            model: model.into(),
            client: reqwest::Client::new(),
        }
    }

    /// Create a backend for OpenAI's API with just an API key and model.
    pub fn openai(api_key: impl Into<String>, model: impl Into<String>) -> Self {
        Self::new(
            "https://api.openai.com/v1/chat/completions",
            api_key,
            model,
        )
    }

    /// Create a backend for a local ollama instance.
    pub fn ollama(model: impl Into<String>) -> Self {
        Self::new(
            "http://localhost:11434/v1/chat/completions",
            "",
            model,
        )
    }
}

// ── OpenAI-compatible request/response types ──────────────────────────

#[derive(Debug, Serialize)]
struct ChatRequest {
    model: String,
    messages: Vec<ChatMessage>,
    #[serde(skip_serializing_if = "Option::is_none")]
    temperature: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    max_tokens: Option<u32>,
}

#[derive(Debug, Serialize, Deserialize)]
struct ChatMessage {
    role: String,
    content: String,
}

#[derive(Debug, Deserialize)]
struct ChatResponseBody {
    choices: Vec<Choice>,
    #[serde(default)]
    usage: Option<Usage>,
}

#[derive(Debug, Deserialize)]
struct Choice {
    message: ChatMessage,
    finish_reason: Option<String>,
}

#[derive(Debug, Deserialize)]
struct Usage {
    prompt_tokens: u32,
    completion_tokens: u32,
}

fn role_to_string(role: Role) -> String {
    match role {
        Role::System => "system".into(),
        Role::User => "user".into(),
        Role::Assistant => "assistant".into(),
        Role::ToolCall => "assistant".into(),
        Role::Tool => "tool".into(),
    }
}

fn parse_finish_reason(reason: Option<&str>) -> FinishReason {
    match reason {
        Some("stop") => FinishReason::Stop,
        Some("length") => FinishReason::Length,
        Some("tool_calls") => FinishReason::ToolUse,
        _ => FinishReason::Stop,
    }
}

#[async_trait]
impl InferenceBackend for HttpInferenceBackend {
    async fn complete(&self, request: CompletionRequest) -> Result<CompletionResponse, Error> {
        let messages: Vec<ChatMessage> = request
            .conversation
            .messages
            .iter()
            .map(|m| ChatMessage {
                role: role_to_string(m.role),
                content: m.content.clone(),
            })
            .collect();

        let body = ChatRequest {
            model: self.model.clone(),
            messages,
            temperature: Some(request.agent.temperature),
            max_tokens: Some(request.agent.max_tokens),
        };

        let mut req = self
            .client
            .post(&self.endpoint)
            .header("Content-Type", "application/json");

        if !self.api_key.is_empty() {
            req = req.header("Authorization", format!("Bearer {}", self.api_key));
        }

        let response = req
            .json(&body)
            .send()
            .await
            .map_err(|e| Error::Other(format!("HTTP request failed: {e}")))?;

        let status = response.status();
        if !status.is_success() {
            let error_body = response
                .text()
                .await
                .unwrap_or_else(|_| "unknown error".into());
            return Err(Error::Other(format!(
                "inference API returned HTTP {status}: {error_body}"
            )));
        }

        let resp: ChatResponseBody = response
            .json()
            .await
            .map_err(|e| Error::Other(format!("failed to parse response: {e}")))?;

        let choice = resp
            .choices
            .into_iter()
            .next()
            .ok_or_else(|| Error::Other("no choices in response".into()))?;

        let usage = resp.usage.unwrap_or(Usage {
            prompt_tokens: 0,
            completion_tokens: 0,
        });

        Ok(CompletionResponse {
            message: crate::conversation::Message::assistant(&choice.message.content),
            usage: TokenUsage {
                prompt_tokens: usage.prompt_tokens,
                completion_tokens: usage.completion_tokens,
            },
            finish_reason: parse_finish_reason(choice.finish_reason.as_deref()),
        })
    }

    fn model_id(&self) -> &str {
        &self.model
    }

    fn supports_tools(&self) -> bool {
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::agent::Agent;
    use crate::conversation::{Conversation, Message};

    #[test]
    fn role_mapping() {
        assert_eq!(role_to_string(Role::System), "system");
        assert_eq!(role_to_string(Role::User), "user");
        assert_eq!(role_to_string(Role::Assistant), "assistant");
        assert_eq!(role_to_string(Role::ToolCall), "assistant");
        assert_eq!(role_to_string(Role::Tool), "tool");
    }

    #[test]
    fn finish_reason_parsing() {
        assert_eq!(parse_finish_reason(Some("stop")), FinishReason::Stop);
        assert_eq!(parse_finish_reason(Some("length")), FinishReason::Length);
        assert_eq!(
            parse_finish_reason(Some("tool_calls")),
            FinishReason::ToolUse
        );
        assert_eq!(parse_finish_reason(None), FinishReason::Stop);
        assert_eq!(parse_finish_reason(Some("unknown")), FinishReason::Stop);
    }

    #[test]
    fn openai_constructor() {
        let backend = HttpInferenceBackend::openai("sk-test", "gpt-4o");
        assert_eq!(backend.model_id(), "gpt-4o");
        assert!(backend.endpoint.contains("openai.com"));
        assert!(backend.supports_tools());
    }

    #[test]
    fn ollama_constructor() {
        let backend = HttpInferenceBackend::ollama("llama3");
        assert_eq!(backend.model_id(), "llama3");
        assert!(backend.endpoint.contains("localhost:11434"));
        assert!(backend.api_key.is_empty());
    }

    #[test]
    fn request_serialization() {
        let body = ChatRequest {
            model: "gpt-4o".into(),
            messages: vec![
                ChatMessage {
                    role: "system".into(),
                    content: "You are helpful.".into(),
                },
                ChatMessage {
                    role: "user".into(),
                    content: "Hello".into(),
                },
            ],
            temperature: Some(0.7),
            max_tokens: Some(1024),
        };
        let json = serde_json::to_string(&body).unwrap();
        assert!(json.contains("gpt-4o"));
        assert!(json.contains("Hello"));
        assert!(json.contains("0.7"));
    }

    #[test]
    fn response_deserialization() {
        let json = r#"{
            "choices": [{
                "message": {"role": "assistant", "content": "Hello!"},
                "finish_reason": "stop"
            }],
            "usage": {"prompt_tokens": 10, "completion_tokens": 5}
        }"#;
        let resp: ChatResponseBody = serde_json::from_str(json).unwrap();
        assert_eq!(resp.choices.len(), 1);
        assert_eq!(resp.choices[0].message.content, "Hello!");
        assert_eq!(resp.usage.unwrap().prompt_tokens, 10);
    }

    #[test]
    fn response_without_usage() {
        let json = r#"{
            "choices": [{
                "message": {"role": "assistant", "content": "Hi"},
                "finish_reason": "stop"
            }]
        }"#;
        let resp: ChatResponseBody = serde_json::from_str(json).unwrap();
        assert!(resp.usage.is_none());
    }

    #[test]
    fn optional_fields_skipped_when_none() {
        let body = ChatRequest {
            model: "test".into(),
            messages: vec![],
            temperature: None,
            max_tokens: None,
        };
        let json = serde_json::to_string(&body).unwrap();
        assert!(!json.contains("temperature"));
        assert!(!json.contains("max_tokens"));
    }

    #[tokio::test]
    async fn unreachable_endpoint_returns_error() {
        let backend =
            HttpInferenceBackend::new("http://127.0.0.1:19998/v1/chat/completions", "", "test");
        let agent = Agent::new("a", "A");
        let mut conv = Conversation::new("a");
        conv.add_message(Message::user("hello"));

        let result = backend
            .complete(CompletionRequest {
                agent,
                conversation: conv,
                stream: false,
            })
            .await;

        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(
            err.contains("HTTP request failed"),
            "unexpected error: {err}"
        );
    }
}
