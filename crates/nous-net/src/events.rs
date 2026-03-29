use libp2p::PeerId;
use serde::{Deserialize, Serialize};

use crate::topics::NousTopic;

#[derive(Debug, Clone)]
pub enum NodeEvent {
    PeerConnected(PeerId),
    PeerDisconnected(PeerId),
    MessageReceived {
        source: Option<PeerId>,
        topic: NousTopic,
        data: Vec<u8>,
    },
    PeerDiscovered(PeerId),
    Subscribed {
        peer: PeerId,
        topic: NousTopic,
    },
    Unsubscribed {
        peer: PeerId,
        topic: NousTopic,
    },
    ListeningOn(libp2p::Multiaddr),
    DhtRecordFound {
        key: String,
        value: Vec<u8>,
    },
    DhtRecordStored {
        key: String,
    },
    Error(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WireMessage {
    pub topic: NousTopic,
    pub payload: Vec<u8>,
    pub sender_did: String,
    pub timestamp_ms: u64,
    pub signature: Vec<u8>,
}

impl WireMessage {
    pub fn new(topic: NousTopic, payload: Vec<u8>, sender_did: String) -> Self {
        Self {
            topic,
            payload,
            sender_did,
            timestamp_ms: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_millis() as u64,
            signature: Vec::new(),
        }
    }

    pub fn encode(&self) -> Result<Vec<u8>, serde_json::Error> {
        serde_json::to_vec(self)
    }

    pub fn decode(data: &[u8]) -> Result<Self, serde_json::Error> {
        serde_json::from_slice(data)
    }

    pub fn signable_bytes(&self) -> Vec<u8> {
        let mut buf = Vec::new();
        buf.extend_from_slice(self.topic.as_str().as_bytes());
        buf.extend_from_slice(&self.payload);
        buf.extend_from_slice(self.sender_did.as_bytes());
        buf.extend_from_slice(&self.timestamp_ms.to_be_bytes());
        buf
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wire_message_encode_decode() {
        let msg = WireMessage::new(
            NousTopic::Messages,
            b"hello world".to_vec(),
            "did:key:z123".to_string(),
        );

        let encoded = msg.encode().unwrap();
        let decoded = WireMessage::decode(&encoded).unwrap();

        assert_eq!(decoded.topic, msg.topic);
        assert_eq!(decoded.payload, msg.payload);
        assert_eq!(decoded.sender_did, msg.sender_did);
    }

    #[test]
    fn wire_message_signable_bytes_deterministic() {
        let msg = WireMessage {
            topic: NousTopic::Social,
            payload: b"test".to_vec(),
            sender_did: "did:key:z123".to_string(),
            timestamp_ms: 1000,
            signature: Vec::new(),
        };

        let bytes1 = msg.signable_bytes();
        let bytes2 = msg.signable_bytes();
        assert_eq!(bytes1, bytes2);
    }

    #[test]
    fn wire_message_signable_bytes_differ_on_content() {
        let msg1 = WireMessage {
            topic: NousTopic::Social,
            payload: b"hello".to_vec(),
            sender_did: "did:key:z123".to_string(),
            timestamp_ms: 1000,
            signature: Vec::new(),
        };

        let msg2 = WireMessage {
            topic: NousTopic::Social,
            payload: b"world".to_vec(),
            sender_did: "did:key:z123".to_string(),
            timestamp_ms: 1000,
            signature: Vec::new(),
        };

        assert_ne!(msg1.signable_bytes(), msg2.signable_bytes());
    }

    #[test]
    fn wire_message_timestamp_nonzero() {
        let msg = WireMessage::new(NousTopic::Sync, vec![], "did:key:z123".to_string());
        assert!(msg.timestamp_ms > 0);
    }

    #[test]
    fn wire_message_starts_unsigned() {
        let msg = WireMessage::new(NousTopic::Sync, vec![], "did:key:z123".to_string());
        assert!(msg.signature.is_empty());
    }
}
