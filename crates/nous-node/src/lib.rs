//! Node orchestrator — ties identity, storage, networking, and messaging into
//! a single coherent Nous node that can be started, used, and shut down.

pub mod gossip_bridge;
pub mod health;
pub mod plugin;
pub mod rate_limit;
pub mod routing;

use std::path::PathBuf;

use nous_identity::Identity;
use nous_net::events::NodeEvent;
use nous_net::node::NodeConfig as NetNodeConfig;
use nous_net::node::NousNode as NetNode;
use nous_storage::Database;
use serde::{Deserialize, Serialize};
use tokio::sync::mpsc;
use tokio::task::JoinHandle;
use tracing::{debug, info, warn};

use routing::{MessageRouter, RoutedMessage};

/// Top-level configuration for a Nous node.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeConfig {
    /// Directory for persistent data (SQLite DB, keys, etc.)
    pub data_dir: PathBuf,
    /// Display name for this node's identity.
    pub display_name: Option<String>,
    /// P2P network configuration.
    pub network: NetNodeConfig,
}

impl Default for NodeConfig {
    fn default() -> Self {
        let data_dir = std::env::var("HOME")
            .map(|h| PathBuf::from(h).join(".nous"))
            .unwrap_or_else(|_| PathBuf::from(".nous"));

        Self {
            data_dir,
            display_name: None,
            network: NetNodeConfig::default(),
        }
    }
}

/// The top-level Nous node that ties all subsystems together.
pub struct NousNode {
    pub identity: Identity,
    pub storage: Database,
    network: Option<NetNode>,
    event_rx: Option<mpsc::Receiver<NodeEvent>>,
    network_handle: Option<JoinHandle<()>>,
    event_handle: Option<JoinHandle<()>>,
    routed_rx: Option<mpsc::Receiver<RoutedMessage>>,
    config: NodeConfig,
}

impl NousNode {
    /// Create a new NousNode. Opens or creates the storage database and loads
    /// (or generates) an identity. Initializes the P2P network but does not
    /// start it — call [`start`] for that.
    pub fn new(config: NodeConfig) -> Result<Self, Box<dyn std::error::Error>> {
        // Ensure data directory exists.
        std::fs::create_dir_all(&config.data_dir)?;

        // Open or create the SQLite database.
        let db_path = config.data_dir.join("nous.db");
        let storage = Database::open(&db_path)?;

        // Load existing identity or generate a new one.
        let identity = load_or_create_identity(&storage, config.display_name.as_deref())?;
        info!(did = %identity.did(), "node identity ready");

        // Create the P2P network node.
        let (net_node, event_rx) = NetNode::new(&config.network)?;
        info!(peer_id = %net_node.local_peer_id(), "p2p network initialized");

        Ok(Self {
            identity,
            storage,
            network: Some(net_node),
            event_rx: Some(event_rx),
            network_handle: None,
            event_handle: None,
            routed_rx: None,
            config,
        })
    }

    /// Create a NousNode backed by an in-memory database (useful for tests).
    pub fn in_memory(config: NodeConfig) -> Result<Self, Box<dyn std::error::Error>> {
        let storage = Database::in_memory()?;
        let identity = load_or_create_identity(&storage, config.display_name.as_deref())?;
        let (net_node, event_rx) = NetNode::new(&config.network)?;

        Ok(Self {
            identity,
            storage,
            network: Some(net_node),
            event_rx: Some(event_rx),
            network_handle: None,
            event_handle: None,
            routed_rx: None,
            config,
        })
    }

    /// The DID of this node's identity.
    pub fn did(&self) -> &str {
        self.identity.did()
    }

    /// The libp2p PeerId of this node (if the network has not been started/taken).
    pub fn peer_id(&self) -> Option<libp2p::PeerId> {
        self.network.as_ref().map(|n| n.local_peer_id())
    }

    /// Reference to the node config.
    pub fn config(&self) -> &NodeConfig {
        &self.config
    }

    /// Whether the networking layer is currently running.
    pub fn is_running(&self) -> bool {
        self.network_handle.is_some()
    }

    /// Take the receiver for routed P2P messages. This is available after
    /// [`start`] is called. The caller (e.g. the API layer) should spawn a
    /// task to consume these messages and apply them to local state.
    pub fn take_routed_receiver(&mut self) -> Option<mpsc::Receiver<RoutedMessage>> {
        self.routed_rx.take()
    }

    /// Start the P2P networking layer. This listens on configured addresses,
    /// subscribes to all default gossipsub topics, and spawns background tasks
    /// for the network event loop and event handler.
    pub async fn start(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        let mut net_node = self
            .network
            .take()
            .ok_or("network already started or not initialized")?;
        let event_rx = self.event_rx.take().ok_or("event receiver already taken")?;

        // Listen on configured addresses.
        for addr in &self.config.network.listen_addresses {
            net_node.listen_on(addr)?;
            info!(%addr, "p2p listening");
        }

        // Subscribe to all default gossipsub topics.
        net_node.subscribe_all_default()?;
        info!("subscribed to all default gossipsub topics");

        // Spawn the network event loop.
        let network_handle = tokio::spawn(async move {
            net_node.run().await;
        });
        self.network_handle = Some(network_handle);

        // Create the message router.
        let (router, routed_rx) = MessageRouter::new(1024);
        self.routed_rx = Some(routed_rx);

        // Spawn the event handler with the router.
        let did = self.identity.did().to_string();
        let event_handle = tokio::spawn(handle_events(event_rx, did, router));
        self.event_handle = Some(event_handle);

        info!("nous node started");
        Ok(())
    }

    /// Shut down the node by aborting background tasks.
    pub async fn shutdown(&mut self) {
        if let Some(h) = self.network_handle.take() {
            h.abort();
            let _ = h.await;
        }
        if let Some(h) = self.event_handle.take() {
            h.abort();
            let _ = h.await;
        }
        info!("nous node shut down");
    }
}

/// Background task that processes incoming network events and routes gossipsub
/// messages to the appropriate subsystem via the [`MessageRouter`].
async fn handle_events(
    mut rx: mpsc::Receiver<NodeEvent>,
    local_did: String,
    router: MessageRouter,
) {
    while let Some(event) = rx.recv().await {
        match event {
            NodeEvent::PeerConnected(peer_id) => {
                info!(%peer_id, "peer connected");
            }
            NodeEvent::PeerDisconnected(peer_id) => {
                debug!(%peer_id, "peer disconnected");
            }
            NodeEvent::PeerDiscovered(peer_id) => {
                debug!(%peer_id, "peer discovered via mDNS");
            }
            NodeEvent::MessageReceived {
                source,
                topic,
                data,
            } => {
                debug!(
                    source = ?source,
                    %topic,
                    bytes = data.len(),
                    our_did = %local_did,
                    "gossipsub message received"
                );

                // Route the message to the appropriate subsystem.
                if let Err(e) = router.route(&topic, &data, &local_did).await {
                    warn!(error = %e, "router channel closed, stopping event handler");
                    break;
                }
            }
            NodeEvent::ListeningOn(addr) => {
                info!(%addr, "node listening on address");
            }
            NodeEvent::DhtRecordFound { key, value } => {
                debug!(%key, bytes = value.len(), "DHT record found");
            }
            NodeEvent::DhtRecordStored { key } => {
                debug!(%key, "DHT record stored");
            }
            NodeEvent::Subscribed { peer, topic } => {
                debug!(%peer, %topic, "remote peer subscribed");
            }
            NodeEvent::Unsubscribed { peer, topic } => {
                debug!(%peer, %topic, "remote peer unsubscribed");
            }
            NodeEvent::Error(err) => {
                warn!(%err, "network error");
            }
        }
    }
}

/// Load an identity from the database or generate a new one and persist it.
fn load_or_create_identity(
    db: &Database,
    display_name: Option<&str>,
) -> Result<Identity, Box<dyn std::error::Error>> {
    // Try to load the stored signing key.
    if let Some(key_bytes) = db.get_kv("identity:signing_key")? {
        let identity = Identity::restore(&key_bytes)?;
        let identity = if let Some(name) = display_name {
            identity.with_display_name(name)
        } else {
            // Try to load persisted display name.
            if let Some(name_bytes) = db.get_kv("identity:display_name")? {
                let name = String::from_utf8(name_bytes).unwrap_or_default();
                if name.is_empty() {
                    identity
                } else {
                    identity.with_display_name(name)
                }
            } else {
                identity
            }
        };
        info!(did = %identity.did(), "loaded existing identity");
        return Ok(identity);
    }

    // Generate a new identity.
    let identity = Identity::generate();
    let identity = if let Some(name) = display_name {
        identity.with_display_name(name)
    } else {
        identity
    };

    // Persist the signing key.
    let key_bytes = identity.export_signing_key();
    db.put_kv("identity:signing_key", &key_bytes)?;

    // Persist the DID document.
    let doc_json = serde_json::to_string(identity.document())?;
    db.store_identity(identity.did(), &doc_json, Some(&key_bytes))?;

    // Persist display name if set.
    if let Some(name) = identity.display_name() {
        db.put_kv("identity:display_name", name.as_bytes())?;
    }

    info!(did = %identity.did(), "generated new identity");
    Ok(identity)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_config_is_sane() {
        let config = NodeConfig::default();
        assert!(config.data_dir.ends_with(".nous"));
        assert!(config.display_name.is_none());
        assert!(config.network.enable_mdns);
    }

    #[test]
    fn config_serializes_roundtrip() {
        let config = NodeConfig {
            data_dir: PathBuf::from("/tmp/nous-test"),
            display_name: Some("TestNode".to_string()),
            network: NetNodeConfig::default(),
        };
        let json = serde_json::to_string(&config).unwrap();
        let restored: NodeConfig = serde_json::from_str(&json).unwrap();
        assert_eq!(restored.data_dir, config.data_dir);
        assert_eq!(restored.display_name, config.display_name);
    }

    #[tokio::test]
    async fn in_memory_node_creates() {
        let config = NodeConfig::default();
        let node = NousNode::in_memory(config).expect("in-memory node should create");
        assert!(node.did().starts_with("did:key:z"));
        assert!(node.peer_id().is_some());
        assert!(!node.is_running());
    }

    #[tokio::test]
    async fn in_memory_node_identity_is_consistent() {
        let config = NodeConfig::default();
        let node = NousNode::in_memory(config).unwrap();
        let did = node.did().to_string();
        // The DID should be a proper did:key
        assert!(did.starts_with("did:key:z"));
        assert!(did.len() > 20);
    }

    #[tokio::test]
    async fn node_with_display_name() {
        let config = NodeConfig {
            display_name: Some("Zarathustra".to_string()),
            ..Default::default()
        };
        let node = NousNode::in_memory(config).unwrap();
        assert_eq!(node.identity.display_name(), Some("Zarathustra"));
    }

    #[tokio::test]
    async fn file_backed_node_persists_identity() {
        let dir = tempfile::tempdir().unwrap();
        let config = NodeConfig {
            data_dir: dir.path().to_path_buf(),
            display_name: Some("Persistent".to_string()),
            network: NetNodeConfig::default(),
        };

        let did1 = {
            let node = NousNode::new(config.clone()).unwrap();
            node.did().to_string()
        };

        // Create a second node with the same data_dir — should load the same identity.
        let did2 = {
            let node = NousNode::new(config).unwrap();
            node.did().to_string()
        };

        assert_eq!(did1, did2, "identity should persist across restarts");
    }

    #[tokio::test]
    async fn node_starts_and_shuts_down() {
        let config = NodeConfig::default();
        let mut node = NousNode::in_memory(config).unwrap();

        node.start().await.expect("start should succeed");
        assert!(node.is_running());

        node.shutdown().await;
        assert!(!node.is_running());
    }

    #[tokio::test]
    async fn start_twice_fails() {
        let config = NodeConfig::default();
        let mut node = NousNode::in_memory(config).unwrap();

        node.start().await.unwrap();
        let result = node.start().await;
        assert!(result.is_err(), "starting twice should fail");

        node.shutdown().await;
    }
}
