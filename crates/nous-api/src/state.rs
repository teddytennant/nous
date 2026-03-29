use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

use nous_files::FileStore;
use nous_governance::{CommittedVote, Dao, Proposal, VoteTally};
use nous_identity::{Credential, Identity, Reputation};
use nous_marketplace::{Listing, Review};
use nous_messaging::{Channel, Message};
use nous_social::{Feed, FollowGraph};

use crate::config::ApiConfig;

pub struct AppState {
    pub config: ApiConfig,
    pub feed: RwLock<Feed>,
    pub follow_graph: RwLock<FollowGraph>,
    pub file_store: RwLock<FileStore>,
    pub daos: RwLock<HashMap<String, Dao>>,
    pub proposals: RwLock<HashMap<String, Proposal>>,
    pub tallies: RwLock<HashMap<String, VoteTally>>,
    pub private_votes: RwLock<HashMap<String, Vec<CommittedVote>>>,
    pub listings: RwLock<HashMap<String, Listing>>,
    pub reviews: RwLock<HashMap<String, Review>>,
    // Messaging
    pub channels: RwLock<HashMap<String, Channel>>,
    pub messages: RwLock<HashMap<String, Vec<Message>>>,
    // Identity
    pub identities: RwLock<HashMap<String, Identity>>,
    pub credentials: RwLock<HashMap<String, Vec<Credential>>>,
    pub reputations: RwLock<HashMap<String, Reputation>>,
}

impl AppState {
    pub fn new(config: ApiConfig) -> Arc<Self> {
        Arc::new(Self {
            config,
            feed: RwLock::new(Feed::new()),
            follow_graph: RwLock::new(FollowGraph::new()),
            file_store: RwLock::new(FileStore::new()),
            daos: RwLock::new(HashMap::new()),
            proposals: RwLock::new(HashMap::new()),
            tallies: RwLock::new(HashMap::new()),
            private_votes: RwLock::new(HashMap::new()),
            listings: RwLock::new(HashMap::new()),
            reviews: RwLock::new(HashMap::new()),
            channels: RwLock::new(HashMap::new()),
            messages: RwLock::new(HashMap::new()),
            identities: RwLock::new(HashMap::new()),
            credentials: RwLock::new(HashMap::new()),
            reputations: RwLock::new(HashMap::new()),
        })
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
