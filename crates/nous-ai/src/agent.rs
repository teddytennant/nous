use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use nous_core::Error;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Agent {
    pub id: String,
    pub name: String,
    pub system_prompt: String,
    pub capabilities: Vec<String>,
    pub model: String,
    pub temperature: f32,
    pub max_tokens: u32,
}

impl Agent {
    pub fn new(id: impl Into<String>, name: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            name: name.into(),
            system_prompt: String::new(),
            capabilities: Vec::new(),
            model: "nous-local".to_string(),
            temperature: 0.7,
            max_tokens: 4096,
        }
    }

    pub fn with_system_prompt(mut self, prompt: impl Into<String>) -> Self {
        self.system_prompt = prompt.into();
        self
    }

    pub fn with_capability(mut self, cap: impl Into<String>) -> Self {
        self.capabilities.push(cap.into());
        self
    }

    pub fn with_model(mut self, model: impl Into<String>) -> Self {
        self.model = model.into();
        self
    }

    pub fn with_temperature(mut self, temp: f32) -> Self {
        self.temperature = temp.clamp(0.0, 2.0);
        self
    }

    pub fn with_max_tokens(mut self, max: u32) -> Self {
        self.max_tokens = max;
        self
    }

    pub fn has_capability(&self, cap: &str) -> bool {
        self.capabilities.iter().any(|c| c == cap)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolDefinition {
    pub name: String,
    pub description: String,
    pub parameters: HashMap<String, ToolParameter>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolParameter {
    pub param_type: String,
    pub description: String,
    pub required: bool,
}

impl ToolDefinition {
    pub fn new(name: impl Into<String>, description: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            description: description.into(),
            parameters: HashMap::new(),
        }
    }

    pub fn with_param(
        mut self,
        name: impl Into<String>,
        param_type: impl Into<String>,
        description: impl Into<String>,
        required: bool,
    ) -> Self {
        self.parameters.insert(
            name.into(),
            ToolParameter {
                param_type: param_type.into(),
                description: description.into(),
                required,
            },
        );
        self
    }
}

#[async_trait]
pub trait ToolExecutor: Send + Sync {
    async fn execute(
        &self,
        tool_name: &str,
        args: &HashMap<String, serde_json::Value>,
    ) -> Result<serde_json::Value, Error>;

    fn available_tools(&self) -> Vec<ToolDefinition>;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn create_agent() {
        let agent = Agent::new("agent-1", "Researcher")
            .with_system_prompt("You are a research agent.")
            .with_capability("search")
            .with_capability("summarize")
            .with_model("claude-sonnet-4-20250514")
            .with_temperature(0.5);

        assert_eq!(agent.name, "Researcher");
        assert!(agent.has_capability("search"));
        assert!(!agent.has_capability("code"));
        assert_eq!(agent.temperature, 0.5);
    }

    #[test]
    fn temperature_clamped() {
        let agent = Agent::new("test", "test").with_temperature(5.0);
        assert_eq!(agent.temperature, 2.0);

        let agent = Agent::new("test", "test").with_temperature(-1.0);
        assert_eq!(agent.temperature, 0.0);
    }

    #[test]
    fn tool_definition() {
        let tool = ToolDefinition::new("search", "Search the knowledge base")
            .with_param("query", "string", "The search query", true)
            .with_param("limit", "number", "Max results", false);

        assert_eq!(tool.parameters.len(), 2);
        assert!(tool.parameters["query"].required);
        assert!(!tool.parameters["limit"].required);
    }

    #[test]
    fn agent_serializes() {
        let agent = Agent::new("test", "Test Agent")
            .with_system_prompt("Test")
            .with_capability("analyze");
        let json = serde_json::to_string(&agent).unwrap();
        let restored: Agent = serde_json::from_str(&json).unwrap();
        assert_eq!(restored.capabilities, agent.capabilities);
    }
}
