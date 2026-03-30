use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use nous_core::Error;

/// An AI agent with a name, system prompt, model configuration, and capabilities.
///
/// Agents are the primary unit of work in the Nous AI framework. Each agent
/// has a system prompt that defines its behavior, a set of capabilities (strings
/// describing what the agent can do), and model parameters controlling inference.
///
/// Agents don't execute themselves -- they're data objects passed to an
/// [`InferenceBackend`](crate::pipeline::InferenceBackend) via the
/// [`run_agent`](crate::executor::run_agent) executor.
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
    /// Create a new agent with the given id and name. All other fields are set
    /// to sensible defaults: empty system prompt, "nous-local" model, 0.7
    /// temperature, 4096 max tokens.
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

    /// Set the temperature. Clamped to `[0.0, 2.0]`.
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

/// Describes a tool that an agent can call during execution.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolDefinition {
    pub name: String,
    pub description: String,
    pub parameters: HashMap<String, ToolParameter>,
}

/// A parameter accepted by a tool.
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

    /// Returns the names of all required parameters.
    pub fn required_params(&self) -> Vec<&str> {
        self.parameters
            .iter()
            .filter(|(_, p)| p.required)
            .map(|(name, _)| name.as_str())
            .collect()
    }
}

/// Trait for executing tools. Implement this to give agents the ability to
/// call external functions during inference.
#[async_trait]
pub trait ToolExecutor: Send + Sync {
    /// Execute a tool by name with the given arguments.
    async fn execute(
        &self,
        tool_name: &str,
        args: &HashMap<String, serde_json::Value>,
    ) -> Result<serde_json::Value, Error>;

    /// Return the definitions of all available tools.
    fn available_tools(&self) -> Vec<ToolDefinition>;
}

/// A registry of tool definitions and their executor functions.
///
/// Use this when you want to compose tools from multiple sources into a single
/// executor. Each tool is registered with a name and an async handler function.
pub struct ToolRegistry {
    tools: HashMap<String, RegisteredTool>,
}

type BoxFuture<T> = std::pin::Pin<Box<dyn std::future::Future<Output = T> + Send>>;

struct RegisteredTool {
    definition: ToolDefinition,
    handler: Box<
        dyn Fn(HashMap<String, serde_json::Value>) -> BoxFuture<Result<serde_json::Value, Error>>
            + Send
            + Sync,
    >,
}

impl ToolRegistry {
    /// Create an empty tool registry.
    pub fn new() -> Self {
        Self {
            tools: HashMap::new(),
        }
    }

    /// Register a tool with its definition and an async handler function.
    ///
    /// The handler receives the tool arguments (owned) and returns a JSON result.
    /// If a tool with the same name already exists, it is replaced.
    pub fn register<F, Fut>(&mut self, definition: ToolDefinition, handler: F)
    where
        F: Fn(HashMap<String, serde_json::Value>) -> Fut + Send + Sync + 'static,
        Fut: std::future::Future<Output = Result<serde_json::Value, Error>> + Send + 'static,
    {
        let name = definition.name.clone();
        self.tools.insert(
            name,
            RegisteredTool {
                definition,
                handler: Box::new(move |args| Box::pin(handler(args))),
            },
        );
    }

    /// Check whether a tool with the given name is registered.
    pub fn has_tool(&self, name: &str) -> bool {
        self.tools.contains_key(name)
    }

    /// Get a tool definition by name.
    pub fn get_definition(&self, name: &str) -> Option<&ToolDefinition> {
        self.tools.get(name).map(|t| &t.definition)
    }

    /// Number of registered tools.
    pub fn len(&self) -> usize {
        self.tools.len()
    }

    /// Whether the registry is empty.
    pub fn is_empty(&self) -> bool {
        self.tools.is_empty()
    }

    /// Names of all registered tools.
    pub fn tool_names(&self) -> Vec<&str> {
        self.tools.keys().map(|s| s.as_str()).collect()
    }

    /// Remove a tool by name. Returns true if the tool was found and removed.
    pub fn remove(&mut self, name: &str) -> bool {
        self.tools.remove(name).is_some()
    }
}

impl Default for ToolRegistry {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl ToolExecutor for ToolRegistry {
    async fn execute(
        &self,
        tool_name: &str,
        args: &HashMap<String, serde_json::Value>,
    ) -> Result<serde_json::Value, Error> {
        let tool = self
            .tools
            .get(tool_name)
            .ok_or_else(|| Error::NotFound(format!("tool not found: {tool_name}")))?;
        (tool.handler)(args.clone()).await
    }

    fn available_tools(&self) -> Vec<ToolDefinition> {
        self.tools.values().map(|t| t.definition.clone()).collect()
    }
}

// Manual Debug because the handler closure isn't Debug.
impl std::fmt::Debug for ToolRegistry {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ToolRegistry")
            .field("tools", &self.tools.keys().collect::<Vec<_>>())
            .finish()
    }
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
    fn tool_definition_required_params() {
        let tool = ToolDefinition::new("search", "Search")
            .with_param("query", "string", "The query", true)
            .with_param("limit", "number", "Max results", false)
            .with_param("filter", "string", "Filter expression", true);

        let required = tool.required_params();
        assert_eq!(required.len(), 2);
        assert!(required.contains(&"query"));
        assert!(required.contains(&"filter"));
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

    #[test]
    fn tool_registry_register_and_lookup() {
        let mut registry = ToolRegistry::new();
        assert!(registry.is_empty());

        let def = ToolDefinition::new("echo", "Echo the input");
        registry.register(def, |args: HashMap<String, serde_json::Value>| async move {
            Ok(serde_json::json!({"echoed": args}))
        });

        assert_eq!(registry.len(), 1);
        assert!(registry.has_tool("echo"));
        assert!(!registry.has_tool("missing"));
        assert_eq!(
            registry.get_definition("echo").unwrap().description,
            "Echo the input"
        );
    }

    #[test]
    fn tool_registry_remove() {
        let mut registry = ToolRegistry::new();
        registry.register(ToolDefinition::new("x", "X"), |_| async move {
            Ok(serde_json::Value::Null)
        });

        assert!(registry.remove("x"));
        assert!(!registry.remove("x"));
        assert!(registry.is_empty());
    }

    #[test]
    fn tool_registry_names() {
        let mut registry = ToolRegistry::new();
        registry.register(ToolDefinition::new("a", "A"), |_| async move {
            Ok(serde_json::Value::Null)
        });
        registry.register(ToolDefinition::new("b", "B"), |_| async move {
            Ok(serde_json::Value::Null)
        });

        let mut names = registry.tool_names();
        names.sort();
        assert_eq!(names, vec!["a", "b"]);
    }

    #[test]
    fn tool_registry_available_tools() {
        let mut registry = ToolRegistry::new();
        registry.register(ToolDefinition::new("search", "Search"), |_| async move {
            Ok(serde_json::Value::Null)
        });

        let tools = registry.available_tools();
        assert_eq!(tools.len(), 1);
        assert_eq!(tools[0].name, "search");
    }

    #[tokio::test]
    async fn tool_registry_execute() {
        let mut registry = ToolRegistry::new();
        registry.register(
            ToolDefinition::new("greet", "Greet someone"),
            |args: HashMap<String, serde_json::Value>| async move {
                let name = args
                    .get("name")
                    .and_then(|v| v.as_str())
                    .unwrap_or("world")
                    .to_string();
                Ok(serde_json::json!({"greeting": format!("hello, {name}")}))
            },
        );

        let args = HashMap::from([("name".to_string(), serde_json::json!("Alice"))]);
        let result = registry.execute("greet", &args).await.unwrap();
        assert_eq!(result["greeting"], "hello, Alice");
    }

    #[tokio::test]
    async fn tool_registry_execute_missing_tool() {
        let registry = ToolRegistry::new();
        let result = registry.execute("missing", &HashMap::new()).await;
        assert!(result.is_err());
    }

    #[test]
    fn tool_registry_debug() {
        let registry = ToolRegistry::new();
        let debug = format!("{registry:?}");
        assert!(debug.contains("ToolRegistry"));
    }
}
