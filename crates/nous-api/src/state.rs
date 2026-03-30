use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{Mutex, RwLock, broadcast};

use nous_ai::{Agent, Conversation, InferenceBackend};
use nous_files::FileStore;
use nous_governance::{
    CommittedVote, Dao, DelegationRegistry, ExecutionEngine, Proposal, VoteTally,
};
use nous_identity::{Credential, Identity, Reputation};
use nous_marketplace::{Dispute, Listing, Offer, Order, Review};
use nous_messaging::{Channel, Message};
use nous_payments::{Escrow, Invoice, Transaction, Wallet};
use nous_social::{Feed, FollowGraph};
use nous_storage::Database;

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
    /// Order status changed.
    OrderUpdate { id: String, status: String },
    /// New dispute opened.
    DisputeOpened { id: String, order_id: String },
    /// Offer made on a listing.
    OfferMade { id: String, listing_id: String },
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
    pub orders: RwLock<HashMap<String, Order>>,
    pub disputes: RwLock<HashMap<String, Dispute>>,
    pub offers: RwLock<HashMap<String, Offer>>,
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
    // AI inference backend (None = placeholder mode)
    pub inference_backend: RwLock<Option<Arc<dyn InferenceBackend>>>,
    // Real-time event bus
    pub events: broadcast::Sender<RealtimeEvent>,
    // SQLite persistence — rusqlite::Connection is Send but not Sync,
    // so we use a Mutex rather than RwLock.
    pub db: Mutex<Database>,
}

/// KV namespace prefixes used for SQLite persistence.
pub mod kv_ns {
    pub const DAOS: &str = "api:daos";
    pub const PROPOSALS: &str = "api:proposals";
    pub const TALLIES: &str = "api:tallies";
    pub const PRIVATE_VOTES: &str = "api:private_votes";
    pub const DELEGATIONS: &str = "api:delegations";
    pub const EXECUTION_ENGINE: &str = "api:execution_engine";
    pub const LISTINGS: &str = "api:listings";
    pub const REVIEWS: &str = "api:reviews";
    pub const ORDERS: &str = "api:orders";
    pub const DISPUTES: &str = "api:disputes";
    pub const OFFERS: &str = "api:offers";
    pub const CHANNELS: &str = "api:channels";
    pub const MESSAGES: &str = "api:messages";
    pub const CREDENTIALS: &str = "api:credentials";
    pub const WALLETS: &str = "api:wallets";
    pub const TRANSACTIONS: &str = "api:transactions";
    pub const ESCROWS: &str = "api:escrows";
    pub const INVOICES: &str = "api:invoices";
    pub const AGENTS: &str = "api:agents";
    pub const CONVERSATIONS: &str = "api:conversations";
    pub const IDENTITIES: &str = "api:identities";
    pub const REPUTATIONS: &str = "api:reputations";
    pub const FEED: &str = "api:feed";
    pub const FOLLOW_GRAPH: &str = "api:follow_graph";
    pub const FILE_STORE: &str = "api:file_store";
}

impl AppState {
    /// Create AppState with an in-memory database (for tests and backwards compatibility).
    pub fn new(config: ApiConfig) -> Arc<Self> {
        let db = Database::in_memory().expect("failed to create in-memory database");
        Self::with_db(config, db)
    }

    /// Create AppState backed by a SQLite database, hydrating persisted data on startup.
    pub fn with_db(config: ApiConfig, db: Database) -> Arc<Self> {
        let (events_tx, _) = broadcast::channel(256);

        // Load persisted state from the database KV store.
        let daos = load_kv_map(&db, kv_ns::DAOS);
        let proposals = load_kv_map(&db, kv_ns::PROPOSALS);
        let tallies = load_kv_map(&db, kv_ns::TALLIES);
        let private_votes = load_kv_map(&db, kv_ns::PRIVATE_VOTES);
        let delegations = load_kv_singleton(&db, kv_ns::DELEGATIONS).unwrap_or_default();
        let execution_engine = load_kv_singleton(&db, kv_ns::EXECUTION_ENGINE)
            .unwrap_or_else(|| ExecutionEngine::new(86400, 259200));
        let listings = load_kv_map(&db, kv_ns::LISTINGS);
        let reviews = load_kv_map(&db, kv_ns::REVIEWS);
        let orders = load_kv_map(&db, kv_ns::ORDERS);
        let disputes = load_kv_map(&db, kv_ns::DISPUTES);
        let offers = load_kv_map(&db, kv_ns::OFFERS);
        let channels = load_kv_map(&db, kv_ns::CHANNELS);
        let messages = load_kv_map(&db, kv_ns::MESSAGES);
        let identities: HashMap<String, Identity> = load_kv_map(&db, kv_ns::IDENTITIES);
        let reputations = load_kv_map(&db, kv_ns::REPUTATIONS);
        let credentials = load_kv_map(&db, kv_ns::CREDENTIALS);
        let wallets = load_kv_map(&db, kv_ns::WALLETS);
        let transactions = load_kv_singleton(&db, kv_ns::TRANSACTIONS).unwrap_or_default();
        let escrows = load_kv_map(&db, kv_ns::ESCROWS);
        let invoices = load_kv_map(&db, kv_ns::INVOICES);
        let agents = load_kv_map(&db, kv_ns::AGENTS);
        let conversations = load_kv_map(&db, kv_ns::CONVERSATIONS);
        let follow_graph =
            load_kv_singleton(&db, kv_ns::FOLLOW_GRAPH).unwrap_or_else(FollowGraph::new);
        let file_store = load_kv_singleton(&db, kv_ns::FILE_STORE).unwrap_or_else(FileStore::new);

        Arc::new(Self {
            config,
            feed: RwLock::new(load_kv_singleton(&db, kv_ns::FEED).unwrap_or_else(Feed::new)),
            follow_graph: RwLock::new(follow_graph),
            file_store: RwLock::new(file_store),
            daos: RwLock::new(daos),
            proposals: RwLock::new(proposals),
            tallies: RwLock::new(tallies),
            private_votes: RwLock::new(private_votes),
            delegations: RwLock::new(delegations),
            execution_engine: RwLock::new(execution_engine),
            listings: RwLock::new(listings),
            reviews: RwLock::new(reviews),
            orders: RwLock::new(orders),
            disputes: RwLock::new(disputes),
            offers: RwLock::new(offers),
            channels: RwLock::new(channels),
            messages: RwLock::new(messages),
            identities: RwLock::new(identities),
            credentials: RwLock::new(credentials),
            reputations: RwLock::new(reputations),
            wallets: RwLock::new(wallets),
            transactions: RwLock::new(transactions),
            escrows: RwLock::new(escrows),
            invoices: RwLock::new(invoices),
            agents: RwLock::new(agents),
            conversations: RwLock::new(conversations),
            inference_backend: RwLock::new(None),
            events: events_tx,
            db: Mutex::new(db),
        })
    }

    /// Configure the inference backend used for AI chat.
    pub async fn set_inference_backend(&self, backend: Arc<dyn InferenceBackend>) {
        *self.inference_backend.write().await = Some(backend);
    }

    /// Broadcast a real-time event to all connected clients.
    pub fn emit(&self, event: RealtimeEvent) {
        let _ = self.events.send(event);
    }

    // ── Persistence helpers ────────────────────────────────────────────
    //
    // Each helper acquires the DB mutex and writes the given value to the
    // KV store.  Callers should invoke these *after* updating the in-memory
    // state so the hot path (reads) stays lock-free on the DB.

    pub async fn persist_map_entry<V: serde::Serialize>(
        &self,
        namespace: &str,
        key: &str,
        value: &V,
    ) {
        let kv_key = format!("{namespace}:{key}");
        if let Ok(bytes) = serde_json::to_vec(value) {
            let db = self.db.lock().await;
            if let Err(e) = db.put_kv(&kv_key, &bytes) {
                tracing::warn!("failed to persist {kv_key}: {e}");
            }
        }
    }

    pub async fn delete_map_entry(&self, namespace: &str, key: &str) {
        let kv_key = format!("{namespace}:{key}");
        let db = self.db.lock().await;
        if let Err(e) = db.delete_kv(&kv_key) {
            tracing::warn!("failed to delete {kv_key}: {e}");
        }
    }

    pub async fn persist_singleton<V: serde::Serialize + ?Sized>(&self, key: &str, value: &V) {
        if let Ok(bytes) = serde_json::to_vec(value) {
            let db = self.db.lock().await;
            if let Err(e) = db.put_kv(key, &bytes) {
                tracing::warn!("failed to persist {key}: {e}");
            }
        }
    }

    // ── Typed persist shortcuts ────────────────────────────────────────

    pub async fn persist_dao(&self, id: &str, dao: &Dao) {
        self.persist_map_entry(kv_ns::DAOS, id, dao).await;
    }

    pub async fn persist_proposal(&self, id: &str, proposal: &Proposal) {
        self.persist_map_entry(kv_ns::PROPOSALS, id, proposal).await;
    }

    pub async fn persist_tally(&self, id: &str, tally: &VoteTally) {
        self.persist_map_entry(kv_ns::TALLIES, id, tally).await;
    }

    pub async fn persist_private_votes(&self, id: &str, votes: &[CommittedVote]) {
        self.persist_map_entry(kv_ns::PRIVATE_VOTES, id, &votes)
            .await;
    }

    pub async fn persist_delegations(&self, delegations: &DelegationRegistry) {
        self.persist_singleton(kv_ns::DELEGATIONS, delegations)
            .await;
    }

    pub async fn persist_execution_engine(&self, engine: &ExecutionEngine) {
        self.persist_singleton(kv_ns::EXECUTION_ENGINE, engine)
            .await;
    }

    pub async fn persist_listing(&self, id: &str, listing: &Listing) {
        self.persist_map_entry(kv_ns::LISTINGS, id, listing).await;
    }

    pub async fn delete_listing(&self, id: &str) {
        self.delete_map_entry(kv_ns::LISTINGS, id).await;
    }

    pub async fn persist_review(&self, id: &str, review: &Review) {
        self.persist_map_entry(kv_ns::REVIEWS, id, review).await;
    }

    pub async fn persist_order(&self, id: &str, order: &Order) {
        self.persist_map_entry(kv_ns::ORDERS, id, order).await;
    }

    pub async fn persist_dispute(&self, id: &str, dispute: &Dispute) {
        self.persist_map_entry(kv_ns::DISPUTES, id, dispute).await;
    }

    pub async fn persist_offer(&self, id: &str, offer: &Offer) {
        self.persist_map_entry(kv_ns::OFFERS, id, offer).await;
    }

    pub async fn persist_channel(&self, id: &str, channel: &Channel) {
        self.persist_map_entry(kv_ns::CHANNELS, id, channel).await;
    }

    pub async fn persist_channel_messages(&self, channel_id: &str, msgs: &[Message]) {
        self.persist_map_entry(kv_ns::MESSAGES, channel_id, &msgs)
            .await;
    }

    pub async fn persist_identity(&self, did: &str, identity: &Identity) {
        self.persist_map_entry(kv_ns::IDENTITIES, did, identity)
            .await;
    }

    pub async fn persist_reputation(&self, did: &str, reputation: &Reputation) {
        self.persist_map_entry(kv_ns::REPUTATIONS, did, reputation)
            .await;
    }

    pub async fn persist_feed(&self, feed: &Feed) {
        self.persist_singleton(kv_ns::FEED, feed).await;
    }

    pub async fn persist_credentials(&self, did: &str, creds: &[Credential]) {
        self.persist_map_entry(kv_ns::CREDENTIALS, did, &creds)
            .await;
    }

    pub async fn persist_wallet(&self, did: &str, wallet: &Wallet) {
        self.persist_map_entry(kv_ns::WALLETS, did, wallet).await;
    }

    pub async fn persist_transactions(&self, txs: &[Transaction]) {
        self.persist_singleton(kv_ns::TRANSACTIONS, txs).await;
    }

    pub async fn persist_escrow(&self, id: &str, escrow: &Escrow) {
        self.persist_map_entry(kv_ns::ESCROWS, id, escrow).await;
    }

    pub async fn persist_invoice(&self, id: &str, invoice: &Invoice) {
        self.persist_map_entry(kv_ns::INVOICES, id, invoice).await;
    }

    pub async fn delete_invoice(&self, id: &str) {
        self.delete_map_entry(kv_ns::INVOICES, id).await;
    }

    pub async fn persist_agent(&self, id: &str, agent: &Agent) {
        self.persist_map_entry(kv_ns::AGENTS, id, agent).await;
    }

    pub async fn delete_agent_entry(&self, id: &str) {
        self.delete_map_entry(kv_ns::AGENTS, id).await;
    }

    pub async fn persist_conversation(&self, id: &str, conv: &Conversation) {
        self.persist_map_entry(kv_ns::CONVERSATIONS, id, conv).await;
    }

    pub async fn delete_conversation_entry(&self, id: &str) {
        self.delete_map_entry(kv_ns::CONVERSATIONS, id).await;
    }

    pub async fn persist_follow_graph(&self, graph: &FollowGraph) {
        self.persist_singleton(kv_ns::FOLLOW_GRAPH, graph).await;
    }

    pub async fn persist_file_store(&self, store: &FileStore) {
        self.persist_singleton(kv_ns::FILE_STORE, store).await;
    }
}

// ── KV load helpers ────────────────────────────────────────────────────

/// Load a HashMap<String, V> from the KV store by scanning for keys with the
/// given namespace prefix. Each entry is stored as "namespace:key" -> JSON(value).
fn load_kv_map<V: serde::de::DeserializeOwned>(
    db: &Database,
    namespace: &str,
) -> HashMap<String, V> {
    let prefix = format!("{namespace}:");
    let mut map = HashMap::new();

    // Scan all KV keys matching the prefix.
    let Ok(mut stmt) = db
        .conn()
        .prepare("SELECT key, value FROM kv WHERE key LIKE ?1")
    else {
        return map;
    };

    let pattern = format!("{prefix}%");
    let Ok(rows) = stmt.query_map(rusqlite::params![pattern], |row| {
        Ok((row.get::<_, String>(0)?, row.get::<_, Vec<u8>>(1)?))
    }) else {
        return map;
    };

    for row in rows.flatten() {
        let (full_key, bytes) = row;
        let short_key = full_key.strip_prefix(&prefix).unwrap_or(&full_key);
        if let Ok(val) = serde_json::from_slice::<V>(&bytes) {
            map.insert(short_key.to_string(), val);
        } else {
            tracing::warn!("failed to deserialize KV entry: {full_key}");
        }
    }

    map
}

/// Load a single value from the KV store.
fn load_kv_singleton<V: serde::de::DeserializeOwned>(db: &Database, key: &str) -> Option<V> {
    let bytes = db.get_kv(key).ok()??;
    serde_json::from_slice(&bytes).ok()
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
