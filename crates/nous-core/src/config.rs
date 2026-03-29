use std::net::{Ipv4Addr, SocketAddr};
use std::path::PathBuf;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Config {
    pub node: NodeConfig,
    pub network: NetworkConfig,
    pub storage: StorageConfig,
    pub crypto: CryptoConfig,
    pub api: ApiConfig,
    pub ui: UiConfig,
}

impl Config {
    pub fn data_dir(&self) -> &PathBuf {
        &self.storage.data_dir
    }

    pub fn listen_addr(&self) -> SocketAddr {
        self.network.listen_addr
    }

    pub fn api_addr(&self) -> SocketAddr {
        self.api.bind_addr
    }

    pub fn validate(&self) -> crate::Result<()> {
        if self.node.name.is_empty() {
            return Err(crate::Error::InvalidInput("node name cannot be empty".into()));
        }
        if self.network.max_peers == 0 {
            return Err(crate::Error::InvalidInput("max_peers must be positive".into()));
        }
        if self.api.rate_limit_per_minute == 0 {
            return Err(crate::Error::InvalidInput(
                "rate_limit_per_minute must be positive".into(),
            ));
        }
        if self.crypto.argon2_memory_kib < 1024 {
            return Err(crate::Error::InvalidInput(
                "argon2 memory must be at least 1024 KiB".into(),
            ));
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeConfig {
    pub name: String,
    pub data_dir: PathBuf,
    pub log_level: LogLevel,
    pub max_memory_mb: u64,
}

impl Default for NodeConfig {
    fn default() -> Self {
        Self {
            name: "nous-node".into(),
            data_dir: default_data_dir(),
            log_level: LogLevel::Info,
            max_memory_mb: 512,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum LogLevel {
    Trace,
    Debug,
    Info,
    Warn,
    Error,
}

impl LogLevel {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Trace => "trace",
            Self::Debug => "debug",
            Self::Info => "info",
            Self::Warn => "warn",
            Self::Error => "error",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkConfig {
    pub listen_addr: SocketAddr,
    pub external_addr: Option<SocketAddr>,
    pub bootstrap_peers: Vec<String>,
    pub max_peers: u32,
    pub enable_relay: bool,
    pub enable_mdns: bool,
    pub gossip_interval_ms: u64,
    pub connection_timeout_secs: u64,
}

impl Default for NetworkConfig {
    fn default() -> Self {
        Self {
            listen_addr: SocketAddr::new(Ipv4Addr::UNSPECIFIED.into(), 9000),
            external_addr: None,
            bootstrap_peers: Vec::new(),
            max_peers: 50,
            enable_relay: true,
            enable_mdns: true,
            gossip_interval_ms: 1000,
            connection_timeout_secs: 30,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StorageConfig {
    pub data_dir: PathBuf,
    pub max_db_size_mb: u64,
    pub wal_mode: bool,
    pub cache_size_pages: u32,
    pub ipfs_api_url: Option<String>,
}

impl Default for StorageConfig {
    fn default() -> Self {
        Self {
            data_dir: default_data_dir(),
            max_db_size_mb: 1024,
            wal_mode: true,
            cache_size_pages: 2000,
            ipfs_api_url: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CryptoConfig {
    pub argon2_memory_kib: u32,
    pub argon2_iterations: u32,
    pub argon2_parallelism: u32,
    pub key_rotation_days: u32,
}

impl Default for CryptoConfig {
    fn default() -> Self {
        Self {
            argon2_memory_kib: 65536,
            argon2_iterations: 3,
            argon2_parallelism: 4,
            key_rotation_days: 90,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiConfig {
    pub bind_addr: SocketAddr,
    pub enable_graphql: bool,
    pub enable_grpc: bool,
    pub enable_rest: bool,
    pub cors_origins: Vec<String>,
    pub rate_limit_per_minute: u32,
    pub max_request_body_bytes: usize,
}

impl Default for ApiConfig {
    fn default() -> Self {
        Self {
            bind_addr: SocketAddr::new(Ipv4Addr::LOCALHOST.into(), 3000),
            enable_graphql: true,
            enable_grpc: true,
            enable_rest: true,
            cors_origins: vec!["http://localhost:3001".into()],
            rate_limit_per_minute: 120,
            max_request_body_bytes: 10 * 1024 * 1024,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UiConfig {
    pub theme: Theme,
    pub accent_color: String,
    pub font_mono: String,
    pub font_sans: String,
    pub animations_enabled: bool,
    pub animation_duration_ms: u32,
}

impl Default for UiConfig {
    fn default() -> Self {
        Self {
            theme: Theme::Dark,
            accent_color: "#d4af37".into(),
            font_mono: "JetBrains Mono".into(),
            font_sans: "Inter".into(),
            animations_enabled: true,
            animation_duration_ms: 200,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Theme {
    Dark,
    Light,
}

fn default_data_dir() -> PathBuf {
    std::env::var("HOME")
        .map(|h| PathBuf::from(h).join(".nous"))
        .unwrap_or_else(|_| PathBuf::from(".nous"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_config_is_valid() {
        let config = Config::default();
        assert!(config.validate().is_ok());
    }

    #[test]
    fn empty_node_name_rejected() {
        let mut config = Config::default();
        config.node.name = String::new();
        assert!(config.validate().is_err());
    }

    #[test]
    fn zero_max_peers_rejected() {
        let mut config = Config::default();
        config.network.max_peers = 0;
        assert!(config.validate().is_err());
    }

    #[test]
    fn zero_rate_limit_rejected() {
        let mut config = Config::default();
        config.api.rate_limit_per_minute = 0;
        assert!(config.validate().is_err());
    }

    #[test]
    fn low_argon2_memory_rejected() {
        let mut config = Config::default();
        config.crypto.argon2_memory_kib = 512;
        assert!(config.validate().is_err());
    }

    #[test]
    fn config_serializes_roundtrip() {
        let config = Config::default();
        let json = serde_json::to_string_pretty(&config).unwrap();
        let restored: Config = serde_json::from_str(&json).unwrap();
        assert_eq!(restored.node.name, "nous-node");
        assert_eq!(restored.ui.accent_color, "#d4af37");
    }

    #[test]
    fn dark_theme_is_default() {
        let config = Config::default();
        assert_eq!(config.ui.theme, Theme::Dark);
    }

    #[test]
    fn api_defaults_localhost() {
        let config = Config::default();
        assert_eq!(config.api.bind_addr.port(), 3000);
        assert!(config.api.bind_addr.ip().is_loopback());
    }

    #[test]
    fn network_defaults_any_addr() {
        let config = Config::default();
        assert!(config.network.listen_addr.ip().is_unspecified());
        assert_eq!(config.network.listen_addr.port(), 9000);
    }

    #[test]
    fn log_level_as_str() {
        assert_eq!(LogLevel::Trace.as_str(), "trace");
        assert_eq!(LogLevel::Debug.as_str(), "debug");
        assert_eq!(LogLevel::Info.as_str(), "info");
        assert_eq!(LogLevel::Warn.as_str(), "warn");
        assert_eq!(LogLevel::Error.as_str(), "error");
    }

    #[test]
    fn custom_config() {
        let mut config = Config::default();
        config.node.name = "my-node".into();
        config.network.max_peers = 100;
        config.ui.accent_color = "#ff0000".into();
        config.crypto.key_rotation_days = 30;

        assert!(config.validate().is_ok());
        assert_eq!(config.node.name, "my-node");
        assert_eq!(config.network.max_peers, 100);
    }

    #[test]
    fn data_dir_accessor() {
        let config = Config::default();
        assert!(config.data_dir().ends_with(".nous"));
    }

    #[test]
    fn listen_addr_accessor() {
        let config = Config::default();
        assert_eq!(config.listen_addr().port(), 9000);
    }

    #[test]
    fn api_addr_accessor() {
        let config = Config::default();
        assert_eq!(config.api_addr().port(), 3000);
    }
}
