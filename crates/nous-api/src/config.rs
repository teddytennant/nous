use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiConfig {
    pub host: String,
    pub port: u16,
    pub enable_graphql: bool,
    pub enable_grpc: bool,
    pub rate_limit_rpm: u32,
}

impl Default for ApiConfig {
    fn default() -> Self {
        Self {
            host: "0.0.0.0".to_string(),
            port: 8080,
            enable_graphql: true,
            enable_grpc: true,
            rate_limit_rpm: 600,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_config() {
        let config = ApiConfig::default();
        assert_eq!(config.port, 8080);
        assert!(config.enable_graphql);
    }
}
