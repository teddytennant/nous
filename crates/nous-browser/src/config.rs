use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BrowserConfig {
    pub ipfs_gateway: String,
    pub enable_ens: bool,
    pub enable_ipfs: bool,
    pub sovereign_mode: bool,
}

impl Default for BrowserConfig {
    fn default() -> Self {
        Self {
            ipfs_gateway: "http://localhost:8080".to_string(),
            enable_ens: true,
            enable_ipfs: true,
            sovereign_mode: false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_config() {
        let config = BrowserConfig::default();
        assert!(config.enable_ens);
        assert!(!config.sovereign_mode);
    }
}
