use std::sync::Arc;
use tokio::sync::RwLock;

use nous_social::{Feed, FollowGraph};

use crate::config::ApiConfig;

pub struct AppState {
    pub config: ApiConfig,
    pub feed: RwLock<Feed>,
    pub follow_graph: RwLock<FollowGraph>,
}

impl AppState {
    pub fn new(config: ApiConfig) -> Arc<Self> {
        Arc::new(Self {
            config,
            feed: RwLock::new(Feed::new()),
            follow_graph: RwLock::new(FollowGraph::new()),
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
