//! Network abstraction. v0 is a trait + topic enum; the in-process simulator
//! and a future libp2p binding both implement it.

use serde::{Deserialize, Serialize};

use crate::state::WorkerId;

/// Gossip topic on which messages are published.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Topic {
    /// New jobs added to the pool.
    Jobs,
    /// Receipt commitments (commit phase).
    ReceiptCommits,
    /// Revealed receipts.
    ReceiptReveals,
    /// Proposed blocks awaiting finality votes.
    Blocks,
    /// Stake-weighted finality votes.
    Votes,
    /// Equivocation proofs.
    Slashes,
}

/// One message observed on the wire.
#[derive(Debug, Clone)]
pub struct NetworkEvent {
    pub topic: Topic,
    pub from: Option<WorkerId>,
    pub payload: Vec<u8>,
}

/// The network abstraction. Implementations are responsible for delivering
/// every message to every subscribed peer (gossip semantics) — consensus
/// makes no assumptions beyond eventual delivery.
pub trait Network: Send + Sync {
    fn publish(&self, topic: Topic, from: WorkerId, payload: Vec<u8>);
    fn drain(&self) -> Vec<NetworkEvent>;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn topic_is_serde_round_trip() {
        for t in [
            Topic::Jobs,
            Topic::ReceiptCommits,
            Topic::ReceiptReveals,
            Topic::Blocks,
            Topic::Votes,
            Topic::Slashes,
        ] {
            let json = serde_json::to_string(&t).unwrap();
            let back: Topic = serde_json::from_str(&json).unwrap();
            assert_eq!(t, back);
        }
    }

    #[test]
    fn topics_distinct() {
        let all = [
            Topic::Jobs,
            Topic::ReceiptCommits,
            Topic::ReceiptReveals,
            Topic::Blocks,
            Topic::Votes,
            Topic::Slashes,
        ];
        for (i, a) in all.iter().enumerate() {
            for b in &all[i + 1..] {
                assert_ne!(a, b);
            }
        }
    }
}
