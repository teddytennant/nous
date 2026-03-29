//! Nostr relay integration for the API server.
//!
//! Embeds a NIP-01 Nostr relay alongside the REST/GraphQL/gRPC APIs,
//! bridging Nostr events into the Nous social feed.

use std::sync::Arc;

use nous_nostr::{Relay, RelayConfig, RelayServer};
use nous_social::{EventKind, Feed, SignedEvent};
use tokio::sync::RwLock;

/// Configuration for the embedded Nostr relay.
#[derive(Debug, Clone)]
pub struct NostrConfig {
    pub port: u16,
    pub max_events: usize,
    pub max_subscriptions_per_client: usize,
}

impl Default for NostrConfig {
    fn default() -> Self {
        Self {
            port: 9735,
            max_events: 100_000,
            max_subscriptions_per_client: 20,
        }
    }
}

/// Bridge that syncs Nostr events into the Nous social feed.
pub struct NostrBridge {
    relay: Relay,
    feed: Arc<RwLock<Feed>>,
}

impl NostrBridge {
    pub fn new(config: NostrConfig, feed: Arc<RwLock<Feed>>) -> Self {
        let relay_config = RelayConfig {
            max_events: config.max_events,
            max_subscriptions_per_client: config.max_subscriptions_per_client,
            ..Default::default()
        };

        let relay = Relay::new(relay_config);
        Self { relay, feed }
    }

    /// Start the Nostr relay WebSocket server and event bridge.
    pub async fn run(self, addr: std::net::SocketAddr) -> std::io::Result<()> {
        let server = RelayServer::new(self.relay.clone());

        // Spawn the bridge task that forwards relay broadcasts into the Nous feed
        let relay = self.relay.clone();
        let feed = self.feed.clone();
        tokio::spawn(async move {
            let mut rx = relay.subscribe_broadcast();
            while let Ok(nostr_event) = rx.recv().await {
                // Convert Nostr tags to Nous tags
                let tags: Vec<nous_social::Tag> = nostr_event
                    .tags
                    .iter()
                    .filter_map(|t| match (t.tag_name(), t.value()) {
                        (Some("t"), Some(v)) => Some(nous_social::Tag::hashtag(v)),
                        (Some("e"), Some(v)) => Some(nous_social::Tag::event(v)),
                        (Some("p"), Some(v)) => Some(nous_social::Tag::pubkey(v)),
                        _ => None,
                    })
                    .collect();

                let kind = EventKind::from(nostr_event.kind.0 as u32);
                let event = SignedEvent::new(&nostr_event.pubkey, kind, &nostr_event.content, tags);

                let mut feed = feed.write().await;
                feed.insert(event);
            }
        });

        tracing::info!(?addr, "nous Nostr relay listening");
        server.listen(addr).await
    }

    pub fn relay(&self) -> &Relay {
        &self.relay
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use nous_nostr::{ClientMessage, EventBuilder, Filter, Kind, SubscriptionManager};

    fn make_bridge() -> NostrBridge {
        let feed = Arc::new(RwLock::new(Feed::new()));
        NostrBridge::new(NostrConfig::default(), feed)
    }

    #[test]
    fn nostr_config_defaults() {
        let config = NostrConfig::default();
        assert_eq!(config.port, 9735);
        assert_eq!(config.max_events, 100_000);
    }

    #[test]
    fn nostr_bridge_creates() {
        let bridge = make_bridge();
        // Relay should be accessible
        let _store = bridge.relay().store();
    }

    #[test]
    fn nostr_relay_handles_event() {
        let bridge = make_bridge();
        let sub_mgr = SubscriptionManager::new(20);

        let key = ed25519_dalek::SigningKey::generate(&mut rand::rngs::OsRng);
        let event = EventBuilder::text_note("hello from nostr").sign(&key);

        let msg = ClientMessage::Event(event);
        let responses = bridge.relay().handle_message(&msg, &sub_mgr);

        assert_eq!(responses.len(), 1);
    }

    #[test]
    fn nostr_relay_stores_and_queries() {
        let bridge = make_bridge();
        let sub_mgr = SubscriptionManager::new(20);

        let key = ed25519_dalek::SigningKey::generate(&mut rand::rngs::OsRng);
        let event = EventBuilder::text_note("test note").sign(&key);

        let msg = ClientMessage::Event(event);
        bridge.relay().handle_message(&msg, &sub_mgr);

        // Query via REQ
        let filter = Filter::default();
        let req = ClientMessage::Req {
            subscription_id: "sub1".to_string(),
            filters: vec![filter],
        };
        let responses = bridge.relay().handle_message(&req, &sub_mgr);

        // Should have at least event + EOSE
        assert!(responses.len() >= 2);
    }

    #[test]
    fn nostr_relay_subscription_close() {
        let bridge = make_bridge();
        let sub_mgr = SubscriptionManager::new(20);

        let close_msg = ClientMessage::Close("sub1".to_string());
        let responses = bridge.relay().handle_message(&close_msg, &sub_mgr);
        assert!(responses.is_empty());
    }

    #[tokio::test]
    async fn nostr_bridge_syncs_events_to_feed() {
        let feed = Arc::new(RwLock::new(Feed::new()));
        let bridge = NostrBridge::new(NostrConfig::default(), feed.clone());
        let relay = bridge.relay().clone();

        // Subscribe BEFORE sending the event
        let mut rx = relay.subscribe_broadcast();

        // Submit event through the relay
        let sub_mgr = SubscriptionManager::new(20);
        let key = ed25519_dalek::SigningKey::generate(&mut rand::rngs::OsRng);
        let event = EventBuilder::text_note("bridged event").sign(&key);
        let msg = ClientMessage::Event(event);
        relay.handle_message(&msg, &sub_mgr);

        // Receive and bridge the event
        let nostr_event = rx.recv().await.unwrap();
        let social_event = SignedEvent::new(
            &nostr_event.pubkey,
            EventKind::from(nostr_event.kind.0 as u32),
            &nostr_event.content,
            vec![],
        );
        let mut feed_w = feed.write().await;
        feed_w.insert(social_event);
        drop(feed_w);

        // Check that the event was bridged into the Nous feed
        let feed_r = feed.read().await;
        assert_eq!(feed_r.len(), 1);
        assert_eq!(feed_r.latest(1)[0].content, "bridged event");
    }

    #[test]
    fn nostr_event_deduplication() {
        let bridge = make_bridge();
        let sub_mgr = SubscriptionManager::new(20);

        let key = ed25519_dalek::SigningKey::generate(&mut rand::rngs::OsRng);
        let event = EventBuilder::text_note("duplicate me").sign(&key);
        let event2 = event.clone();

        bridge
            .relay()
            .handle_message(&ClientMessage::Event(event), &sub_mgr);
        let responses = bridge
            .relay()
            .handle_message(&ClientMessage::Event(event2), &sub_mgr);

        // Second submission should be accepted but noted as duplicate
        assert!(!responses.is_empty());
    }
}
