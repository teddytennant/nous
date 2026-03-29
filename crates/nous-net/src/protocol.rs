use serde::{Deserialize, Serialize};

pub const PROTOCOL_VERSION: &str = "/nous/1.0.0";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum NousProtocol {
    Ping,
    Pong,
    Message(Vec<u8>),
    Sync { from_seq: u64 },
    SyncResponse { events: Vec<Vec<u8>> },
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn protocol_serializes() {
        let msg = NousProtocol::Message(b"hello".to_vec());
        let bytes = serde_json::to_vec(&msg).unwrap();
        let _: NousProtocol = serde_json::from_slice(&bytes).unwrap();
    }
}
