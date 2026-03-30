use async_trait::async_trait;
use serde::{Deserialize, Serialize};

use nous_core::Error;

use crate::agent::Agent;
use crate::conversation::{Conversation, Message, Role};

/// A request to complete a conversation using an agent's configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompletionRequest {
    pub agent: Agent,
    pub conversation: Conversation,
    pub stream: bool,
}

/// The response from an inference backend.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompletionResponse {
    pub message: Message,
    pub usage: TokenUsage,
    pub finish_reason: FinishReason,
}

/// Token counts for a completion.
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct TokenUsage {
    pub prompt_tokens: u32,
    pub completion_tokens: u32,
}

impl TokenUsage {
    pub fn total(&self) -> u32 {
        self.prompt_tokens + self.completion_tokens
    }
}

/// Why the model stopped generating.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum FinishReason {
    /// The model finished naturally.
    Stop,
    /// The response hit the max token limit.
    Length,
    /// The model wants to call a tool.
    ToolUse,
}

/// Trait for inference backends. Implement this to connect the agent framework
/// to a real LLM (OpenAI, Anthropic, local GGML, etc.).
#[async_trait]
pub trait InferenceBackend: Send + Sync {
    /// Send a completion request and return the response.
    async fn complete(&self, request: CompletionRequest) -> Result<CompletionResponse, Error>;
    /// The model identifier this backend uses.
    fn model_id(&self) -> &str;
    /// Whether this backend supports tool calling.
    fn supports_tools(&self) -> bool;
}

/// A step in a multi-agent pipeline.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PipelineStep {
    pub name: String,
    pub agent_id: String,
    pub input_key: String,
    pub output_key: String,
}

impl PipelineStep {
    pub fn new(
        name: impl Into<String>,
        agent_id: impl Into<String>,
        input_key: impl Into<String>,
        output_key: impl Into<String>,
    ) -> Self {
        Self {
            name: name.into(),
            agent_id: agent_id.into(),
            input_key: input_key.into(),
            output_key: output_key.into(),
        }
    }
}

/// A pipeline chains multiple agents in sequence, passing output from one step
/// as input to the next.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Pipeline {
    pub name: String,
    pub steps: Vec<PipelineStep>,
}

impl Pipeline {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            steps: Vec::new(),
        }
    }

    pub fn add_step(&mut self, step: PipelineStep) {
        self.steps.push(step);
    }

    pub fn step_count(&self) -> usize {
        self.steps.len()
    }
}

/// A simple echo backend for testing and development.
///
/// This backend doesn't do real inference -- it echoes the last user message
/// back as an assistant response, optionally prefixed with a configurable
/// string. Useful for testing the executor loop without a real LLM.
pub struct EchoBackend {
    prefix: String,
}

impl EchoBackend {
    /// Create an echo backend with no prefix.
    pub fn new() -> Self {
        Self {
            prefix: String::new(),
        }
    }

    /// Create an echo backend that prefixes all responses.
    pub fn with_prefix(prefix: impl Into<String>) -> Self {
        Self {
            prefix: prefix.into(),
        }
    }
}

impl Default for EchoBackend {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl InferenceBackend for EchoBackend {
    async fn complete(&self, request: CompletionRequest) -> Result<CompletionResponse, Error> {
        // Find the last user message to echo.
        let last_user = request
            .conversation
            .messages
            .iter()
            .rev()
            .find(|m| m.role == Role::User)
            .map(|m| m.content.clone())
            .unwrap_or_else(|| "(no user message)".to_string());

        let content = if self.prefix.is_empty() {
            last_user.clone()
        } else {
            format!("{}{}", self.prefix, last_user)
        };

        let prompt_tokens = request
            .conversation
            .messages
            .iter()
            .map(|m| m.token_estimate() as u32)
            .sum();
        let completion_tokens = (content.len() / 4) as u32;

        Ok(CompletionResponse {
            message: Message::assistant(content),
            usage: TokenUsage {
                prompt_tokens,
                completion_tokens,
            },
            finish_reason: FinishReason::Stop,
        })
    }

    fn model_id(&self) -> &str {
        "echo"
    }

    fn supports_tools(&self) -> bool {
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn token_usage_total() {
        let usage = TokenUsage {
            prompt_tokens: 100,
            completion_tokens: 50,
        };
        assert_eq!(usage.total(), 150);
    }

    #[test]
    fn pipeline_steps() {
        let mut pipeline = Pipeline::new("analysis");
        pipeline.add_step(PipelineStep::new(
            "extract",
            "extractor",
            "raw_text",
            "entities",
        ));
        pipeline.add_step(PipelineStep::new(
            "summarize",
            "summarizer",
            "entities",
            "summary",
        ));
        assert_eq!(pipeline.step_count(), 2);
    }

    #[test]
    fn pipeline_serializes() {
        let mut pipeline = Pipeline::new("test");
        pipeline.add_step(PipelineStep::new("step1", "agent1", "in", "out"));
        let json = serde_json::to_string(&pipeline).unwrap();
        let restored: Pipeline = serde_json::from_str(&json).unwrap();
        assert_eq!(restored.step_count(), 1);
    }

    #[test]
    fn finish_reasons() {
        assert_ne!(FinishReason::Stop, FinishReason::Length);
        assert_ne!(FinishReason::Stop, FinishReason::ToolUse);
    }

    #[tokio::test]
    async fn echo_backend_echoes_user_message() {
        let backend = EchoBackend::new();
        let agent = Agent::new("test", "Test");
        let mut conv = Conversation::new("test");
        conv.add_message(Message::user("hello world"));

        let response = backend
            .complete(CompletionRequest {
                agent,
                conversation: conv,
                stream: false,
            })
            .await
            .unwrap();

        assert_eq!(response.message.content, "hello world");
        assert_eq!(response.finish_reason, FinishReason::Stop);
    }

    #[tokio::test]
    async fn echo_backend_with_prefix() {
        let backend = EchoBackend::with_prefix("Echo: ");
        let agent = Agent::new("test", "Test");
        let mut conv = Conversation::new("test");
        conv.add_message(Message::user("test input"));

        let response = backend
            .complete(CompletionRequest {
                agent,
                conversation: conv,
                stream: false,
            })
            .await
            .unwrap();

        assert_eq!(response.message.content, "Echo: test input");
    }

    #[tokio::test]
    async fn echo_backend_no_user_message() {
        let backend = EchoBackend::new();
        let agent = Agent::new("test", "Test");
        let mut conv = Conversation::new("test");
        conv.add_message(Message::system("system prompt"));

        let response = backend
            .complete(CompletionRequest {
                agent,
                conversation: conv,
                stream: false,
            })
            .await
            .unwrap();

        assert_eq!(response.message.content, "(no user message)");
    }

    #[test]
    fn echo_backend_model_id() {
        let backend = EchoBackend::new();
        assert_eq!(backend.model_id(), "echo");
        assert!(!backend.supports_tools());
    }
}
