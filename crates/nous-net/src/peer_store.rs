use libp2p::{Multiaddr, PeerId};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Persistent record of a known peer.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PeerRecord {
    pub peer_id: String,
    pub addresses: Vec<String>,
    pub last_seen_ms: u64,
    pub score: i64,
    pub connection_count: u64,
    pub failed_dials: u64,
}

impl PeerRecord {
    fn new(peer_id: PeerId, addr: Multiaddr) -> Self {
        Self {
            peer_id: peer_id.to_string(),
            addresses: vec![addr.to_string()],
            last_seen_ms: now_ms(),
            score: 0,
            connection_count: 0,
            failed_dials: 0,
        }
    }
}

/// In-memory peer store with scoring and pruning.
///
/// Tracks known peers, their addresses, reputation scores, and connection
/// history. Designed to be serializable for persistence across restarts.
pub struct PeerStore {
    peers: HashMap<String, PeerRecord>,
    max_peers: usize,
}

impl PeerStore {
    pub fn new(max_peers: usize) -> Self {
        Self {
            peers: HashMap::new(),
            max_peers,
        }
    }

    /// Record a peer sighting. Adds the peer if new, or updates last_seen and
    /// merges addresses if known.
    pub fn record_peer(&mut self, peer_id: PeerId, addr: Multiaddr) {
        let key = peer_id.to_string();
        let addr_str = addr.to_string();

        if let Some(record) = self.peers.get_mut(&key) {
            record.last_seen_ms = now_ms();
            if !record.addresses.contains(&addr_str) {
                record.addresses.push(addr_str);
            }
        } else {
            if self.peers.len() >= self.max_peers {
                self.evict_lowest_score();
            }
            self.peers.insert(key, PeerRecord::new(peer_id, addr));
        }
    }

    /// Remove a peer from the store.
    pub fn remove_peer(&mut self, peer_id: &PeerId) -> Option<PeerRecord> {
        self.peers.remove(&peer_id.to_string())
    }

    /// Look up a peer record.
    pub fn get_peer(&self, peer_id: &PeerId) -> Option<&PeerRecord> {
        self.peers.get(&peer_id.to_string())
    }

    /// Adjust a peer's reputation score.
    pub fn adjust_score(&mut self, peer_id: &PeerId, delta: i64) {
        if let Some(record) = self.peers.get_mut(&peer_id.to_string()) {
            record.score = record.score.saturating_add(delta);
        }
    }

    /// Record a successful connection.
    pub fn record_connection(&mut self, peer_id: &PeerId) {
        if let Some(record) = self.peers.get_mut(&peer_id.to_string()) {
            record.connection_count += 1;
            record.last_seen_ms = now_ms();
        }
    }

    /// Record a failed dial attempt.
    pub fn record_dial_failure(&mut self, peer_id: &PeerId) {
        if let Some(record) = self.peers.get_mut(&peer_id.to_string()) {
            record.failed_dials += 1;
            record.score = record.score.saturating_sub(1);
        }
    }

    /// Return the `n` best-scored peers.
    pub fn best_peers(&self, n: usize) -> Vec<&PeerRecord> {
        let mut sorted: Vec<_> = self.peers.values().collect();
        sorted.sort_by(|a, b| b.score.cmp(&a.score));
        sorted.truncate(n);
        sorted
    }

    /// Remove peers not seen within `max_age`.
    pub fn prune_stale(&mut self, max_age_ms: u64) -> usize {
        let cutoff = now_ms().saturating_sub(max_age_ms);
        let before = self.peers.len();
        self.peers.retain(|_, r| r.last_seen_ms >= cutoff);
        before - self.peers.len()
    }

    /// Total known peers.
    pub fn len(&self) -> usize {
        self.peers.len()
    }

    pub fn is_empty(&self) -> bool {
        self.peers.is_empty()
    }

    /// Export all records for serialization.
    pub fn export(&self) -> Vec<PeerRecord> {
        self.peers.values().cloned().collect()
    }

    /// Import records from a previous session.
    pub fn import(&mut self, records: Vec<PeerRecord>) {
        for record in records {
            if self.peers.len() < self.max_peers {
                self.peers
                    .entry(record.peer_id.clone())
                    .or_insert(record);
            }
        }
    }

    fn evict_lowest_score(&mut self) {
        if let Some(key) = self
            .peers
            .iter()
            .min_by_key(|(_, r)| r.score)
            .map(|(k, _)| k.clone())
        {
            self.peers.remove(&key);
        }
    }
}

fn now_ms() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_peer() -> PeerId {
        PeerId::random()
    }

    fn test_addr() -> Multiaddr {
        "/ip4/127.0.0.1/tcp/4001".parse().unwrap()
    }

    fn test_addr_2() -> Multiaddr {
        "/ip4/192.168.1.1/tcp/4001".parse().unwrap()
    }

    #[test]
    fn new_store_is_empty() {
        let store = PeerStore::new(100);
        assert!(store.is_empty());
        assert_eq!(store.len(), 0);
    }

    #[test]
    fn record_peer_adds_entry() {
        let mut store = PeerStore::new(100);
        let peer = test_peer();
        store.record_peer(peer, test_addr());

        assert_eq!(store.len(), 1);
        let record = store.get_peer(&peer).unwrap();
        assert_eq!(record.addresses.len(), 1);
    }

    #[test]
    fn record_peer_merges_addresses() {
        let mut store = PeerStore::new(100);
        let peer = test_peer();

        store.record_peer(peer, test_addr());
        store.record_peer(peer, test_addr_2());

        let record = store.get_peer(&peer).unwrap();
        assert_eq!(record.addresses.len(), 2);
    }

    #[test]
    fn record_peer_deduplicates_addresses() {
        let mut store = PeerStore::new(100);
        let peer = test_peer();

        store.record_peer(peer, test_addr());
        store.record_peer(peer, test_addr());

        let record = store.get_peer(&peer).unwrap();
        assert_eq!(record.addresses.len(), 1);
    }

    #[test]
    fn remove_peer_returns_record() {
        let mut store = PeerStore::new(100);
        let peer = test_peer();
        store.record_peer(peer, test_addr());

        let removed = store.remove_peer(&peer);
        assert!(removed.is_some());
        assert!(store.is_empty());
    }

    #[test]
    fn remove_unknown_peer_returns_none() {
        let mut store = PeerStore::new(100);
        assert!(store.remove_peer(&test_peer()).is_none());
    }

    #[test]
    fn adjust_score() {
        let mut store = PeerStore::new(100);
        let peer = test_peer();
        store.record_peer(peer, test_addr());

        store.adjust_score(&peer, 5);
        assert_eq!(store.get_peer(&peer).unwrap().score, 5);

        store.adjust_score(&peer, -3);
        assert_eq!(store.get_peer(&peer).unwrap().score, 2);
    }

    #[test]
    fn score_saturates() {
        let mut store = PeerStore::new(100);
        let peer = test_peer();
        store.record_peer(peer, test_addr());

        store.adjust_score(&peer, i64::MAX);
        store.adjust_score(&peer, 1);
        assert_eq!(store.get_peer(&peer).unwrap().score, i64::MAX);
    }

    #[test]
    fn record_connection_increments() {
        let mut store = PeerStore::new(100);
        let peer = test_peer();
        store.record_peer(peer, test_addr());

        store.record_connection(&peer);
        store.record_connection(&peer);
        assert_eq!(store.get_peer(&peer).unwrap().connection_count, 2);
    }

    #[test]
    fn record_dial_failure_decrements_score() {
        let mut store = PeerStore::new(100);
        let peer = test_peer();
        store.record_peer(peer, test_addr());

        store.record_dial_failure(&peer);
        let record = store.get_peer(&peer).unwrap();
        assert_eq!(record.failed_dials, 1);
        assert_eq!(record.score, -1);
    }

    #[test]
    fn best_peers_sorted_by_score() {
        let mut store = PeerStore::new(100);
        let p1 = test_peer();
        let p2 = test_peer();
        let p3 = test_peer();

        store.record_peer(p1, test_addr());
        store.record_peer(p2, test_addr());
        store.record_peer(p3, test_addr());

        store.adjust_score(&p1, 10);
        store.adjust_score(&p2, 30);
        store.adjust_score(&p3, 20);

        let best = store.best_peers(2);
        assert_eq!(best.len(), 2);
        assert_eq!(best[0].score, 30);
        assert_eq!(best[1].score, 20);
    }

    #[test]
    fn best_peers_handles_fewer_than_n() {
        let mut store = PeerStore::new(100);
        store.record_peer(test_peer(), test_addr());
        assert_eq!(store.best_peers(10).len(), 1);
    }

    #[test]
    fn eviction_at_capacity() {
        let mut store = PeerStore::new(2);
        let p1 = test_peer();
        let p2 = test_peer();
        let p3 = test_peer();

        store.record_peer(p1, test_addr());
        store.adjust_score(&p1, 10);
        store.record_peer(p2, test_addr());
        store.adjust_score(&p2, -5);

        // p3 should evict p2 (lowest score).
        store.record_peer(p3, test_addr());
        assert_eq!(store.len(), 2);
        assert!(store.get_peer(&p1).is_some());
        assert!(store.get_peer(&p2).is_none());
        assert!(store.get_peer(&p3).is_some());
    }

    #[test]
    fn export_import_roundtrip() {
        let mut store = PeerStore::new(100);
        let p1 = test_peer();
        store.record_peer(p1, test_addr());
        store.adjust_score(&p1, 42);

        let exported = store.export();
        assert_eq!(exported.len(), 1);

        let mut store2 = PeerStore::new(100);
        store2.import(exported);
        assert_eq!(store2.len(), 1);
        let record = store2.peers.values().next().unwrap();
        assert_eq!(record.score, 42);
    }

    #[test]
    fn import_respects_capacity() {
        let mut store = PeerStore::new(1);
        let records = vec![
            PeerRecord {
                peer_id: test_peer().to_string(),
                addresses: vec![test_addr().to_string()],
                last_seen_ms: now_ms(),
                score: 10,
                connection_count: 0,
                failed_dials: 0,
            },
            PeerRecord {
                peer_id: test_peer().to_string(),
                addresses: vec![test_addr().to_string()],
                last_seen_ms: now_ms(),
                score: 20,
                connection_count: 0,
                failed_dials: 0,
            },
        ];

        store.import(records);
        assert_eq!(store.len(), 1);
    }

    #[test]
    fn peer_record_serializes() {
        let record = PeerRecord {
            peer_id: test_peer().to_string(),
            addresses: vec![test_addr().to_string()],
            last_seen_ms: 1000,
            score: 5,
            connection_count: 3,
            failed_dials: 1,
        };

        let json = serde_json::to_string(&record).unwrap();
        let deserialized: PeerRecord = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.score, 5);
        assert_eq!(deserialized.connection_count, 3);
    }
}
