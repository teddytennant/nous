use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use nous_core::Error;

use crate::agent::{Agent, ToolExecutor};
use crate::conversation::{Conversation, Message};
use crate::pipeline::{CompletionRequest, FinishReason, InferenceBackend, TokenUsage};

/// Configuration for agent execution.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionConfig {
    /// Maximum number of inference + tool steps before stopping.
    pub max_steps: usize,
    /// Whether to inject available tool descriptions into the system prompt.
    pub inject_tools: bool,
}

impl Default for ExecutionConfig {
    fn default() -> Self {
        Self {
            max_steps: 16,
            inject_tools: true,
        }
    }
}

/// A single step in the execution trace.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionStep {
    pub step_index: usize,
    pub kind: StepKind,
    pub usage: TokenUsage,
}

/// What happened in a step.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum StepKind {
    /// The model produced a text response.
    Response { content: String },
    /// The model requested a tool call which was executed.
    ToolCall {
        tool_name: String,
        args: HashMap<String, serde_json::Value>,
        result: String,
    },
}

/// Final result of running an agent to completion.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionResult {
    /// The final assistant response.
    pub response: String,
    /// Complete execution trace.
    pub steps: Vec<ExecutionStep>,
    /// Aggregated token usage.
    pub total_usage: TokenUsage,
    /// Whether the agent hit the step limit.
    pub truncated: bool,
}

/// Runs an agent in a loop: inference → tool call → inference → ... → final response.
///
/// The executor drives the conversation forward by calling the backend, detecting
/// tool-use finish reasons, executing the requested tool, feeding the result back,
/// and repeating until the model stops or the step limit is reached.
pub async fn run_agent(
    agent: &Agent,
    conversation: &mut Conversation,
    backend: &dyn InferenceBackend,
    tools: Option<&dyn ToolExecutor>,
    config: &ExecutionConfig,
) -> Result<ExecutionResult, Error> {
    // Inject system prompt if conversation is empty.
    if conversation.is_empty() && !agent.system_prompt.is_empty() {
        let mut system_content = agent.system_prompt.clone();
        if config.inject_tools
            && let Some(executor) = tools
        {
            let tool_descs = format_tool_descriptions(executor);
            if !tool_descs.is_empty() {
                system_content.push_str("\n\nAvailable tools:\n");
                system_content.push_str(&tool_descs);
            }
        }
        conversation.add_message(Message::system(system_content));
    }

    let mut steps = Vec::new();
    let mut total_prompt = 0u32;
    let mut total_completion = 0u32;
    let mut truncated = false;
    let mut final_response = String::new();

    for step_idx in 0..config.max_steps {
        let request = CompletionRequest {
            agent: agent.clone(),
            conversation: conversation.clone(),
            stream: false,
        };

        let response = backend.complete(request).await?;
        total_prompt += response.usage.prompt_tokens;
        total_completion += response.usage.completion_tokens;

        match response.finish_reason {
            FinishReason::ToolUse => {
                // Parse tool call from the response content.
                let (tool_name, args) = parse_tool_call(&response.message.content)?;

                // Execute the tool.
                let tool_result = if let Some(executor) = tools {
                    match executor.execute(&tool_name, &args).await {
                        Ok(val) => serde_json::to_string(&val).unwrap_or_default(),
                        Err(e) => format!("Tool error: {e}"),
                    }
                } else {
                    "No tool executor available".to_string()
                };

                // Add assistant message and tool result to conversation.
                conversation.add_message(response.message);
                conversation.add_message(Message::tool_result(
                    &tool_result,
                    format!("call-{step_idx}"),
                ));

                steps.push(ExecutionStep {
                    step_index: step_idx,
                    kind: StepKind::ToolCall {
                        tool_name,
                        args,
                        result: tool_result,
                    },
                    usage: response.usage,
                });
            }
            FinishReason::Stop | FinishReason::Length => {
                final_response = response.message.content.clone();
                conversation.add_message(response.message);

                steps.push(ExecutionStep {
                    step_index: step_idx,
                    kind: StepKind::Response {
                        content: final_response.clone(),
                    },
                    usage: response.usage,
                });
                break;
            }
        }

        if step_idx == config.max_steps - 1 {
            truncated = true;
        }
    }

    Ok(ExecutionResult {
        response: final_response,
        steps,
        total_usage: TokenUsage {
            prompt_tokens: total_prompt,
            completion_tokens: total_completion,
        },
        truncated,
    })
}

fn format_tool_descriptions(executor: &dyn ToolExecutor) -> String {
    let tools = executor.available_tools();
    let mut out = String::new();
    for tool in tools {
        out.push_str(&format!("- {}: {}\n", tool.name, tool.description));
        for (name, param) in &tool.parameters {
            let req = if param.required {
                "required"
            } else {
                "optional"
            };
            out.push_str(&format!(
                "  - {name} ({}, {req}): {}\n",
                param.param_type, param.description
            ));
        }
    }
    out
}

/// Parse a tool call from model output. Expected format:
/// ```json
/// {"tool": "name", "args": {"key": "value"}}
/// ```
fn parse_tool_call(content: &str) -> Result<(String, HashMap<String, serde_json::Value>), Error> {
    // Try to extract JSON from the content (model may wrap it in markdown).
    let json_str = extract_json(content).unwrap_or(content);

    let val: serde_json::Value = serde_json::from_str(json_str)
        .map_err(|e| Error::Other(format!("failed to parse tool call: {e}")))?;

    let tool_name = val
        .get("tool")
        .and_then(|v| v.as_str())
        .ok_or_else(|| Error::Other("tool call missing 'tool' field".into()))?
        .to_string();

    let args = val
        .get("args")
        .and_then(|v| v.as_object())
        .map(|obj| obj.iter().map(|(k, v)| (k.clone(), v.clone())).collect())
        .unwrap_or_default();

    Ok((tool_name, args))
}

/// Extract the first JSON object from text that may contain markdown fences.
fn extract_json(text: &str) -> Option<&str> {
    // Look for ```json ... ``` first.
    if let Some(start) = text.find("```json") {
        let after = &text[start + 7..];
        if let Some(end) = after.find("```") {
            return Some(after[..end].trim());
        }
    }
    // Look for first { ... last }.
    let start = text.find('{')?;
    let end = text.rfind('}')?;
    if end > start {
        Some(&text[start..=end])
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::agent::ToolDefinition;
    use crate::conversation::Role;
    use crate::pipeline::CompletionResponse;
    use async_trait::async_trait;

    // ── Mock Backend ────────────────────────────────────────────────

    struct MockBackend {
        responses: std::sync::Mutex<Vec<CompletionResponse>>,
    }

    impl MockBackend {
        fn new(responses: Vec<CompletionResponse>) -> Self {
            Self {
                responses: std::sync::Mutex::new(responses),
            }
        }
    }

    #[async_trait]
    impl InferenceBackend for MockBackend {
        async fn complete(&self, _request: CompletionRequest) -> Result<CompletionResponse, Error> {
            let mut responses = self.responses.lock().unwrap();
            if responses.is_empty() {
                return Err(Error::Other("no more mock responses".into()));
            }
            Ok(responses.remove(0))
        }

        fn model_id(&self) -> &str {
            "mock-model"
        }

        fn supports_tools(&self) -> bool {
            true
        }
    }

    fn mock_response(content: &str, reason: FinishReason) -> CompletionResponse {
        CompletionResponse {
            message: Message::assistant(content),
            usage: TokenUsage {
                prompt_tokens: 10,
                completion_tokens: 5,
            },
            finish_reason: reason,
        }
    }

    // ── Mock Tool Executor ──────────────────────────────────────────

    struct MockTools;

    #[async_trait]
    impl ToolExecutor for MockTools {
        async fn execute(
            &self,
            tool_name: &str,
            args: &HashMap<String, serde_json::Value>,
        ) -> Result<serde_json::Value, Error> {
            match tool_name {
                "search" => {
                    let query = args
                        .get("query")
                        .and_then(|v| v.as_str())
                        .unwrap_or("unknown");
                    Ok(serde_json::json!({
                        "results": [{"title": "Result for query", "query": query}]
                    }))
                }
                "calculate" => {
                    let expr = args
                        .get("expression")
                        .and_then(|v| v.as_str())
                        .unwrap_or("0");
                    Ok(serde_json::json!({"result": expr, "value": 42}))
                }
                _ => Err(Error::NotFound(format!("unknown tool: {tool_name}"))),
            }
        }

        fn available_tools(&self) -> Vec<ToolDefinition> {
            vec![
                ToolDefinition::new("search", "Search the knowledge base").with_param(
                    "query",
                    "string",
                    "The search query",
                    true,
                ),
                ToolDefinition::new("calculate", "Evaluate a math expression").with_param(
                    "expression",
                    "string",
                    "The expression",
                    true,
                ),
            ]
        }
    }

    // ── Tests ───────────────────────────────────────────────────────

    #[tokio::test]
    async fn simple_response() {
        let backend = MockBackend::new(vec![mock_response(
            "Hello! How can I help?",
            FinishReason::Stop,
        )]);
        let agent = Agent::new("test", "Test Agent").with_system_prompt("You are helpful.");
        let mut conv = Conversation::new("test");
        conv.add_message(Message::user("Hi"));

        let result = run_agent(
            &agent,
            &mut conv,
            &backend,
            None,
            &ExecutionConfig::default(),
        )
        .await
        .unwrap();

        assert_eq!(result.response, "Hello! How can I help?");
        assert_eq!(result.steps.len(), 1);
        assert!(!result.truncated);
        assert!(matches!(result.steps[0].kind, StepKind::Response { .. }));
    }

    #[tokio::test]
    async fn tool_call_then_response() {
        let backend = MockBackend::new(vec![
            mock_response(
                r#"{"tool": "search", "args": {"query": "Nous architecture"}}"#,
                FinishReason::ToolUse,
            ),
            mock_response(
                "Based on the search results, Nous uses a modular architecture.",
                FinishReason::Stop,
            ),
        ]);

        let agent = Agent::new("test", "Test Agent");
        let mut conv = Conversation::new("test");
        conv.add_message(Message::user("Describe the architecture"));

        let result = run_agent(
            &agent,
            &mut conv,
            &backend,
            Some(&MockTools),
            &ExecutionConfig::default(),
        )
        .await
        .unwrap();

        assert_eq!(result.steps.len(), 2);
        assert!(matches!(result.steps[0].kind, StepKind::ToolCall { .. }));
        assert!(matches!(result.steps[1].kind, StepKind::Response { .. }));
        assert!(result.response.contains("modular architecture"));
    }

    #[tokio::test]
    async fn multiple_tool_calls() {
        let backend = MockBackend::new(vec![
            mock_response(
                r#"{"tool": "search", "args": {"query": "identity"}}"#,
                FinishReason::ToolUse,
            ),
            mock_response(
                r#"{"tool": "calculate", "args": {"expression": "2+2"}}"#,
                FinishReason::ToolUse,
            ),
            mock_response("Done researching.", FinishReason::Stop),
        ]);

        let agent = Agent::new("test", "Test Agent");
        let mut conv = Conversation::new("test");
        conv.add_message(Message::user("Research and calculate"));

        let result = run_agent(
            &agent,
            &mut conv,
            &backend,
            Some(&MockTools),
            &ExecutionConfig::default(),
        )
        .await
        .unwrap();

        assert_eq!(result.steps.len(), 3);
        assert_eq!(result.response, "Done researching.");
        assert!(!result.truncated);
    }

    #[tokio::test]
    async fn max_steps_truncation() {
        // All responses are tool calls — should truncate at max_steps.
        let responses: Vec<_> = (0..5)
            .map(|_| {
                mock_response(
                    r#"{"tool": "search", "args": {"query": "loop"}}"#,
                    FinishReason::ToolUse,
                )
            })
            .collect();

        let backend = MockBackend::new(responses);
        let agent = Agent::new("test", "Test Agent");
        let mut conv = Conversation::new("test");
        conv.add_message(Message::user("Go"));

        let config = ExecutionConfig {
            max_steps: 3,
            inject_tools: false,
        };

        let result = run_agent(&agent, &mut conv, &backend, Some(&MockTools), &config)
            .await
            .unwrap();

        assert!(result.truncated);
        assert_eq!(result.steps.len(), 3);
    }

    #[tokio::test]
    async fn token_usage_aggregation() {
        let backend = MockBackend::new(vec![
            mock_response(
                r#"{"tool": "search", "args": {"query": "x"}}"#,
                FinishReason::ToolUse,
            ),
            mock_response("Final.", FinishReason::Stop),
        ]);

        let agent = Agent::new("test", "Test Agent");
        let mut conv = Conversation::new("test");
        conv.add_message(Message::user("Go"));

        let result = run_agent(
            &agent,
            &mut conv,
            &backend,
            Some(&MockTools),
            &ExecutionConfig {
                max_steps: 10,
                inject_tools: false,
            },
        )
        .await
        .unwrap();

        // 2 calls × 10 prompt + 5 completion each
        assert_eq!(result.total_usage.prompt_tokens, 20);
        assert_eq!(result.total_usage.completion_tokens, 10);
        assert_eq!(result.total_usage.total(), 30);
    }

    #[tokio::test]
    async fn system_prompt_injected() {
        let backend = MockBackend::new(vec![mock_response("OK", FinishReason::Stop)]);

        let agent = Agent::new("test", "Test Agent").with_system_prompt("You are Nous.");
        let mut conv = Conversation::new("test");
        // Empty conversation — system prompt should be injected.

        let result = run_agent(
            &agent,
            &mut conv,
            &backend,
            Some(&MockTools),
            &ExecutionConfig::default(),
        )
        .await
        .unwrap();

        assert_eq!(result.response, "OK");
        // Conversation should now have system + assistant messages.
        let system_msgs = conv.messages_by_role(Role::System);
        assert_eq!(system_msgs.len(), 1);
        assert!(system_msgs[0].content.contains("You are Nous."));
        assert!(system_msgs[0].content.contains("Available tools:"));
    }

    #[tokio::test]
    async fn tool_error_handled() {
        let backend = MockBackend::new(vec![
            mock_response(
                r#"{"tool": "nonexistent", "args": {}}"#,
                FinishReason::ToolUse,
            ),
            mock_response("I got an error.", FinishReason::Stop),
        ]);

        let agent = Agent::new("test", "Test Agent");
        let mut conv = Conversation::new("test");
        conv.add_message(Message::user("Try something"));

        let result = run_agent(
            &agent,
            &mut conv,
            &backend,
            Some(&MockTools),
            &ExecutionConfig {
                max_steps: 10,
                inject_tools: false,
            },
        )
        .await
        .unwrap();

        // The tool error should be captured in the step, not crash the executor.
        if let StepKind::ToolCall { result: res, .. } = &result.steps[0].kind {
            assert!(res.contains("Tool error"));
        } else {
            panic!("expected tool call step");
        }
    }

    #[test]
    fn parse_tool_call_basic() {
        let (name, args) =
            parse_tool_call(r#"{"tool": "search", "args": {"query": "hello"}}"#).unwrap();
        assert_eq!(name, "search");
        assert_eq!(args["query"], "hello");
    }

    #[test]
    fn parse_tool_call_with_markdown() {
        let content =
            "Let me search:\n```json\n{\"tool\": \"search\", \"args\": {\"query\": \"test\"}}\n```";
        let (name, _) = parse_tool_call(content).unwrap();
        assert_eq!(name, "search");
    }

    #[test]
    fn parse_tool_call_no_args() {
        let (name, args) = parse_tool_call(r#"{"tool": "list"}"#).unwrap();
        assert_eq!(name, "list");
        assert!(args.is_empty());
    }

    #[test]
    fn parse_tool_call_invalid_json() {
        assert!(parse_tool_call("not json at all").is_err());
    }

    #[test]
    fn extract_json_from_text() {
        let text = "Some text before {\"key\": \"value\"} after";
        let extracted = extract_json(text).unwrap();
        assert_eq!(extracted, "{\"key\": \"value\"}");
    }

    #[test]
    fn config_default() {
        let config = ExecutionConfig::default();
        assert_eq!(config.max_steps, 16);
        assert!(config.inject_tools);
    }

    #[test]
    fn config_serializes() {
        let config = ExecutionConfig {
            max_steps: 8,
            inject_tools: false,
        };
        let json = serde_json::to_string(&config).unwrap();
        let restored: ExecutionConfig = serde_json::from_str(&json).unwrap();
        assert_eq!(restored.max_steps, 8);
        assert!(!restored.inject_tools);
    }
}
