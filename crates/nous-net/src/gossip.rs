use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::{HashMap, HashSet, VecDeque};
use std::time::{Duration, Instant};

/// Unique identifier for a gossip message, derived from its content hash.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct GossipId(pub String);

impl std::fmt::Display for GossipId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", &self.0[..12.min(self.0.len())])
    }
}

/// A vector clock entry for causal ordering.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct VectorClock {
    entries: HashMap<String, u64>,
}

impl VectorClock {
    pub fn new() -> Self {
        Self {
            entries: HashMap::new(),
        }
    }

    /// Increment the clock for a given node.
    pub fn increment(&mut self, node_id: &str) {
        let counter = self.entries.entry(node_id.to_string()).or_insert(0);
        *counter += 1;
    }

    /// Get the counter for a node.
    pub fn get(&self, node_id: &str) -> u64 {
        self.entries.get(node_id).copied().unwrap_or(0)
    }

    /// Merge two vector clocks, taking the maximum of each entry.
    pub fn merge(&mut self, other: &VectorClock) {
        for (key, &value) in &other.entries {
            let entry = self.entries.entry(key.clone()).or_insert(0);
            *entry = (*entry).max(value);
        }
    }

    /// Returns true if self causally precedes other (self < other).
    pub fn precedes(&self, other: &VectorClock) -> bool {
        let mut at_least_one_less = false;
        for key in self.all_keys(other) {
            let s = self.get(&key);
            let o = other.get(&key);
            if s > o {
                return false;
            }
            if s < o {
                at_least_one_less = true;
            }
        }
        at_least_one_less
    }

    /// Returns true if the two clocks are concurrent (neither precedes the other).
    pub fn is_concurrent(&self, other: &VectorClock) -> bool {
        !self.precedes(other) && !other.precedes(self) && self != other
    }

    fn all_keys(&self, other: &VectorClock) -> Vec<String> {
        let mut keys: HashSet<&String> = self.entries.keys().collect();
        keys.extend(other.entries.keys());
        keys.into_iter().cloned().collect()
    }
}

impl Default for VectorClock {
    fn default() -> Self {
        Self::new()
    }
}

/// A message in the gossip layer.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GossipMessage {
    pub id: GossipId,
    pub origin: String,
    pub payload: Vec<u8>,
    pub clock: VectorClock,
    pub timestamp_ms: u64,
    pub ttl: u8,
}

impl GossipMessage {
    pub fn new(origin: String, payload: Vec<u8>, clock: VectorClock) -> Self {
        let timestamp_ms = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as u64;

        let id = Self::compute_id(&origin, &payload, timestamp_ms);

        Self {
            id,
            origin,
            payload,
            clock,
            timestamp_ms,
            ttl: 6,
        }
    }

    /// Create a message with a specific TTL.
    pub fn with_ttl(mut self, ttl: u8) -> Self {
        self.ttl = ttl;
        self
    }

    /// Decrement TTL for forwarding. Returns false if expired.
    pub fn decrement_ttl(&mut self) -> bool {
        if self.ttl == 0 {
            return false;
        }
        self.ttl -= 1;
        true
    }

    /// Check if this message is expired based on age.
    pub fn is_expired(&self, max_age: Duration) -> bool {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as u64;
        let age_ms = now.saturating_sub(self.timestamp_ms);
        age_ms > max_age.as_millis() as u64
    }

    fn compute_id(origin: &str, payload: &[u8], timestamp_ms: u64) -> GossipId {
        let mut hasher = Sha256::new();
        hasher.update(origin.as_bytes());
        hasher.update(payload);
        hasher.update(timestamp_ms.to_be_bytes());
        GossipId(hex::encode(hasher.finalize()))
    }
}

/// Digest for anti-entropy pull-based sync.
/// Contains compact representations of known message IDs.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GossipDigest {
    /// Known message IDs with their vector clock summaries.
    pub known_ids: Vec<GossipId>,
    /// Highest clock value per origin node this peer has seen.
    pub clock_summary: HashMap<String, u64>,
}

impl GossipDigest {
    pub fn new() -> Self {
        Self {
            known_ids: Vec::new(),
            clock_summary: HashMap::new(),
        }
    }

    /// Compute the set of IDs the remote is missing based on local state.
    pub fn missing_from(&self, local_ids: &HashSet<GossipId>) -> Vec<GossipId> {
        let remote_set: HashSet<&GossipId> = self.known_ids.iter().collect();
        local_ids
            .iter()
            .filter(|id| !remote_set.contains(id))
            .cloned()
            .collect()
    }
}

impl Default for GossipDigest {
    fn default() -> Self {
        Self::new()
    }
}

/// Configuration for the gossip layer.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GossipConfig {
    /// Number of peers to push each message to (fanout).
    pub fanout: usize,
    /// Maximum age before a message is garbage collected.
    pub max_message_age: Duration,
    /// Maximum messages to retain in the buffer.
    pub max_buffer_size: usize,
    /// Interval between anti-entropy rounds.
    pub sync_interval: Duration,
    /// Maximum TTL for messages.
    pub max_ttl: u8,
}

impl Default for GossipConfig {
    fn default() -> Self {
        Self {
            fanout: 6,
            max_message_age: Duration::from_secs(300),
            max_buffer_size: 10_000,
            sync_interval: Duration::from_secs(30),
            max_ttl: 8,
        }
    }
}

/// Tracks per-peer gossip state for protocol coordination.
#[derive(Debug)]
struct PeerGossipState {
    /// Messages already sent to this peer (avoid re-sending).
    sent: HashSet<GossipId>,
    /// Last time we synced with this peer.
    last_sync: Instant,
    /// Consecutive sync failures for backoff.
    failures: u32,
}

impl PeerGossipState {
    fn new() -> Self {
        Self {
            sent: HashSet::new(),
            last_sync: Instant::now(),
            failures: 0,
        }
    }

    /// Exponential backoff duration based on failure count.
    fn backoff_duration(&self, base_interval: Duration) -> Duration {
        if self.failures == 0 {
            return base_interval;
        }
        let multiplier = 2u32.saturating_pow(self.failures.min(6));
        base_interval * multiplier
    }

    /// Whether enough time has passed to sync with this peer again.
    fn should_sync(&self, base_interval: Duration) -> bool {
        self.last_sync.elapsed() >= self.backoff_duration(base_interval)
    }
}

/// Actions the gossip layer wants the node to take.
#[derive(Debug, Clone)]
pub enum GossipAction {
    /// Push this message to these peers.
    Forward {
        message: GossipMessage,
        targets: Vec<String>,
    },
    /// Send a digest to this peer for anti-entropy sync.
    SendDigest {
        peer: String,
        digest: GossipDigest,
    },
    /// Send specific messages to a peer who is missing them.
    SendMissing {
        peer: String,
        messages: Vec<GossipMessage>,
    },
}

/// The gossip protocol engine.
///
/// Implements epidemic-style message dissemination with:
/// - Push-based rumor spreading with configurable fanout
/// - Pull-based anti-entropy with digest exchange
/// - Causal ordering via vector clocks
/// - TTL-based flood control
/// - Per-peer deduplication tracking
pub struct GossipEngine {
    config: GossipConfig,
    local_id: String,
    /// All known messages, keyed by ID.
    messages: HashMap<GossipId, GossipMessage>,
    /// Insertion order for eviction.
    insertion_order: VecDeque<GossipId>,
    /// Per-peer gossip state.
    peer_states: HashMap<String, PeerGossipState>,
    /// Our local vector clock.
    clock: VectorClock,
    /// Count of messages received.
    received_count: u64,
    /// Count of messages forwarded.
    forwarded_count: u64,
    /// Count of duplicate messages dropped.
    duplicate_count: u64,
}

impl GossipEngine {
    pub fn new(local_id: String, config: GossipConfig) -> Self {
        Self {
            config,
            local_id,
            messages: HashMap::new(),
            insertion_order: VecDeque::new(),
            peer_states: HashMap::new(),
            clock: VectorClock::new(),
            received_count: 0,
            forwarded_count: 0,
            duplicate_count: 0,
        }
    }

    pub fn local_id(&self) -> &str {
        &self.local_id
    }

    pub fn clock(&self) -> &VectorClock {
        &self.clock
    }

    pub fn message_count(&self) -> usize {
        self.messages.len()
    }

    pub fn received_count(&self) -> u64 {
        self.received_count
    }

    pub fn forwarded_count(&self) -> u64 {
        self.forwarded_count
    }

    pub fn duplicate_count(&self) -> u64 {
        self.duplicate_count
    }

    pub fn peer_count(&self) -> usize {
        self.peer_states.len()
    }

    /// Register a peer in the gossip layer.
    pub fn add_peer(&mut self, peer_id: String) {
        self.peer_states
            .entry(peer_id)
            .or_insert_with(PeerGossipState::new);
    }

    /// Remove a peer from the gossip layer.
    pub fn remove_peer(&mut self, peer_id: &str) {
        self.peer_states.remove(peer_id);
    }

    /// Create a new gossip message from local content.
    /// Returns the message and actions to disseminate it.
    pub fn originate(&mut self, payload: Vec<u8>) -> (GossipMessage, Vec<GossipAction>) {
        self.clock.increment(&self.local_id);
        let mut msg = GossipMessage::new(self.local_id.clone(), payload, self.clock.clone());
        msg.ttl = msg.ttl.min(self.config.max_ttl);

        let actions = self.push_to_peers(&msg);
        self.store_message(msg.clone());
        self.received_count += 1;

        (msg, actions)
    }

    /// Handle a received gossip message from a peer.
    /// Returns actions to take (forwarding, etc.) and whether the message was new.
    pub fn receive(
        &mut self,
        mut msg: GossipMessage,
        from_peer: &str,
    ) -> (Vec<GossipAction>, bool) {
        // Mark as sent by this peer (they clearly have it).
        if let Some(state) = self.peer_states.get_mut(from_peer) {
            state.sent.insert(msg.id.clone());
        }

        // Duplicate check.
        if self.messages.contains_key(&msg.id) {
            self.duplicate_count += 1;
            return (Vec::new(), false);
        }

        // TTL check.
        if !msg.decrement_ttl() {
            return (Vec::new(), false);
        }

        // Age check.
        if msg.is_expired(self.config.max_message_age) {
            return (Vec::new(), false);
        }

        // Merge the remote clock into ours.
        self.clock.merge(&msg.clock);

        // Forward to other peers.
        let actions = self.push_to_peers_excluding(&msg, from_peer);
        self.forwarded_count += actions.len() as u64;

        self.store_message(msg);
        self.received_count += 1;

        (actions, true)
    }

    /// Handle a received digest from a peer for anti-entropy.
    /// Returns messages that the peer is missing.
    pub fn handle_digest(&self, digest: &GossipDigest, from_peer: &str) -> Vec<GossipAction> {
        let local_ids: HashSet<GossipId> = self.messages.keys().cloned().collect();
        let missing = digest.missing_from(&local_ids);

        if missing.is_empty() {
            return Vec::new();
        }

        let messages: Vec<GossipMessage> = missing
            .iter()
            .filter_map(|id| self.messages.get(id).cloned())
            .collect();

        if messages.is_empty() {
            return Vec::new();
        }

        vec![GossipAction::SendMissing {
            peer: from_peer.to_string(),
            messages,
        }]
    }

    /// Build a digest of our current state for anti-entropy sync.
    pub fn build_digest(&self) -> GossipDigest {
        let known_ids: Vec<GossipId> = self.messages.keys().cloned().collect();
        let mut clock_summary = HashMap::new();

        for msg in self.messages.values() {
            let entry = clock_summary
                .entry(msg.origin.clone())
                .or_insert(0u64);
            *entry = (*entry).max(msg.clock.get(&msg.origin));
        }

        GossipDigest {
            known_ids,
            clock_summary,
        }
    }

    /// Periodic tick: garbage collect expired messages and produce sync actions.
    pub fn tick(&mut self) -> Vec<GossipAction> {
        self.gc_expired();
        self.produce_sync_actions()
    }

    /// Get a message by ID.
    pub fn get_message(&self, id: &GossipId) -> Option<&GossipMessage> {
        self.messages.get(id)
    }

    /// Check if a peer has been told about a specific message.
    pub fn peer_has_message(&self, peer_id: &str, msg_id: &GossipId) -> bool {
        self.peer_states
            .get(peer_id)
            .is_some_and(|state| state.sent.contains(msg_id))
    }

    /// Record a sync failure for a peer (increases backoff).
    pub fn record_sync_failure(&mut self, peer_id: &str) {
        if let Some(state) = self.peer_states.get_mut(peer_id) {
            state.failures = state.failures.saturating_add(1);
        }
    }

    /// Record a successful sync with a peer (resets backoff).
    pub fn record_sync_success(&mut self, peer_id: &str) {
        if let Some(state) = self.peer_states.get_mut(peer_id) {
            state.failures = 0;
            state.last_sync = Instant::now();
        }
    }

    fn store_message(&mut self, msg: GossipMessage) {
        let id = msg.id.clone();
        self.messages.insert(id.clone(), msg);
        self.insertion_order.push_back(id);
        self.evict_if_full();
    }

    fn evict_if_full(&mut self) {
        while self.messages.len() > self.config.max_buffer_size {
            if let Some(oldest_id) = self.insertion_order.pop_front() {
                self.messages.remove(&oldest_id);
            }
        }
    }

    fn gc_expired(&mut self) {
        let max_age = self.config.max_message_age;
        let expired: Vec<GossipId> = self
            .messages
            .iter()
            .filter(|(_, msg)| msg.is_expired(max_age))
            .map(|(id, _)| id.clone())
            .collect();

        for id in &expired {
            self.messages.remove(id);
        }

        self.insertion_order.retain(|id| self.messages.contains_key(id));
    }

    fn push_to_peers(&mut self, msg: &GossipMessage) -> Vec<GossipAction> {
        self.push_to_peers_excluding(msg, "")
    }

    fn push_to_peers_excluding(
        &mut self,
        msg: &GossipMessage,
        exclude: &str,
    ) -> Vec<GossipAction> {
        let targets: Vec<String> = self
            .peer_states
            .keys()
            .filter(|peer| *peer != exclude && *peer != &self.local_id)
            .filter(|peer| {
                self.peer_states
                    .get(*peer)
                    .is_some_and(|state| !state.sent.contains(&msg.id))
            })
            .take(self.config.fanout)
            .cloned()
            .collect();

        if targets.is_empty() {
            return Vec::new();
        }

        // Mark as sent to these peers.
        for target in &targets {
            if let Some(state) = self.peer_states.get_mut(target) {
                state.sent.insert(msg.id.clone());
            }
        }

        vec![GossipAction::Forward {
            message: msg.clone(),
            targets,
        }]
    }

    fn produce_sync_actions(&mut self) -> Vec<GossipAction> {
        let sync_interval = self.config.sync_interval;
        let peers_needing_sync: Vec<String> = self
            .peer_states
            .iter()
            .filter(|(peer, _)| *peer != &self.local_id)
            .filter(|(_, state)| state.should_sync(sync_interval))
            .map(|(peer, _)| peer.clone())
            .collect();

        let digest = self.build_digest();

        let mut actions = Vec::new();
        for peer in peers_needing_sync {
            if let Some(state) = self.peer_states.get_mut(&peer) {
                state.last_sync = Instant::now();
            }
            actions.push(GossipAction::SendDigest {
                peer,
                digest: digest.clone(),
            });
        }
        actions
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_engine(id: &str) -> GossipEngine {
        GossipEngine::new(
            id.to_string(),
            GossipConfig {
                fanout: 3,
                max_message_age: Duration::from_secs(60),
                max_buffer_size: 100,
                sync_interval: Duration::from_millis(10),
                max_ttl: 6,
                ..Default::default()
            },
        )
    }

    #[test]
    fn vector_clock_increment() {
        let mut clock = VectorClock::new();
        clock.increment("a");
        clock.increment("a");
        clock.increment("b");
        assert_eq!(clock.get("a"), 2);
        assert_eq!(clock.get("b"), 1);
        assert_eq!(clock.get("c"), 0);
    }

    #[test]
    fn vector_clock_merge() {
        let mut c1 = VectorClock::new();
        c1.increment("a");
        c1.increment("a");

        let mut c2 = VectorClock::new();
        c2.increment("a");
        c2.increment("b");
        c2.increment("b");
        c2.increment("b");

        c1.merge(&c2);
        assert_eq!(c1.get("a"), 2); // max(2, 1) = 2
        assert_eq!(c1.get("b"), 3); // max(0, 3) = 3
    }

    #[test]
    fn vector_clock_precedes() {
        let mut c1 = VectorClock::new();
        c1.increment("a");

        let mut c2 = VectorClock::new();
        c2.increment("a");
        c2.increment("a");

        assert!(c1.precedes(&c2));
        assert!(!c2.precedes(&c1));
    }

    #[test]
    fn vector_clock_concurrent() {
        let mut c1 = VectorClock::new();
        c1.increment("a");

        let mut c2 = VectorClock::new();
        c2.increment("b");

        assert!(c1.is_concurrent(&c2));
        assert!(c2.is_concurrent(&c1));
    }

    #[test]
    fn vector_clock_equal_not_concurrent() {
        let mut c1 = VectorClock::new();
        c1.increment("a");
        let c2 = c1.clone();

        assert!(!c1.is_concurrent(&c2));
        assert!(!c1.precedes(&c2));
    }

    #[test]
    fn gossip_message_id_deterministic() {
        let clock = VectorClock::new();
        let id1 = GossipMessage::compute_id("node-a", b"hello", 1000);
        let id2 = GossipMessage::compute_id("node-a", b"hello", 1000);
        let id3 = GossipMessage::compute_id("node-a", b"world", 1000);

        assert_eq!(id1, id2);
        assert_ne!(id1, id3);
        let _ = clock;
    }

    #[test]
    fn gossip_message_ttl_decrement() {
        let msg = GossipMessage::new("node-a".into(), b"data".to_vec(), VectorClock::new());
        let mut msg = msg.with_ttl(2);

        assert!(msg.decrement_ttl()); // 2 -> 1
        assert!(msg.decrement_ttl()); // 1 -> 0
        assert!(!msg.decrement_ttl()); // 0 -> can't
    }

    #[test]
    fn gossip_message_ttl_zero_rejected() {
        let mut msg =
            GossipMessage::new("node-a".into(), b"data".to_vec(), VectorClock::new()).with_ttl(0);
        assert!(!msg.decrement_ttl());
    }

    #[test]
    fn engine_originate_message() {
        let mut engine = make_engine("node-a");
        engine.add_peer("node-b".into());
        engine.add_peer("node-c".into());

        let (msg, actions) = engine.originate(b"hello world".to_vec());

        assert_eq!(msg.origin, "node-a");
        assert_eq!(msg.payload, b"hello world");
        assert_eq!(engine.message_count(), 1);
        assert_eq!(engine.received_count(), 1);
        assert!(!actions.is_empty());
    }

    #[test]
    fn engine_receive_new_message() {
        let mut engine = make_engine("node-b");
        engine.add_peer("node-a".into());
        engine.add_peer("node-c".into());

        let msg = GossipMessage::new("node-a".into(), b"hello".to_vec(), VectorClock::new());
        let (actions, is_new) = engine.receive(msg, "node-a");

        assert!(is_new);
        assert_eq!(engine.message_count(), 1);
        assert_eq!(engine.received_count(), 1);
        // Should forward to node-c (not back to node-a).
        if let Some(GossipAction::Forward { targets, .. }) = actions.first() {
            assert!(!targets.contains(&"node-a".to_string()));
        }
    }

    #[test]
    fn engine_rejects_duplicate() {
        let mut engine = make_engine("node-b");
        engine.add_peer("node-a".into());

        let msg = GossipMessage::new("node-a".into(), b"dup".to_vec(), VectorClock::new());
        let (_, is_new) = engine.receive(msg.clone(), "node-a");
        assert!(is_new);

        let (_, is_dup) = engine.receive(msg, "node-a");
        assert!(!is_dup);
        assert_eq!(engine.duplicate_count(), 1);
    }

    #[test]
    fn engine_respects_fanout() {
        let mut engine = make_engine("node-a");
        // Add 10 peers but fanout is 3.
        for i in 0..10 {
            engine.add_peer(format!("peer-{i}"));
        }

        let (_, actions) = engine.originate(b"broadcast".to_vec());
        if let Some(GossipAction::Forward { targets, .. }) = actions.first() {
            assert!(targets.len() <= 3);
        }
    }

    #[test]
    fn engine_does_not_forward_to_sender() {
        let mut engine = make_engine("node-b");
        engine.add_peer("node-a".into());

        let msg = GossipMessage::new("node-a".into(), b"test".to_vec(), VectorClock::new());
        let (actions, _) = engine.receive(msg, "node-a");

        for action in &actions {
            if let GossipAction::Forward { targets, .. } = action {
                assert!(!targets.contains(&"node-a".to_string()));
            }
        }
    }

    #[test]
    fn engine_evicts_oldest_when_full() {
        let mut engine = GossipEngine::new(
            "node-a".into(),
            GossipConfig {
                max_buffer_size: 3,
                ..Default::default()
            },
        );

        let msg1 = GossipMessage::new("node-a".into(), b"msg1".to_vec(), VectorClock::new());
        let msg2 = GossipMessage::new("node-a".into(), b"msg2".to_vec(), VectorClock::new());
        let msg3 = GossipMessage::new("node-a".into(), b"msg3".to_vec(), VectorClock::new());
        let msg4 = GossipMessage::new("node-a".into(), b"msg4".to_vec(), VectorClock::new());
        let id1 = msg1.id.clone();

        engine.store_message(msg1);
        engine.store_message(msg2);
        engine.store_message(msg3);
        assert_eq!(engine.message_count(), 3);

        engine.store_message(msg4);
        assert_eq!(engine.message_count(), 3);
        assert!(engine.get_message(&id1).is_none()); // Oldest evicted.
    }

    #[test]
    fn engine_build_digest() {
        let mut engine = make_engine("node-a");
        engine.originate(b"msg1".to_vec());
        engine.originate(b"msg2".to_vec());

        let digest = engine.build_digest();
        assert_eq!(digest.known_ids.len(), 2);
        assert!(digest.clock_summary.contains_key("node-a"));
    }

    #[test]
    fn engine_handle_digest_finds_missing() {
        let mut engine = make_engine("node-a");
        engine.add_peer("node-b".into());
        engine.originate(b"msg1".to_vec());
        engine.originate(b"msg2".to_vec());

        // Remote has nothing.
        let empty_digest = GossipDigest::new();
        let actions = engine.handle_digest(&empty_digest, "node-b");

        assert_eq!(actions.len(), 1);
        if let GossipAction::SendMissing { messages, .. } = &actions[0] {
            assert_eq!(messages.len(), 2);
        } else {
            panic!("expected SendMissing action");
        }
    }

    #[test]
    fn engine_handle_digest_nothing_missing() {
        let mut engine = make_engine("node-a");
        engine.originate(b"msg1".to_vec());

        let mut digest = GossipDigest::new();
        // Remote knows everything we know.
        for id in engine.messages.keys() {
            digest.known_ids.push(id.clone());
        }

        let actions = engine.handle_digest(&digest, "node-b");
        assert!(actions.is_empty());
    }

    #[test]
    fn engine_peer_tracking() {
        let mut engine = make_engine("node-a");
        engine.add_peer("peer-1".into());
        engine.add_peer("peer-2".into());
        assert_eq!(engine.peer_count(), 2);

        engine.remove_peer("peer-1");
        assert_eq!(engine.peer_count(), 1);
    }

    #[test]
    fn engine_sync_backoff() {
        let mut engine = make_engine("node-a");
        engine.add_peer("node-b".into());

        engine.record_sync_failure("node-b");
        engine.record_sync_failure("node-b");
        engine.record_sync_failure("node-b");

        // After 3 failures, backoff should be 2^3 = 8x the base interval.
        let state = engine.peer_states.get("node-b").unwrap();
        let backoff = state.backoff_duration(Duration::from_secs(30));
        assert_eq!(backoff, Duration::from_secs(240));

        // Reset on success.
        engine.record_sync_success("node-b");
        let state = engine.peer_states.get("node-b").unwrap();
        assert_eq!(state.failures, 0);
    }

    #[test]
    fn engine_clock_advances_on_originate() {
        let mut engine = make_engine("node-a");
        assert_eq!(engine.clock().get("node-a"), 0);

        engine.originate(b"msg1".to_vec());
        assert_eq!(engine.clock().get("node-a"), 1);

        engine.originate(b"msg2".to_vec());
        assert_eq!(engine.clock().get("node-a"), 2);
    }

    #[test]
    fn engine_clock_merges_on_receive() {
        let mut engine = make_engine("node-b");
        engine.add_peer("node-a".into());

        let mut clock = VectorClock::new();
        clock.increment("node-a");
        clock.increment("node-a");
        clock.increment("node-a");

        let msg = GossipMessage::new("node-a".into(), b"data".to_vec(), clock);
        engine.receive(msg, "node-a");

        assert_eq!(engine.clock().get("node-a"), 3);
    }

    #[test]
    fn gossip_id_display_truncates() {
        let id = GossipId("abcdef1234567890abcdef".to_string());
        let display = format!("{id}");
        assert_eq!(display, "abcdef123456");
    }

    #[test]
    fn gossip_config_default() {
        let config = GossipConfig::default();
        assert_eq!(config.fanout, 6);
        assert_eq!(config.max_ttl, 8);
        assert_eq!(config.max_buffer_size, 10_000);
    }

    #[test]
    fn gossip_message_serializes() {
        let msg = GossipMessage::new("node-a".into(), b"test".to_vec(), VectorClock::new());
        let json = serde_json::to_string(&msg).unwrap();
        let decoded: GossipMessage = serde_json::from_str(&json).unwrap();
        assert_eq!(decoded.id, msg.id);
        assert_eq!(decoded.origin, msg.origin);
        assert_eq!(decoded.payload, msg.payload);
    }

    #[test]
    fn digest_missing_from() {
        let mut local_ids = HashSet::new();
        local_ids.insert(GossipId("aaa".into()));
        local_ids.insert(GossipId("bbb".into()));
        local_ids.insert(GossipId("ccc".into()));

        let digest = GossipDigest {
            known_ids: vec![GossipId("aaa".into())],
            clock_summary: HashMap::new(),
        };

        let missing = digest.missing_from(&local_ids);
        assert_eq!(missing.len(), 2);
        assert!(missing.contains(&GossipId("bbb".into())));
        assert!(missing.contains(&GossipId("ccc".into())));
    }

    #[test]
    fn engine_marks_peer_as_having_message() {
        let mut engine = make_engine("node-b");
        engine.add_peer("node-a".into());

        let msg = GossipMessage::new("node-a".into(), b"data".to_vec(), VectorClock::new());
        let msg_id = msg.id.clone();
        engine.receive(msg, "node-a");

        assert!(engine.peer_has_message("node-a", &msg_id));
        assert!(!engine.peer_has_message("node-c", &msg_id));
    }
}
