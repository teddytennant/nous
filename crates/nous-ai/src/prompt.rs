use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use nous_core::{Error, Result};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PromptTemplate {
    pub name: String,
    pub template: String,
    pub variables: Vec<String>,
    pub description: Option<String>,
}

impl PromptTemplate {
    pub fn new(name: &str, template: &str) -> Self {
        let variables = Self::extract_variables(template);
        Self {
            name: name.into(),
            template: template.into(),
            variables,
            description: None,
        }
    }

    pub fn with_description(mut self, desc: impl Into<String>) -> Self {
        self.description = Some(desc.into());
        self
    }

    fn extract_variables(template: &str) -> Vec<String> {
        let mut vars = Vec::new();
        let mut chars = template.chars().peekable();

        while let Some(c) = chars.next() {
            if c == '{' && chars.peek() == Some(&'{') {
                chars.next();
                let mut var = String::new();
                for ch in chars.by_ref() {
                    if ch == '}' {
                        if chars.peek() == Some(&'}') {
                            chars.next();
                        }
                        break;
                    }
                    var.push(ch);
                }
                let trimmed = var.trim().to_string();
                if !trimmed.is_empty() && !vars.contains(&trimmed) {
                    vars.push(trimmed);
                }
            }
        }

        vars
    }

    pub fn render(&self, vars: &HashMap<String, String>) -> Result<String> {
        let mut result = self.template.clone();

        for var_name in &self.variables {
            let placeholder = format!("{{{{{var_name}}}}}");
            let value = vars
                .get(var_name)
                .ok_or_else(|| Error::InvalidInput(format!("missing variable: {var_name}")))?;
            result = result.replace(&placeholder, value);
        }

        Ok(result)
    }

    pub fn render_partial(&self, vars: &HashMap<String, String>) -> String {
        let mut result = self.template.clone();

        for (key, value) in vars {
            let placeholder = format!("{{{{{key}}}}}");
            result = result.replace(&placeholder, value);
        }

        result
    }

    pub fn has_variable(&self, name: &str) -> bool {
        self.variables.iter().any(|v| v == name)
    }

    pub fn variable_count(&self) -> usize {
        self.variables.len()
    }
}

#[derive(Debug, Default)]
pub struct PromptLibrary {
    templates: HashMap<String, PromptTemplate>,
}

impl PromptLibrary {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn register(&mut self, template: PromptTemplate) {
        self.templates.insert(template.name.clone(), template);
    }

    pub fn get(&self, name: &str) -> Option<&PromptTemplate> {
        self.templates.get(name)
    }

    pub fn render(&self, name: &str, vars: &HashMap<String, String>) -> Result<String> {
        let template = self
            .get(name)
            .ok_or_else(|| Error::NotFound(format!("template not found: {name}")))?;
        template.render(vars)
    }

    pub fn names(&self) -> Vec<&str> {
        self.templates.keys().map(|s| s.as_str()).collect()
    }

    pub fn len(&self) -> usize {
        self.templates.len()
    }

    pub fn is_empty(&self) -> bool {
        self.templates.is_empty()
    }

    pub fn with_defaults(mut self) -> Self {
        self.register(PromptTemplate::new(
            "summarize",
            "Summarize the following text concisely:\n\n{{text}}",
        ).with_description("Summarize text"));

        self.register(PromptTemplate::new(
            "analyze_proposal",
            "Analyze this governance proposal and provide your assessment:\n\nTitle: {{title}}\nDescription: {{description}}\n\nConsider: impact, risks, feasibility, and alignment with community values.",
        ).with_description("Analyze a governance proposal"));

        self.register(PromptTemplate::new(
            "explain",
            "Explain {{topic}} in {{style}} terms. Target audience: {{audience}}.",
        ).with_description("Explain a topic"));

        self.register(PromptTemplate::new(
            "code_review",
            "Review this code for security, correctness, and performance:\n\n```{{language}}\n{{code}}\n```",
        ).with_description("Review code"));

        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extract_variables() {
        let tmpl = PromptTemplate::new("test", "Hello {{name}}, welcome to {{place}}!");
        assert_eq!(tmpl.variables, vec!["name", "place"]);
    }

    #[test]
    fn extract_no_variables() {
        let tmpl = PromptTemplate::new("test", "No variables here");
        assert!(tmpl.variables.is_empty());
    }

    #[test]
    fn extract_deduplicates() {
        let tmpl = PromptTemplate::new("test", "{{x}} and {{x}} again");
        assert_eq!(tmpl.variables, vec!["x"]);
    }

    #[test]
    fn render_success() {
        let tmpl = PromptTemplate::new("test", "Hello {{name}}, welcome to {{place}}!");
        let vars = HashMap::from([
            ("name".into(), "Alice".into()),
            ("place".into(), "Nous".into()),
        ]);
        let result = tmpl.render(&vars).unwrap();
        assert_eq!(result, "Hello Alice, welcome to Nous!");
    }

    #[test]
    fn render_missing_variable_fails() {
        let tmpl = PromptTemplate::new("test", "Hello {{name}}!");
        let vars = HashMap::new();
        assert!(tmpl.render(&vars).is_err());
    }

    #[test]
    fn render_partial() {
        let tmpl = PromptTemplate::new("test", "{{greeting}} {{name}}!");
        let vars = HashMap::from([("greeting".into(), "Hello".into())]);
        let result = tmpl.render_partial(&vars);
        assert_eq!(result, "Hello {{name}}!");
    }

    #[test]
    fn has_variable() {
        let tmpl = PromptTemplate::new("test", "{{x}} and {{y}}");
        assert!(tmpl.has_variable("x"));
        assert!(tmpl.has_variable("y"));
        assert!(!tmpl.has_variable("z"));
    }

    #[test]
    fn variable_count() {
        let tmpl = PromptTemplate::new("test", "{{a}} {{b}} {{c}}");
        assert_eq!(tmpl.variable_count(), 3);
    }

    #[test]
    fn with_description() {
        let tmpl = PromptTemplate::new("test", "hi").with_description("A test prompt");
        assert_eq!(tmpl.description.as_deref(), Some("A test prompt"));
    }

    #[test]
    fn library_register_and_get() {
        let mut lib = PromptLibrary::new();
        lib.register(PromptTemplate::new("greet", "Hello {{name}}"));

        assert!(lib.get("greet").is_some());
        assert!(lib.get("missing").is_none());
        assert_eq!(lib.len(), 1);
    }

    #[test]
    fn library_render() {
        let mut lib = PromptLibrary::new();
        lib.register(PromptTemplate::new("greet", "Hello {{name}}"));

        let vars = HashMap::from([("name".into(), "Bob".into())]);
        let result = lib.render("greet", &vars).unwrap();
        assert_eq!(result, "Hello Bob");
    }

    #[test]
    fn library_render_missing_template() {
        let lib = PromptLibrary::new();
        assert!(lib.render("nonexistent", &HashMap::new()).is_err());
    }

    #[test]
    fn library_names() {
        let mut lib = PromptLibrary::new();
        lib.register(PromptTemplate::new("a", "{{x}}"));
        lib.register(PromptTemplate::new("b", "{{y}}"));

        let mut names = lib.names();
        names.sort();
        assert_eq!(names, vec!["a", "b"]);
    }

    #[test]
    fn library_with_defaults() {
        let lib = PromptLibrary::new().with_defaults();
        assert!(lib.get("summarize").is_some());
        assert!(lib.get("analyze_proposal").is_some());
        assert!(lib.get("explain").is_some());
        assert!(lib.get("code_review").is_some());
        assert_eq!(lib.len(), 4);
    }

    #[test]
    fn defaults_render() {
        let lib = PromptLibrary::new().with_defaults();
        let vars = HashMap::from([("text".into(), "The quick brown fox.".into())]);
        let result = lib.render("summarize", &vars).unwrap();
        assert!(result.contains("The quick brown fox."));
    }

    #[test]
    fn template_serializes() {
        let tmpl = PromptTemplate::new("test", "Hello {{name}}")
            .with_description("greeting");
        let json = serde_json::to_string(&tmpl).unwrap();
        let restored: PromptTemplate = serde_json::from_str(&json).unwrap();
        assert_eq!(restored.name, "test");
        assert_eq!(restored.variables, vec!["name"]);
    }

    #[test]
    fn complex_template() {
        let tmpl = PromptTemplate::new(
            "analysis",
            "Analyze the following:\n\nContext: {{context}}\nData: {{data}}\n\nProvide insights on {{focus_area}}.",
        );
        assert_eq!(tmpl.variable_count(), 3);

        let vars = HashMap::from([
            ("context".into(), "governance".into()),
            ("data".into(), "vote results".into()),
            ("focus_area".into(), "participation rates".into()),
        ]);
        let result = tmpl.render(&vars).unwrap();
        assert!(result.contains("governance"));
        assert!(result.contains("vote results"));
        assert!(result.contains("participation rates"));
    }
}
