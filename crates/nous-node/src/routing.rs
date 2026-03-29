//! P2P message routing — dispatches gossipsub messages to the appropriate
//! subsystem based on topic.

use nous_net::topics::NousTopic;
use serde::{Deserialize, Serialize};
use tokio::sync::mpsc;
use tracing::{debug, warn};

// ── Wire envelope ─────────────────────────────────────────────────────

/// Envelope for messages published on gossipsub topics.
/// The inner `payload` is a JSON-encoded subsystem-specific type.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct P2pEnvelope {
    /// Which subsystem this message targets.
    pub topic: NousTopic,
    /// The sender's DID (self-declared, verify via signature in payload).
    pub sender_did: String,
    /// Monotonic millisecond timestamp.
    pub timestamp_ms: u64,
    /// JSON-encoded subsystem payload.
    pub payload: serde_json::Value,
}

impl P2pEnvelope {
    pub fn new(topic: NousTopic, sender_did: &str, payload: serde_json::Value) -> Self {
        Self {
            topic,
            sender_did: sender_did.to_string(),
            timestamp_ms: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_millis() as u64,
            payload,
        }
    }

    pub fn encode(&self) -> Result<Vec<u8>, serde_json::Error> {
        serde_json::to_vec(self)
    }

    pub fn decode(data: &[u8]) -> Result<Self, serde_json::Error> {
        serde_json::from_slice(data)
    }
}

// ── Routed event types ────────────────────────────────────────────────

/// A social event received from the network.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SocialEvent {
    pub event: nous_social::SignedEvent,
}

/// A messaging event received from the network.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MessagingEvent {
    pub message: nous_messaging::Message,
}

/// A governance event received from the network.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum GovernanceEvent {
    NewProposal(nous_governance::Proposal),
    Vote(nous_governance::Ballot),
}

/// A marketplace event received from the network.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MarketplaceEvent {
    NewListing(nous_marketplace::Listing),
    OrderUpdate(nous_marketplace::Order),
}

// ── Routed output ─────────────────────────────────────────────────────

/// A successfully routed message, ready for the subsystem to consume.
#[derive(Debug, Clone)]
pub enum RoutedMessage {
    Social(SocialEvent),
    Messaging(MessagingEvent),
    Governance(GovernanceEvent),
    Marketplace(MarketplaceEvent),
    /// Topics we acknowledge but don't actively route yet (Payments, Identity,
    /// Sync, Custom). Logged and forwarded as raw bytes.
    Unhandled {
        topic: NousTopic,
        sender_did: String,
        payload: serde_json::Value,
    },
}

// ── Router ────────────────────────────────────────────────────────────

/// Routes raw gossipsub bytes into typed subsystem messages.
pub struct MessageRouter {
    /// Channel for routed messages to be consumed by the node orchestrator
    /// or subsystem handlers.
    tx: mpsc::Sender<RoutedMessage>,
}

impl MessageRouter {
    /// Create a new router and its receiving end.
    pub fn new(buffer: usize) -> (Self, mpsc::Receiver<RoutedMessage>) {
        let (tx, rx) = mpsc::channel(buffer);
        (Self { tx }, rx)
    }

    /// Route a raw gossipsub message (topic + bytes) into the correct subsystem
    /// channel. Returns `Ok(())` if the message was dispatched (or intentionally
    /// skipped), or `Err` only if the internal channel is closed.
    pub async fn route(
        &self,
        topic: &NousTopic,
        data: &[u8],
        _local_did: &str,
    ) -> Result<(), mpsc::error::SendError<RoutedMessage>> {
        // Step 1: decode the envelope.
        let envelope = match P2pEnvelope::decode(data) {
            Ok(env) => env,
            Err(e) => {
                warn!(
                    %topic,
                    error = %e,
                    "failed to decode P2pEnvelope, skipping message"
                );
                return Ok(());
            }
        };

        debug!(
            %topic,
            sender = %envelope.sender_did,
            "routing message"
        );

        // Step 2: deserialize the inner payload based on topic.
        let routed = match topic {
            NousTopic::Social => {
                match serde_json::from_value::<SocialEvent>(envelope.payload.clone()) {
                    Ok(social) => {
                        debug!(event_id = %social.event.id, "routed social event");
                        RoutedMessage::Social(social)
                    }
                    Err(e) => {
                        warn!(error = %e, "failed to deserialize social event payload");
                        return Ok(());
                    }
                }
            }

            NousTopic::Messages => {
                match serde_json::from_value::<MessagingEvent>(envelope.payload.clone()) {
                    Ok(msg) => {
                        debug!(msg_id = %msg.message.id, "routed messaging event");
                        RoutedMessage::Messaging(msg)
                    }
                    Err(e) => {
                        warn!(error = %e, "failed to deserialize messaging event payload");
                        return Ok(());
                    }
                }
            }

            NousTopic::Governance => {
                match serde_json::from_value::<GovernanceEvent>(envelope.payload.clone()) {
                    Ok(gov) => {
                        debug!("routed governance event");
                        RoutedMessage::Governance(gov)
                    }
                    Err(e) => {
                        warn!(error = %e, "failed to deserialize governance event payload");
                        return Ok(());
                    }
                }
            }

            NousTopic::Marketplace => {
                match serde_json::from_value::<MarketplaceEvent>(envelope.payload.clone()) {
                    Ok(mkt) => {
                        debug!("routed marketplace event");
                        RoutedMessage::Marketplace(mkt)
                    }
                    Err(e) => {
                        warn!(error = %e, "failed to deserialize marketplace event payload");
                        return Ok(());
                    }
                }
            }

            // Payments, Identity, Sync, Custom — acknowledged but not fully
            // routed yet.
            other => {
                debug!(%other, "topic not actively routed, forwarding as unhandled");
                RoutedMessage::Unhandled {
                    topic: other.clone(),
                    sender_did: envelope.sender_did,
                    payload: envelope.payload,
                }
            }
        };

        self.tx.send(routed).await
    }
}

// ── Outbound publishing helpers ───────────────────────────────────────

/// Encode a social event into bytes ready for gossipsub publishing.
pub fn encode_social(
    sender_did: &str,
    event: &nous_social::SignedEvent,
) -> Result<Vec<u8>, serde_json::Error> {
    let social = SocialEvent {
        event: event.clone(),
    };
    let payload = serde_json::to_value(&social)?;
    P2pEnvelope::new(NousTopic::Social, sender_did, payload).encode()
}

/// Encode a messaging event into bytes ready for gossipsub publishing.
pub fn encode_messaging(
    sender_did: &str,
    message: &nous_messaging::Message,
) -> Result<Vec<u8>, serde_json::Error> {
    let msg = MessagingEvent {
        message: message.clone(),
    };
    let payload = serde_json::to_value(&msg)?;
    P2pEnvelope::new(NousTopic::Messages, sender_did, payload).encode()
}

/// Encode a governance event into bytes ready for gossipsub publishing.
pub fn encode_governance(
    sender_did: &str,
    event: &GovernanceEvent,
) -> Result<Vec<u8>, serde_json::Error> {
    let payload = serde_json::to_value(event)?;
    P2pEnvelope::new(NousTopic::Governance, sender_did, payload).encode()
}

/// Encode a marketplace event into bytes ready for gossipsub publishing.
pub fn encode_marketplace(
    sender_did: &str,
    event: &MarketplaceEvent,
) -> Result<Vec<u8>, serde_json::Error> {
    let payload = serde_json::to_value(event)?;
    P2pEnvelope::new(NousTopic::Marketplace, sender_did, payload).encode()
}

// ── Tests ─────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use nous_social::{EventKind, SignedEvent};

    fn make_social_envelope(sender: &str, content: &str) -> Vec<u8> {
        let event = SignedEvent::new(sender, EventKind::TextNote, content, vec![]);
        encode_social(sender, &event).unwrap()
    }

    fn make_messaging_envelope(sender: &str) -> Vec<u8> {
        let identity = nous_identity::Identity::generate();
        let msg = nous_messaging::message::MessageBuilder::text("channel-1", "hello p2p")
            .sign(&identity)
            .unwrap();
        encode_messaging(sender, &msg).unwrap()
    }

    fn make_governance_proposal_envelope(sender: &str) -> Vec<u8> {
        let identity = nous_identity::Identity::generate();
        let proposal = nous_governance::ProposalBuilder::new("dao-1", "Test prop", "A test")
            .submit(&identity)
            .unwrap();
        let event = GovernanceEvent::NewProposal(proposal);
        encode_governance(sender, &event).unwrap()
    }

    fn make_governance_vote_envelope(sender: &str) -> Vec<u8> {
        let identity = nous_identity::Identity::generate();
        let ballot = nous_governance::Ballot::new(
            "prop:123",
            &identity,
            nous_governance::VoteChoice::For,
            4,
        )
        .unwrap();
        let event = GovernanceEvent::Vote(ballot);
        encode_governance(sender, &event).unwrap()
    }

    fn make_marketplace_listing_envelope(sender: &str) -> Vec<u8> {
        let listing = nous_marketplace::Listing::new(
            sender,
            "Widget",
            "A great widget",
            nous_marketplace::ListingCategory::Physical,
            "ETH",
            100,
        )
        .unwrap();
        let event = MarketplaceEvent::NewListing(listing);
        encode_marketplace(sender, &event).unwrap()
    }

    // ── Envelope tests ────────────────────────────────────────────────

    #[test]
    fn envelope_roundtrip() {
        let payload = serde_json::json!({"hello": "world"});
        let env = P2pEnvelope::new(NousTopic::Social, "did:key:z123", payload.clone());
        let bytes = env.encode().unwrap();
        let decoded = P2pEnvelope::decode(&bytes).unwrap();
        assert_eq!(decoded.topic, NousTopic::Social);
        assert_eq!(decoded.sender_did, "did:key:z123");
        assert_eq!(decoded.payload, payload);
        assert!(decoded.timestamp_ms > 0);
    }

    #[test]
    fn envelope_decode_garbage_fails() {
        let result = P2pEnvelope::decode(b"not json at all");
        assert!(result.is_err());
    }

    // ── Encoding helpers ──────────────────────────────────────────────

    #[test]
    fn encode_social_roundtrip() {
        let data = make_social_envelope("did:key:alice", "hello network");
        let env = P2pEnvelope::decode(&data).unwrap();
        assert_eq!(env.topic, NousTopic::Social);
        let social: SocialEvent = serde_json::from_value(env.payload).unwrap();
        assert_eq!(social.event.content, "hello network");
    }

    #[test]
    fn encode_messaging_roundtrip() {
        let data = make_messaging_envelope("did:key:bob");
        let env = P2pEnvelope::decode(&data).unwrap();
        assert_eq!(env.topic, NousTopic::Messages);
        let msg: MessagingEvent = serde_json::from_value(env.payload).unwrap();
        assert_eq!(msg.message.channel_id, "channel-1");
    }

    #[test]
    fn encode_governance_proposal_roundtrip() {
        let data = make_governance_proposal_envelope("did:key:carol");
        let env = P2pEnvelope::decode(&data).unwrap();
        assert_eq!(env.topic, NousTopic::Governance);
        let gov: GovernanceEvent = serde_json::from_value(env.payload).unwrap();
        match gov {
            GovernanceEvent::NewProposal(p) => assert_eq!(p.title, "Test prop"),
            _ => panic!("expected NewProposal"),
        }
    }

    #[test]
    fn encode_governance_vote_roundtrip() {
        let data = make_governance_vote_envelope("did:key:dave");
        let env = P2pEnvelope::decode(&data).unwrap();
        assert_eq!(env.topic, NousTopic::Governance);
        let gov: GovernanceEvent = serde_json::from_value(env.payload).unwrap();
        match gov {
            GovernanceEvent::Vote(b) => assert_eq!(b.choice, nous_governance::VoteChoice::For),
            _ => panic!("expected Vote"),
        }
    }

    #[test]
    fn encode_marketplace_listing_roundtrip() {
        let data = make_marketplace_listing_envelope("did:key:eve");
        let env = P2pEnvelope::decode(&data).unwrap();
        assert_eq!(env.topic, NousTopic::Marketplace);
        let mkt: MarketplaceEvent = serde_json::from_value(env.payload).unwrap();
        match mkt {
            MarketplaceEvent::NewListing(l) => assert_eq!(l.title, "Widget"),
            _ => panic!("expected NewListing"),
        }
    }

    // ── Router tests ──────────────────────────────────────────────────

    #[tokio::test]
    async fn route_social_message() {
        let (router, mut rx) = MessageRouter::new(16);
        let data = make_social_envelope("did:key:alice", "routed post");

        router
            .route(&NousTopic::Social, &data, "did:key:local")
            .await
            .unwrap();

        let msg = rx.try_recv().unwrap();
        match msg {
            RoutedMessage::Social(s) => assert_eq!(s.event.content, "routed post"),
            other => panic!("expected Social, got {other:?}"),
        }
    }

    #[tokio::test]
    async fn route_messaging_message() {
        let (router, mut rx) = MessageRouter::new(16);
        let data = make_messaging_envelope("did:key:bob");

        router
            .route(&NousTopic::Messages, &data, "did:key:local")
            .await
            .unwrap();

        let msg = rx.try_recv().unwrap();
        match msg {
            RoutedMessage::Messaging(m) => assert_eq!(m.message.channel_id, "channel-1"),
            other => panic!("expected Messaging, got {other:?}"),
        }
    }

    #[tokio::test]
    async fn route_governance_proposal() {
        let (router, mut rx) = MessageRouter::new(16);
        let data = make_governance_proposal_envelope("did:key:carol");

        router
            .route(&NousTopic::Governance, &data, "did:key:local")
            .await
            .unwrap();

        let msg = rx.try_recv().unwrap();
        match msg {
            RoutedMessage::Governance(GovernanceEvent::NewProposal(p)) => {
                assert_eq!(p.title, "Test prop");
            }
            other => panic!("expected Governance NewProposal, got {other:?}"),
        }
    }

    #[tokio::test]
    async fn route_governance_vote() {
        let (router, mut rx) = MessageRouter::new(16);
        let data = make_governance_vote_envelope("did:key:dave");

        router
            .route(&NousTopic::Governance, &data, "did:key:local")
            .await
            .unwrap();

        let msg = rx.try_recv().unwrap();
        match msg {
            RoutedMessage::Governance(GovernanceEvent::Vote(b)) => {
                assert_eq!(b.choice, nous_governance::VoteChoice::For);
            }
            other => panic!("expected Governance Vote, got {other:?}"),
        }
    }

    #[tokio::test]
    async fn route_marketplace_listing() {
        let (router, mut rx) = MessageRouter::new(16);
        let data = make_marketplace_listing_envelope("did:key:eve");

        router
            .route(&NousTopic::Marketplace, &data, "did:key:local")
            .await
            .unwrap();

        let msg = rx.try_recv().unwrap();
        match msg {
            RoutedMessage::Marketplace(MarketplaceEvent::NewListing(l)) => {
                assert_eq!(l.title, "Widget");
            }
            other => panic!("expected Marketplace NewListing, got {other:?}"),
        }
    }

    #[tokio::test]
    async fn route_unhandled_topic() {
        let (router, mut rx) = MessageRouter::new(16);

        // Build an envelope for the Payments topic (not actively routed).
        let payload = serde_json::json!({"tx": "abc123"});
        let env = P2pEnvelope::new(NousTopic::Payments, "did:key:frank", payload.clone());
        let data = env.encode().unwrap();

        router
            .route(&NousTopic::Payments, &data, "did:key:local")
            .await
            .unwrap();

        let msg = rx.try_recv().unwrap();
        match msg {
            RoutedMessage::Unhandled {
                topic,
                sender_did,
                payload: p,
            } => {
                assert_eq!(topic, NousTopic::Payments);
                assert_eq!(sender_did, "did:key:frank");
                assert_eq!(p, payload);
            }
            other => panic!("expected Unhandled, got {other:?}"),
        }
    }

    #[tokio::test]
    async fn route_garbage_data_skips_gracefully() {
        let (router, mut rx) = MessageRouter::new(16);
        let garbage = b"this is not valid json";

        // Should not panic and should not send anything.
        router
            .route(&NousTopic::Social, garbage, "did:key:local")
            .await
            .unwrap();

        // Channel should be empty.
        assert!(rx.try_recv().is_err());
    }

    #[tokio::test]
    async fn route_valid_envelope_bad_payload_skips() {
        let (router, mut rx) = MessageRouter::new(16);

        // Valid envelope but the payload doesn't match the expected social schema.
        let env = P2pEnvelope::new(
            NousTopic::Social,
            "did:key:z",
            serde_json::json!({"wrong": "schema"}),
        );
        let data = env.encode().unwrap();

        router
            .route(&NousTopic::Social, &data, "did:key:local")
            .await
            .unwrap();

        // Should be skipped — nothing in channel.
        assert!(rx.try_recv().is_err());
    }

    #[tokio::test]
    async fn route_multiple_messages_in_sequence() {
        let (router, mut rx) = MessageRouter::new(16);

        let social_data = make_social_envelope("did:key:alice", "post one");
        let msg_data = make_messaging_envelope("did:key:bob");

        router
            .route(&NousTopic::Social, &social_data, "did:key:local")
            .await
            .unwrap();
        router
            .route(&NousTopic::Messages, &msg_data, "did:key:local")
            .await
            .unwrap();

        let first = rx.try_recv().unwrap();
        assert!(matches!(first, RoutedMessage::Social(_)));

        let second = rx.try_recv().unwrap();
        assert!(matches!(second, RoutedMessage::Messaging(_)));
    }
}
