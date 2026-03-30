//! Bridges the application-level [`GossipEngine`] with the node's P2P layer.
//!
//! While libp2p gossipsub handles transport-level pub/sub, the gossip engine
//! adds causal ordering via vector clocks, application-level deduplication,
//! TTL-based flood control, and pull-based anti-entropy for offline sync.
//!
//! The bridge translates between gossipsub messages and gossip engine events,
//! producing [`GossipAction`]s that the node should execute (forwarding,
//! sending digests, replying with missing messages).

use std::collections::HashMap;
use std::time::Duration;

use nous_net::gossip::{GossipAction, GossipConfig, GossipEngine, GossipMessage};
use serde::{Deserialize, Serialize};
use tracing::{debug, warn};

/// Configuration for the gossip bridge.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GossipBridgeConfig {
    /// Underlying gossip engine configuration.
    pub engine: GossipConfig,
    /// Interval at which to run the engine tick (GC + sync).
    pub tick_interval: Duration,
    /// Maximum peers to track in the gossip layer.
    pub max_peers: usize,
}

impl Default for GossipBridgeConfig {
    fn default() -> Self {
        Self {
            engine: GossipConfig::default(),
            tick_interval: Duration::from_secs(30),
            max_peers: 256,
        }
    }
}

/// Statistics about the gossip bridge.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GossipStats {
    pub messages_originated: u64,
    pub messages_received: u64,
    pub messages_forwarded: u64,
    pub duplicates_dropped: u64,
    pub active_peers: usize,
    pub buffered_messages: usize,
}

/// Bridges the [`GossipEngine`] with the node event system.
///
/// Call [`originate`] to publish a new message, [`handle_incoming`] to
/// process received gossipsub data, and [`tick`] periodically for
/// maintenance. Each method returns a list of [`GossipAction`]s that the
/// node's network layer should execute.
pub struct GossipBridge {
    engine: GossipEngine,
    config: GossipBridgeConfig,
    originated_count: u64,
}

impl GossipBridge {
    /// Create a new gossip bridge for the given node ID.
    pub fn new(local_id: String, config: GossipBridgeConfig) -> Self {
        let engine = GossipEngine::new(local_id, config.engine.clone());
        Self {
            engine,
            config,
            originated_count: 0,
        }
    }

    /// Register a remote peer in the gossip layer.
    pub fn add_peer(&mut self, peer_id: String) {
        if self.engine.peer_count() >= self.config.max_peers {
            warn!(
                peer_id,
                max = self.config.max_peers,
                "gossip bridge at max peers, ignoring"
            );
            return;
        }
        self.engine.add_peer(peer_id);
    }

    /// Remove a peer from the gossip layer.
    pub fn remove_peer(&mut self, peer_id: &str) {
        self.engine.remove_peer(peer_id);
    }

    /// Originate a new message from this node.
    /// Returns the gossip message and actions to disseminate it.
    pub fn originate(&mut self, payload: Vec<u8>) -> (GossipMessage, Vec<GossipAction>) {
        self.originated_count += 1;
        let (msg, actions) = self.engine.originate(payload);
        debug!(
            msg_id = %msg.id,
            actions = actions.len(),
            "originated gossip message"
        );
        (msg, actions)
    }

    /// Handle a raw incoming message from a peer.
    /// Returns actions to take and whether the message was new.
    pub fn handle_incoming(&mut self, data: &[u8], from_peer: &str) -> (Vec<GossipAction>, bool) {
        let msg: GossipMessage = match serde_json::from_slice(data) {
            Ok(m) => m,
            Err(e) => {
                warn!(error = %e, from = from_peer, "failed to decode gossip message");
                return (Vec::new(), false);
            }
        };

        let (actions, is_new) = self.engine.receive(msg, from_peer);

        if is_new {
            debug!(from = from_peer, "accepted new gossip message");
        }

        (actions, is_new)
    }

    /// Periodic tick: garbage collection and anti-entropy sync.
    /// Returns actions (digest sends, etc.) the node should execute.
    pub fn tick(&mut self) -> Vec<GossipAction> {
        self.engine.tick()
    }

    /// Record a sync failure for backoff tracking.
    pub fn record_sync_failure(&mut self, peer_id: &str) {
        self.engine.record_sync_failure(peer_id);
    }

    /// Record a successful sync (resets backoff).
    pub fn record_sync_success(&mut self, peer_id: &str) {
        self.engine.record_sync_success(peer_id);
    }

    /// Get current statistics.
    pub fn stats(&self) -> GossipStats {
        GossipStats {
            messages_originated: self.originated_count,
            messages_received: self.engine.received_count(),
            messages_forwarded: self.engine.forwarded_count(),
            duplicates_dropped: self.engine.duplicate_count(),
            active_peers: self.engine.peer_count(),
            buffered_messages: self.engine.message_count(),
        }
    }

    /// Tick interval from config.
    pub fn tick_interval(&self) -> Duration {
        self.config.tick_interval
    }
}

/// Serialize a [`GossipAction`] into bytes that can be sent over the network.
pub fn encode_gossip_message(msg: &GossipMessage) -> Result<Vec<u8>, serde_json::Error> {
    serde_json::to_vec(msg)
}

/// Decode a gossip message from network bytes.
pub fn decode_gossip_message(data: &[u8]) -> Result<GossipMessage, serde_json::Error> {
    serde_json::from_slice(data)
}

/// Map of peer ID to the serialized messages that need to be sent.
pub fn resolve_actions(actions: &[GossipAction]) -> HashMap<String, Vec<Vec<u8>>> {
    let mut outbox: HashMap<String, Vec<Vec<u8>>> = HashMap::new();

    for action in actions {
        match action {
            GossipAction::Forward { message, targets } => {
                if let Ok(data) = encode_gossip_message(message) {
                    for target in targets {
                        outbox.entry(target.clone()).or_default().push(data.clone());
                    }
                }
            }
            GossipAction::SendMissing { peer, messages } => {
                for msg in messages {
                    if let Ok(data) = encode_gossip_message(msg) {
                        outbox.entry(peer.clone()).or_default().push(data);
                    }
                }
            }
            GossipAction::SendDigest { peer, digest } => {
                if let Ok(data) = serde_json::to_vec(digest) {
                    outbox.entry(peer.clone()).or_default().push(data);
                }
            }
        }
    }

    outbox
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_bridge(id: &str) -> GossipBridge {
        GossipBridge::new(
            id.to_string(),
            GossipBridgeConfig {
                engine: GossipConfig {
                    fanout: 3,
                    max_message_age: Duration::from_secs(60),
                    max_buffer_size: 100,
                    sync_interval: Duration::from_millis(10),
                    max_ttl: 6,
                },
                tick_interval: Duration::from_secs(5),
                max_peers: 50,
            },
        )
    }

    #[test]
    fn bridge_creates() {
        let bridge = make_bridge("node-a");
        let stats = bridge.stats();
        assert_eq!(stats.messages_originated, 0);
        assert_eq!(stats.active_peers, 0);
    }

    #[test]
    fn bridge_add_peer() {
        let mut bridge = make_bridge("node-a");
        bridge.add_peer("peer-1".into());
        bridge.add_peer("peer-2".into());
        assert_eq!(bridge.stats().active_peers, 2);
    }

    #[test]
    fn bridge_remove_peer() {
        let mut bridge = make_bridge("node-a");
        bridge.add_peer("peer-1".into());
        bridge.remove_peer("peer-1");
        assert_eq!(bridge.stats().active_peers, 0);
    }

    #[test]
    fn bridge_originate() {
        let mut bridge = make_bridge("node-a");
        bridge.add_peer("peer-1".into());

        let (msg, actions) = bridge.originate(b"hello".to_vec());
        assert_eq!(msg.origin, "node-a");
        assert!(!actions.is_empty());

        let stats = bridge.stats();
        assert_eq!(stats.messages_originated, 1);
        assert_eq!(stats.buffered_messages, 1);
    }

    #[test]
    fn bridge_handle_incoming() {
        let mut bridge_a = make_bridge("node-a");
        let mut bridge_b = make_bridge("node-b");
        bridge_b.add_peer("node-a".into());

        let (msg, _) = bridge_a.originate(b"from-a".to_vec());
        let encoded = encode_gossip_message(&msg).unwrap();

        let (actions, is_new) = bridge_b.handle_incoming(&encoded, "node-a");
        assert!(is_new);
        assert_eq!(bridge_b.stats().messages_received, 1);
    }

    #[test]
    fn bridge_rejects_duplicate() {
        let mut bridge_a = make_bridge("node-a");
        let mut bridge_b = make_bridge("node-b");
        bridge_b.add_peer("node-a".into());

        let (msg, _) = bridge_a.originate(b"dup".to_vec());
        let encoded = encode_gossip_message(&msg).unwrap();

        let (_, first) = bridge_b.handle_incoming(&encoded, "node-a");
        assert!(first);

        let (_, second) = bridge_b.handle_incoming(&encoded, "node-a");
        assert!(!second);
        assert_eq!(bridge_b.stats().duplicates_dropped, 1);
    }

    #[test]
    fn bridge_handle_garbage() {
        let mut bridge = make_bridge("node-a");
        let (_, is_new) = bridge.handle_incoming(b"not json", "peer-1");
        assert!(!is_new);
    }

    #[test]
    fn bridge_tick() {
        let mut bridge = make_bridge("node-a");
        bridge.add_peer("peer-1".into());
        let actions = bridge.tick();
        // May or may not produce actions depending on timing.
        let _ = actions;
    }

    #[test]
    fn bridge_max_peers() {
        let mut bridge = GossipBridge::new(
            "node-a".into(),
            GossipBridgeConfig {
                max_peers: 3,
                ..Default::default()
            },
        );

        bridge.add_peer("p1".into());
        bridge.add_peer("p2".into());
        bridge.add_peer("p3".into());
        bridge.add_peer("p4".into()); // Should be ignored.

        assert_eq!(bridge.stats().active_peers, 3);
    }

    #[test]
    fn bridge_sync_backoff() {
        let mut bridge = make_bridge("node-a");
        bridge.add_peer("peer-1".into());

        bridge.record_sync_failure("peer-1");
        bridge.record_sync_failure("peer-1");
        // After failures, backoff increases.

        bridge.record_sync_success("peer-1");
        // After success, backoff resets.
    }

    #[test]
    fn resolve_actions_forward() {
        let mut bridge = make_bridge("node-a");
        bridge.add_peer("peer-1".into());
        bridge.add_peer("peer-2".into());

        let (_, actions) = bridge.originate(b"broadcast".to_vec());
        let outbox = resolve_actions(&actions);
        assert!(!outbox.is_empty());
    }

    #[test]
    fn encode_decode_roundtrip() {
        let mut bridge = make_bridge("node-a");
        let (msg, _) = bridge.originate(b"test".to_vec());

        let encoded = encode_gossip_message(&msg).unwrap();
        let decoded = decode_gossip_message(&encoded).unwrap();
        assert_eq!(decoded.id, msg.id);
        assert_eq!(decoded.payload, msg.payload);
    }

    #[test]
    fn stats_serializes() {
        let bridge = make_bridge("node-a");
        let stats = bridge.stats();
        let json = serde_json::to_string(&stats).unwrap();
        let restored: GossipStats = serde_json::from_str(&json).unwrap();
        assert_eq!(restored.messages_originated, 0);
    }

    #[test]
    fn config_serializes() {
        let config = GossipBridgeConfig::default();
        let json = serde_json::to_string(&config).unwrap();
        let restored: GossipBridgeConfig = serde_json::from_str(&json).unwrap();
        assert_eq!(restored.max_peers, config.max_peers);
    }

    #[test]
    fn tick_interval() {
        let bridge = make_bridge("node-a");
        assert_eq!(bridge.tick_interval(), Duration::from_secs(5));
    }
}
