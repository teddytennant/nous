use async_trait::async_trait;
use serde::{Deserialize, Serialize};

use nous_core::Error;

use crate::agent::Agent;
use crate::conversation::{Conversation, Message};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompletionRequest {
    pub agent: Agent,
    pub conversation: Conversation,
    pub stream: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompletionResponse {
    pub message: Message,
    pub usage: TokenUsage,
    pub finish_reason: FinishReason,
}

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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum FinishReason {
    Stop,
    Length,
    ToolUse,
}

#[async_trait]
pub trait InferenceBackend: Send + Sync {
    async fn complete(&self, request: CompletionRequest) -> Result<CompletionResponse, Error>;
    fn model_id(&self) -> &str;
    fn supports_tools(&self) -> bool;
}

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
        pipeline.add_step(PipelineStep::new("extract", "extractor", "raw_text", "entities"));
        pipeline.add_step(PipelineStep::new("summarize", "summarizer", "entities", "summary"));
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
}
