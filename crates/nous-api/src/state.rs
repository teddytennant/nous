use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{RwLock, broadcast};

use nous_ai::{Agent, Conversation};
use nous_files::FileStore;
use nous_governance::{
    CommittedVote, Dao, DelegationRegistry, ExecutionEngine, Proposal, VoteTally,
};
use nous_identity::{Credential, Identity, Reputation};
use nous_marketplace::{Listing, Review};
use nous_messaging::{Channel, Message};
use nous_payments::{Escrow, Invoice, Transaction, Wallet};
use nous_social::{Feed, FollowGraph};

use crate::config::ApiConfig;

/// Real-time event broadcast across all connected clients.
#[derive(Debug, Clone, serde::Serialize)]
#[serde(tag = "type", content = "data")]
pub enum RealtimeEvent {
    /// New post in the social feed.
    NewPost {
        id: String,
        author: String,
        content: String,
    },
    /// New message in a channel.
    NewMessage {
        channel_id: String,
        sender: String,
        content: String,
    },
    /// Vote cast on a proposal.
    VoteCast { proposal_id: String, voter: String },
    /// New DAO created.
    DaoCreated { id: String, name: String },
    /// Proposal submitted.
    ProposalCreated {
        id: String,
        title: String,
        dao_id: String,
    },
    /// Payment or transfer completed.
    Transfer {
        from: String,
        to: String,
        amount: String,
        token: String,
    },
    /// Listing created or updated in marketplace.
    ListingUpdate { id: String, title: String },
}

pub struct AppState {
    pub config: ApiConfig,
    pub feed: RwLock<Feed>,
    pub follow_graph: RwLock<FollowGraph>,
    pub file_store: RwLock<FileStore>,
    pub daos: RwLock<HashMap<String, Dao>>,
    pub proposals: RwLock<HashMap<String, Proposal>>,
    pub tallies: RwLock<HashMap<String, VoteTally>>,
    pub private_votes: RwLock<HashMap<String, Vec<CommittedVote>>>,
    pub delegations: RwLock<DelegationRegistry>,
    pub execution_engine: RwLock<ExecutionEngine>,
    pub listings: RwLock<HashMap<String, Listing>>,
    pub reviews: RwLock<HashMap<String, Review>>,
    // Messaging
    pub channels: RwLock<HashMap<String, Channel>>,
    pub messages: RwLock<HashMap<String, Vec<Message>>>,
    // Identity
    pub identities: RwLock<HashMap<String, Identity>>,
    pub credentials: RwLock<HashMap<String, Vec<Credential>>>,
    pub reputations: RwLock<HashMap<String, Reputation>>,
    // Payments
    pub wallets: RwLock<HashMap<String, Wallet>>,
    pub transactions: RwLock<Vec<Transaction>>,
    pub escrows: RwLock<HashMap<String, Escrow>>,
    pub invoices: RwLock<HashMap<String, Invoice>>,
    // AI
    pub agents: RwLock<HashMap<String, Agent>>,
    pub conversations: RwLock<HashMap<String, Conversation>>,
    // Real-time event bus
    pub events: broadcast::Sender<RealtimeEvent>,
}

impl AppState {
    pub fn new(config: ApiConfig) -> Arc<Self> {
        let (events_tx, _) = broadcast::channel(256);
        Arc::new(Self {
            config,
            feed: RwLock::new(Feed::new()),
            follow_graph: RwLock::new(FollowGraph::new()),
            file_store: RwLock::new(FileStore::new()),
            daos: RwLock::new(HashMap::new()),
            proposals: RwLock::new(HashMap::new()),
            tallies: RwLock::new(HashMap::new()),
            private_votes: RwLock::new(HashMap::new()),
            delegations: RwLock::new(DelegationRegistry::new()),
            execution_engine: RwLock::new(ExecutionEngine::new(86400, 259200)), // 1 day timelock, 3 day grace
            listings: RwLock::new(HashMap::new()),
            reviews: RwLock::new(HashMap::new()),
            channels: RwLock::new(HashMap::new()),
            messages: RwLock::new(HashMap::new()),
            identities: RwLock::new(HashMap::new()),
            credentials: RwLock::new(HashMap::new()),
            reputations: RwLock::new(HashMap::new()),
            wallets: RwLock::new(HashMap::new()),
            transactions: RwLock::new(Vec::new()),
            escrows: RwLock::new(HashMap::new()),
            invoices: RwLock::new(HashMap::new()),
            agents: RwLock::new(HashMap::new()),
            conversations: RwLock::new(HashMap::new()),
            events: events_tx,
        })
    }

    /// Broadcast a real-time event to all connected clients.
    pub fn emit(&self, event: RealtimeEvent) {
        let _ = self.events.send(event);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn creates_with_defaults() {
        let state = AppState::new(ApiConfig::default());
        assert_eq!(state.config.port, 8080);
    }
}
