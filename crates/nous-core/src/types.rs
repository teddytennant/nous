use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct NodeId(pub String);

impl NodeId {
    pub fn new(id: impl Into<String>) -> Self {
        Self(id.into())
    }
}

impl std::fmt::Display for NodeId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct EventId(pub Uuid);

impl EventId {
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }
}

impl Default for EventId {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Display for EventId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Timestamp(pub DateTime<Utc>);

impl Timestamp {
    pub fn now() -> Self {
        Self(Utc::now())
    }

    pub fn is_expired(&self) -> bool {
        self.0 < Utc::now()
    }
}

impl Default for Timestamp {
    fn default() -> Self {
        Self::now()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Envelope<T: Serialize> {
    pub id: EventId,
    pub timestamp: Timestamp,
    pub sender: NodeId,
    pub payload: T,
    pub signature: Option<Vec<u8>>,
}

impl<T: Serialize> Envelope<T> {
    pub fn new(sender: NodeId, payload: T) -> Self {
        Self {
            id: EventId::new(),
            timestamp: Timestamp::now(),
            sender,
            payload,
            signature: None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn node_id_display() {
        let id = NodeId::new("test-node");
        assert_eq!(id.to_string(), "test-node");
    }

    #[test]
    fn event_id_unique() {
        let a = EventId::new();
        let b = EventId::new();
        assert_ne!(a, b);
    }

    #[test]
    fn timestamp_not_expired_in_future() {
        let ts = Timestamp(Utc::now() + chrono::Duration::hours(1));
        assert!(!ts.is_expired());
    }

    #[test]
    fn timestamp_expired_in_past() {
        let ts = Timestamp(Utc::now() - chrono::Duration::hours(1));
        assert!(ts.is_expired());
    }

    #[test]
    fn envelope_creation() {
        let env = Envelope::new(NodeId::new("sender"), "hello");
        assert_eq!(env.sender.0, "sender");
        assert!(env.signature.is_none());
    }

    #[test]
    fn node_id_serde_roundtrip() {
        let id = NodeId::new("test");
        let json = serde_json::to_string(&id).unwrap();
        let deserialized: NodeId = serde_json::from_str(&json).unwrap();
        assert_eq!(id, deserialized);
    }
}
