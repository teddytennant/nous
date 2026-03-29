use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeConfig {
    pub listen_addresses: Vec<String>,
    pub bootstrap_peers: Vec<String>,
    pub enable_mdns: bool,
    pub enable_relay: bool,
    pub max_connections: usize,
}

impl Default for NodeConfig {
    fn default() -> Self {
        Self {
            listen_addresses: vec!["/ip4/0.0.0.0/tcp/0".to_string()],
            bootstrap_peers: Vec::new(),
            enable_mdns: true,
            enable_relay: true,
            max_connections: 128,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_config() {
        let config = NodeConfig::default();
        assert!(config.enable_mdns);
        assert!(config.enable_relay);
        assert_eq!(config.max_connections, 128);
    }

    #[test]
    fn config_serializes() {
        let config = NodeConfig::default();
        let json = serde_json::to_string(&config).unwrap();
        let _: NodeConfig = serde_json::from_str(&json).unwrap();
    }
}
