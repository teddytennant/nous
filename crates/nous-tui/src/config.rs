use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TuiConfig {
    pub theme: String,
    pub show_timestamps: bool,
    pub max_visible_messages: usize,
    pub api_url: String,
}

impl Default for TuiConfig {
    fn default() -> Self {
        Self {
            theme: "dark".to_string(),
            show_timestamps: true,
            max_visible_messages: 100,
            api_url: "http://localhost:8080/api/v1".to_string(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_config() {
        let config = TuiConfig::default();
        assert_eq!(config.theme, "dark");
    }
}
