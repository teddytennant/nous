use serde::{Deserialize, Serialize};

pub const PROTOCOL_VERSION: &str = "/nous/1.0.0";
pub const PROTOCOL_ID: &str = "/nous/wire/1.0.0";

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum NousProtocol {
    Ping { nonce: u64 },
    Pong { nonce: u64 },
    Message(Vec<u8>),
    Sync { from_seq: u64 },
    SyncResponse { events: Vec<Vec<u8>> },
    GetPeers,
    PeerList { peers: Vec<String> },
    GetRecord { key: String },
    RecordResponse { key: String, value: Option<Vec<u8>> },
}

impl NousProtocol {
    pub fn encode(&self) -> Result<Vec<u8>, serde_json::Error> {
        serde_json::to_vec(self)
    }

    pub fn decode(data: &[u8]) -> Result<Self, serde_json::Error> {
        serde_json::from_slice(data)
    }

    pub fn is_request(&self) -> bool {
        matches!(
            self,
            Self::Ping { .. }
                | Self::Sync { .. }
                | Self::GetPeers
                | Self::GetRecord { .. }
                | Self::Message(_)
        )
    }

    pub fn is_response(&self) -> bool {
        !self.is_request()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn protocol_serializes() {
        let msg = NousProtocol::Message(b"hello".to_vec());
        let bytes = msg.encode().unwrap();
        let decoded = NousProtocol::decode(&bytes).unwrap();
        assert_eq!(msg, decoded);
    }

    #[test]
    fn ping_pong_roundtrip() {
        let ping = NousProtocol::Ping { nonce: 42 };
        let bytes = ping.encode().unwrap();
        let decoded = NousProtocol::decode(&bytes).unwrap();
        assert_eq!(decoded, NousProtocol::Ping { nonce: 42 });
    }

    #[test]
    fn sync_request_response() {
        let req = NousProtocol::Sync { from_seq: 100 };
        assert!(req.is_request());

        let resp = NousProtocol::SyncResponse {
            events: vec![b"event1".to_vec(), b"event2".to_vec()],
        };
        assert!(resp.is_response());
    }

    #[test]
    fn get_peers_roundtrip() {
        let req = NousProtocol::GetPeers;
        assert!(req.is_request());

        let resp = NousProtocol::PeerList {
            peers: vec!["peer1".into(), "peer2".into()],
        };
        let bytes = resp.encode().unwrap();
        let decoded = NousProtocol::decode(&bytes).unwrap();
        assert_eq!(decoded, resp);
    }

    #[test]
    fn get_record_roundtrip() {
        let req = NousProtocol::GetRecord {
            key: "user:alice".into(),
        };
        assert!(req.is_request());

        let resp = NousProtocol::RecordResponse {
            key: "user:alice".into(),
            value: Some(b"profile data".to_vec()),
        };
        assert!(resp.is_response());

        let bytes = resp.encode().unwrap();
        let decoded = NousProtocol::decode(&bytes).unwrap();
        assert_eq!(decoded, resp);
    }

    #[test]
    fn empty_sync_response() {
        let resp = NousProtocol::SyncResponse { events: vec![] };
        let bytes = resp.encode().unwrap();
        let decoded = NousProtocol::decode(&bytes).unwrap();
        assert_eq!(decoded, resp);
    }

    #[test]
    fn protocol_version_is_namespaced() {
        assert!(PROTOCOL_VERSION.starts_with("/nous/"));
        assert!(PROTOCOL_ID.starts_with("/nous/"));
    }
}
