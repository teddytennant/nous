use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::time::{Duration, Instant};

/// Health status of a bootstrap node.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum BootstrapHealth {
    Healthy,
    Degraded,
    Unreachable,
    Unknown,
}

/// A tracked bootstrap node with health metrics.
#[derive(Debug, Clone)]
pub struct BootstrapNode {
    pub address: String,
    pub health: BootstrapHealth,
    /// Round-trip latency from last successful probe.
    pub latency: Option<Duration>,
    /// Total successful connections.
    pub successes: u64,
    /// Total failed connection attempts.
    pub failures: u64,
    /// Last time this node was successfully contacted.
    last_success: Option<Instant>,
    /// Last time a connection attempt was made.
    last_attempt: Option<Instant>,
}

impl BootstrapNode {
    pub fn new(address: String) -> Self {
        Self {
            address,
            health: BootstrapHealth::Unknown,
            latency: None,
            successes: 0,
            failures: 0,
            last_success: None,
            last_attempt: None,
        }
    }

    /// Record a successful connection with measured latency.
    pub fn record_success(&mut self, latency: Duration) {
        self.successes += 1;
        self.latency = Some(latency);
        let now = Instant::now();
        self.last_success = Some(now);
        self.last_attempt = Some(now);
        self.update_health();
    }

    /// Record a failed connection attempt.
    pub fn record_failure(&mut self) {
        self.failures += 1;
        self.last_attempt = Some(Instant::now());
        self.update_health();
    }

    /// Success rate as a fraction [0.0, 1.0].
    pub fn success_rate(&self) -> f64 {
        let total = self.successes + self.failures;
        if total == 0 {
            return 0.0;
        }
        self.successes as f64 / total as f64
    }

    /// Time since last successful contact, if any.
    pub fn time_since_success(&self) -> Option<Duration> {
        self.last_success.map(|t| t.elapsed())
    }

    /// Time since last attempt of any kind, if any.
    pub fn time_since_attempt(&self) -> Option<Duration> {
        self.last_attempt.map(|t| t.elapsed())
    }

    fn update_health(&mut self) {
        let rate = self.success_rate();
        let recent_success = self
            .last_success
            .is_some_and(|t| t.elapsed() < Duration::from_secs(300));

        self.health = if rate >= 0.8 && recent_success {
            BootstrapHealth::Healthy
        } else if rate >= 0.4 || recent_success {
            BootstrapHealth::Degraded
        } else {
            BootstrapHealth::Unreachable
        };
    }
}

/// Manager for bootstrap nodes with health tracking and rotation.
pub struct BootstrapManager {
    nodes: Vec<BootstrapNode>,
    /// Index of the next node to try (round-robin with health weighting).
    next_index: usize,
}

impl BootstrapManager {
    pub fn new() -> Self {
        Self {
            nodes: Vec::new(),
            next_index: 0,
        }
    }

    /// Add a bootstrap node address.
    pub fn add(&mut self, address: String) {
        if !self.nodes.iter().any(|n| n.address == address) {
            self.nodes.push(BootstrapNode::new(address));
        }
    }

    /// Remove a bootstrap node by address.
    pub fn remove(&mut self, address: &str) -> bool {
        let before = self.nodes.len();
        self.nodes.retain(|n| n.address != address);
        let removed = self.nodes.len() < before;
        if removed && self.next_index >= self.nodes.len() && !self.nodes.is_empty() {
            self.next_index = 0;
        }
        removed
    }

    /// Get the next healthy bootstrap node to try, preferring healthy nodes.
    pub fn next_healthy(&mut self) -> Option<&BootstrapNode> {
        if self.nodes.is_empty() {
            return None;
        }

        // First pass: try to find a healthy node.
        let start = self.next_index;
        for i in 0..self.nodes.len() {
            let idx = (start + i) % self.nodes.len();
            if self.nodes[idx].health == BootstrapHealth::Healthy
                || self.nodes[idx].health == BootstrapHealth::Unknown
            {
                self.next_index = (idx + 1) % self.nodes.len();
                return Some(&self.nodes[idx]);
            }
        }

        // Second pass: try degraded.
        for i in 0..self.nodes.len() {
            let idx = (start + i) % self.nodes.len();
            if self.nodes[idx].health == BootstrapHealth::Degraded {
                self.next_index = (idx + 1) % self.nodes.len();
                return Some(&self.nodes[idx]);
            }
        }

        // Last resort: any node.
        self.next_index = (start + 1) % self.nodes.len();
        Some(&self.nodes[start % self.nodes.len()])
    }

    /// Record success for a node by address.
    pub fn record_success(&mut self, address: &str, latency: Duration) {
        if let Some(node) = self.nodes.iter_mut().find(|n| n.address == address) {
            node.record_success(latency);
        }
    }

    /// Record failure for a node by address.
    pub fn record_failure(&mut self, address: &str) {
        if let Some(node) = self.nodes.iter_mut().find(|n| n.address == address) {
            node.record_failure();
        }
    }

    /// Get all nodes sorted by health (healthy first, then by latency).
    pub fn sorted_by_health(&self) -> Vec<&BootstrapNode> {
        let mut sorted: Vec<&BootstrapNode> = self.nodes.iter().collect();
        sorted.sort_by(|a, b| {
            let health_ord = health_priority(a.health).cmp(&health_priority(b.health));
            if health_ord != std::cmp::Ordering::Equal {
                return health_ord;
            }
            a.latency.cmp(&b.latency)
        });
        sorted
    }

    /// Get all node addresses.
    pub fn addresses(&self) -> Vec<&str> {
        self.nodes.iter().map(|n| n.address.as_str()).collect()
    }

    pub fn len(&self) -> usize {
        self.nodes.len()
    }

    pub fn is_empty(&self) -> bool {
        self.nodes.is_empty()
    }

    /// Count of healthy nodes.
    pub fn healthy_count(&self) -> usize {
        self.nodes
            .iter()
            .filter(|n| n.health == BootstrapHealth::Healthy)
            .count()
    }
}

impl Default for BootstrapManager {
    fn default() -> Self {
        Self::new()
    }
}

fn health_priority(h: BootstrapHealth) -> u8 {
    match h {
        BootstrapHealth::Healthy => 0,
        BootstrapHealth::Unknown => 1,
        BootstrapHealth::Degraded => 2,
        BootstrapHealth::Unreachable => 3,
    }
}

/// Peer exchange (PEX) protocol state.
///
/// Tracks recently seen peers and enables sharing peer lists with
/// connected peers to accelerate network discovery beyond mDNS.
pub struct PeerExchange {
    /// Known peer addresses we can share with others.
    known_peers: HashMap<String, PexEntry>,
    /// Maximum peers to track.
    max_peers: usize,
    /// Maximum peers to send in a single exchange.
    max_exchange_size: usize,
    /// Peers we've recently exchanged with (cooldown tracking).
    exchange_cooldowns: HashMap<String, Instant>,
    /// Cooldown duration between exchanges with the same peer.
    cooldown_duration: Duration,
}

/// A peer entry in the exchange table.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PexEntry {
    pub peer_id: String,
    pub addresses: Vec<String>,
    pub last_seen: u64,
    /// How many times this peer was shared with us.
    pub share_count: u32,
}

impl PexEntry {
    pub fn new(peer_id: String, addresses: Vec<String>) -> Self {
        let last_seen = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        Self {
            peer_id,
            addresses,
            last_seen,
            share_count: 0,
        }
    }

    /// Update the last seen time and merge addresses.
    pub fn refresh(&mut self, addresses: &[String]) {
        self.last_seen = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        for addr in addresses {
            if !self.addresses.contains(addr) {
                self.addresses.push(addr.clone());
            }
        }
    }
}

impl PeerExchange {
    pub fn new(max_peers: usize, max_exchange_size: usize) -> Self {
        Self {
            known_peers: HashMap::new(),
            max_peers,
            max_exchange_size,
            exchange_cooldowns: HashMap::new(),
            cooldown_duration: Duration::from_secs(60),
        }
    }

    /// Add or update a known peer.
    pub fn add_peer(&mut self, peer_id: String, addresses: Vec<String>) {
        if let Some(entry) = self.known_peers.get_mut(&peer_id) {
            entry.refresh(&addresses);
        } else {
            if self.known_peers.len() >= self.max_peers {
                self.evict_oldest();
            }
            self.known_peers
                .insert(peer_id.clone(), PexEntry::new(peer_id, addresses));
        }
    }

    /// Remove a peer from the exchange table.
    pub fn remove_peer(&mut self, peer_id: &str) -> bool {
        self.known_peers.remove(peer_id).is_some()
    }

    /// Check if we can exchange with a peer (cooldown has expired).
    pub fn can_exchange_with(&self, peer_id: &str) -> bool {
        self.exchange_cooldowns
            .get(peer_id)
            .is_none_or(|last| last.elapsed() >= self.cooldown_duration)
    }

    /// Get peers to share with a remote peer.
    /// Excludes the requesting peer from the result.
    pub fn peers_for_exchange(&mut self, requesting_peer: &str) -> Vec<PexEntry> {
        self.exchange_cooldowns
            .insert(requesting_peer.to_string(), Instant::now());

        let mut entries: Vec<&PexEntry> = self
            .known_peers
            .values()
            .filter(|e| e.peer_id != requesting_peer)
            .collect();

        // Sort by most recently seen first.
        entries.sort_by(|a, b| b.last_seen.cmp(&a.last_seen));
        entries.truncate(self.max_exchange_size);

        entries.into_iter().cloned().collect()
    }

    /// Process peers received from a remote peer exchange.
    /// Returns the count of new peers learned.
    pub fn receive_exchange(&mut self, peers: Vec<PexEntry>) -> usize {
        let mut new_count = 0;
        for mut entry in peers {
            entry.share_count += 1;
            if self.known_peers.contains_key(&entry.peer_id) {
                if let Some(existing) = self.known_peers.get_mut(&entry.peer_id) {
                    existing.refresh(&entry.addresses);
                }
            } else {
                if self.known_peers.len() >= self.max_peers {
                    self.evict_oldest();
                }
                self.known_peers.insert(entry.peer_id.clone(), entry);
                new_count += 1;
            }
        }
        new_count
    }

    pub fn peer_count(&self) -> usize {
        self.known_peers.len()
    }

    pub fn get_peer(&self, peer_id: &str) -> Option<&PexEntry> {
        self.known_peers.get(peer_id)
    }

    /// Garbage collect stale peers older than the given max age.
    pub fn gc_stale(&mut self, max_age: Duration) {
        let cutoff = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs()
            .saturating_sub(max_age.as_secs());

        self.known_peers.retain(|_, entry| entry.last_seen >= cutoff);

        // Also clean up expired cooldowns.
        self.exchange_cooldowns
            .retain(|_, last| last.elapsed() < self.cooldown_duration * 2);
    }

    fn evict_oldest(&mut self) {
        if let Some(oldest_key) = self
            .known_peers
            .iter()
            .min_by_key(|(_, e)| e.last_seen)
            .map(|(k, _)| k.clone())
        {
            self.known_peers.remove(&oldest_key);
        }
    }
}

/// Rendezvous point for namespace-based peer discovery.
///
/// Peers register under namespaces (e.g., "nous/governance/dao-42")
/// and can discover other peers in the same namespace.
pub struct RendezvousRegistry {
    /// Namespace → set of registered peer IDs.
    namespaces: HashMap<String, HashSet<String>>,
    /// Peer ID → set of namespaces they're registered in.
    peer_namespaces: HashMap<String, HashSet<String>>,
    /// Registration timestamps for TTL-based expiry.
    registrations: HashMap<(String, String), Instant>,
    /// TTL for registrations.
    ttl: Duration,
    /// Max registrations per peer.
    max_per_peer: usize,
    /// Max registrations per namespace.
    max_per_namespace: usize,
}

impl RendezvousRegistry {
    pub fn new(ttl: Duration, max_per_peer: usize, max_per_namespace: usize) -> Self {
        Self {
            namespaces: HashMap::new(),
            peer_namespaces: HashMap::new(),
            registrations: HashMap::new(),
            ttl,
            max_per_peer,
            max_per_namespace,
        }
    }

    /// Register a peer under a namespace.
    pub fn register(&mut self, peer_id: &str, namespace: &str) -> Result<(), &'static str> {
        // Check per-peer limit.
        let peer_ns = self
            .peer_namespaces
            .entry(peer_id.to_string())
            .or_default();
        if peer_ns.len() >= self.max_per_peer && !peer_ns.contains(namespace) {
            return Err("peer has reached max namespace registrations");
        }

        // Check per-namespace limit.
        let ns_peers = self
            .namespaces
            .entry(namespace.to_string())
            .or_default();
        if ns_peers.len() >= self.max_per_namespace && !ns_peers.contains(peer_id) {
            return Err("namespace has reached max registrations");
        }

        ns_peers.insert(peer_id.to_string());
        peer_ns.insert(namespace.to_string());
        self.registrations
            .insert((namespace.to_string(), peer_id.to_string()), Instant::now());

        Ok(())
    }

    /// Unregister a peer from a namespace.
    pub fn unregister(&mut self, peer_id: &str, namespace: &str) -> bool {
        let removed = self
            .namespaces
            .get_mut(namespace)
            .is_some_and(|peers| peers.remove(peer_id));

        if removed {
            if let Some(ns) = self.peer_namespaces.get_mut(peer_id) {
                ns.remove(namespace);
            }
            self.registrations
                .remove(&(namespace.to_string(), peer_id.to_string()));
        }

        removed
    }

    /// Unregister a peer from all namespaces.
    pub fn unregister_all(&mut self, peer_id: &str) {
        if let Some(namespaces) = self.peer_namespaces.remove(peer_id) {
            for ns in &namespaces {
                if let Some(peers) = self.namespaces.get_mut(ns) {
                    peers.remove(peer_id);
                }
                self.registrations
                    .remove(&(ns.clone(), peer_id.to_string()));
            }
        }
    }

    /// Discover peers registered under a namespace.
    pub fn discover(&self, namespace: &str) -> Vec<String> {
        self.namespaces
            .get(namespace)
            .map(|peers| {
                peers
                    .iter()
                    .filter(|peer_id| {
                        // Only return non-expired registrations.
                        self.registrations
                            .get(&(namespace.to_string(), peer_id.to_string()))
                            .is_some_and(|registered_at| registered_at.elapsed() < self.ttl)
                    })
                    .cloned()
                    .collect()
            })
            .unwrap_or_default()
    }

    /// Get all namespaces a peer is registered in.
    pub fn peer_namespaces(&self, peer_id: &str) -> Vec<String> {
        self.peer_namespaces
            .get(peer_id)
            .map(|ns| ns.iter().cloned().collect())
            .unwrap_or_default()
    }

    /// List all namespaces with at least one registered peer.
    pub fn active_namespaces(&self) -> Vec<String> {
        self.namespaces
            .iter()
            .filter(|(_, peers)| !peers.is_empty())
            .map(|(ns, _)| ns.clone())
            .collect()
    }

    /// Clean up expired registrations.
    pub fn gc_expired(&mut self) {
        let expired: Vec<(String, String)> = self
            .registrations
            .iter()
            .filter(|(_, registered_at)| registered_at.elapsed() >= self.ttl)
            .map(|(key, _)| key.clone())
            .collect();

        for (namespace, peer_id) in expired {
            self.unregister(&peer_id, &namespace);
        }
    }

    pub fn namespace_count(&self) -> usize {
        self.namespaces.iter().filter(|(_, p)| !p.is_empty()).count()
    }

    pub fn total_registrations(&self) -> usize {
        self.registrations.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // --- BootstrapNode tests ---

    #[test]
    fn bootstrap_node_initial_state() {
        let node = BootstrapNode::new("/ip4/1.2.3.4/tcp/9000".into());
        assert_eq!(node.health, BootstrapHealth::Unknown);
        assert_eq!(node.successes, 0);
        assert_eq!(node.failures, 0);
        assert!(node.latency.is_none());
        assert_eq!(node.success_rate(), 0.0);
    }

    #[test]
    fn bootstrap_node_records_success() {
        let mut node = BootstrapNode::new("/ip4/1.2.3.4/tcp/9000".into());
        node.record_success(Duration::from_millis(50));
        assert_eq!(node.successes, 1);
        assert_eq!(node.latency, Some(Duration::from_millis(50)));
        assert_eq!(node.health, BootstrapHealth::Healthy);
        assert_eq!(node.success_rate(), 1.0);
    }

    #[test]
    fn bootstrap_node_records_failure() {
        let mut node = BootstrapNode::new("/ip4/1.2.3.4/tcp/9000".into());
        node.record_failure();
        node.record_failure();
        assert_eq!(node.failures, 2);
        assert_eq!(node.health, BootstrapHealth::Unreachable);
        assert_eq!(node.success_rate(), 0.0);
    }

    #[test]
    fn bootstrap_node_mixed_health() {
        let mut node = BootstrapNode::new("/ip4/1.2.3.4/tcp/9000".into());
        node.record_success(Duration::from_millis(30));
        node.record_failure();
        // 1 success, 1 failure = 50% rate, but recent success within 5 min
        assert_eq!(node.health, BootstrapHealth::Degraded);
    }

    // --- BootstrapManager tests ---

    #[test]
    fn bootstrap_manager_add_remove() {
        let mut mgr = BootstrapManager::new();
        mgr.add("/ip4/1.2.3.4/tcp/9000".into());
        mgr.add("/ip4/5.6.7.8/tcp/9000".into());
        assert_eq!(mgr.len(), 2);

        // Duplicate add is no-op.
        mgr.add("/ip4/1.2.3.4/tcp/9000".into());
        assert_eq!(mgr.len(), 2);

        assert!(mgr.remove("/ip4/1.2.3.4/tcp/9000"));
        assert_eq!(mgr.len(), 1);
        assert!(!mgr.remove("nonexistent"));
    }

    #[test]
    fn bootstrap_manager_next_healthy() {
        let mut mgr = BootstrapManager::new();
        mgr.add("/ip4/1.1.1.1/tcp/9000".into());
        mgr.add("/ip4/2.2.2.2/tcp/9000".into());

        // Both Unknown → either is fine.
        let first = mgr.next_healthy().unwrap().address.clone();
        assert!(
            first == "/ip4/1.1.1.1/tcp/9000" || first == "/ip4/2.2.2.2/tcp/9000"
        );

        // Mark first as unhealthy.
        mgr.record_failure("/ip4/1.1.1.1/tcp/9000");
        mgr.record_failure("/ip4/1.1.1.1/tcp/9000");
        mgr.record_failure("/ip4/1.1.1.1/tcp/9000");

        // Mark second as healthy.
        mgr.record_success("/ip4/2.2.2.2/tcp/9000", Duration::from_millis(20));

        // Should prefer the healthy one.
        let next = mgr.next_healthy().unwrap();
        assert_eq!(next.address, "/ip4/2.2.2.2/tcp/9000");
    }

    #[test]
    fn bootstrap_manager_sorted_by_health() {
        let mut mgr = BootstrapManager::new();
        mgr.add("a".into());
        mgr.add("b".into());
        mgr.add("c".into());

        mgr.record_success("b", Duration::from_millis(10));
        mgr.record_failure("c");
        mgr.record_failure("c");
        mgr.record_failure("c");

        let sorted = mgr.sorted_by_health();
        assert_eq!(sorted[0].address, "b"); // Healthy
        assert_eq!(sorted[1].address, "a"); // Unknown
        assert_eq!(sorted[2].address, "c"); // Unreachable
    }

    #[test]
    fn bootstrap_manager_empty() {
        let mut mgr = BootstrapManager::new();
        assert!(mgr.is_empty());
        assert!(mgr.next_healthy().is_none());
    }

    // --- PeerExchange tests ---

    #[test]
    fn pex_add_and_get_peer() {
        let mut pex = PeerExchange::new(100, 10);
        pex.add_peer("peer-a".into(), vec!["/ip4/1.1.1.1/tcp/9000".into()]);
        assert_eq!(pex.peer_count(), 1);
        assert!(pex.get_peer("peer-a").is_some());
        assert!(pex.get_peer("peer-b").is_none());
    }

    #[test]
    fn pex_refresh_existing_peer() {
        let mut pex = PeerExchange::new(100, 10);
        pex.add_peer("peer-a".into(), vec!["/ip4/1.1.1.1/tcp/9000".into()]);
        pex.add_peer(
            "peer-a".into(),
            vec!["/ip4/2.2.2.2/tcp/9000".into()],
        );

        // Should not duplicate, but should add new address.
        assert_eq!(pex.peer_count(), 1);
        let entry = pex.get_peer("peer-a").unwrap();
        assert_eq!(entry.addresses.len(), 2);
    }

    #[test]
    fn pex_remove_peer() {
        let mut pex = PeerExchange::new(100, 10);
        pex.add_peer("peer-a".into(), vec![]);
        assert!(pex.remove_peer("peer-a"));
        assert!(!pex.remove_peer("peer-a"));
        assert_eq!(pex.peer_count(), 0);
    }

    #[test]
    fn pex_evicts_oldest_when_full() {
        let mut pex = PeerExchange::new(2, 10);
        pex.add_peer("peer-a".into(), vec![]);
        pex.add_peer("peer-b".into(), vec![]);
        assert_eq!(pex.peer_count(), 2);

        pex.add_peer("peer-c".into(), vec![]); // Should evict one of the existing.
        assert_eq!(pex.peer_count(), 2);
        // peer-c must exist since it was just added.
        assert!(pex.get_peer("peer-c").is_some());
    }

    #[test]
    fn pex_exchange_excludes_requester() {
        let mut pex = PeerExchange::new(100, 10);
        pex.add_peer("peer-a".into(), vec!["/ip4/1.1.1.1".into()]);
        pex.add_peer("peer-b".into(), vec!["/ip4/2.2.2.2".into()]);
        pex.add_peer("peer-c".into(), vec!["/ip4/3.3.3.3".into()]);

        let exchange = pex.peers_for_exchange("peer-a");
        assert_eq!(exchange.len(), 2);
        assert!(exchange.iter().all(|e| e.peer_id != "peer-a"));
    }

    #[test]
    fn pex_exchange_respects_max_size() {
        let mut pex = PeerExchange::new(100, 2); // Max 2 per exchange.
        for i in 0..10 {
            pex.add_peer(format!("peer-{i}"), vec![]);
        }

        let exchange = pex.peers_for_exchange("requester");
        assert!(exchange.len() <= 2);
    }

    #[test]
    fn pex_receive_exchange() {
        let mut pex = PeerExchange::new(100, 10);
        pex.add_peer("existing".into(), vec![]);

        let received = vec![
            PexEntry::new("new-peer".into(), vec!["/ip4/1.1.1.1".into()]),
            PexEntry::new("existing".into(), vec!["/ip4/2.2.2.2".into()]),
        ];

        let new_count = pex.receive_exchange(received);
        assert_eq!(new_count, 1); // Only "new-peer" is new.
        assert_eq!(pex.peer_count(), 2);

        // Existing peer should have merged addresses.
        let existing = pex.get_peer("existing").unwrap();
        assert!(existing.addresses.contains(&"/ip4/2.2.2.2".to_string()));
    }

    #[test]
    fn pex_cooldown() {
        let mut pex = PeerExchange::new(100, 10);
        pex.cooldown_duration = Duration::from_millis(50);
        pex.add_peer("peer-a".into(), vec![]);

        assert!(pex.can_exchange_with("peer-b"));
        pex.peers_for_exchange("peer-b"); // Sets cooldown.
        assert!(!pex.can_exchange_with("peer-b")); // On cooldown.

        // Wait for cooldown to expire.
        std::thread::sleep(Duration::from_millis(60));
        assert!(pex.can_exchange_with("peer-b"));
    }

    // --- RendezvousRegistry tests ---

    #[test]
    fn rendezvous_register_discover() {
        let mut reg = RendezvousRegistry::new(Duration::from_secs(300), 10, 100);
        reg.register("peer-a", "nous/governance").unwrap();
        reg.register("peer-b", "nous/governance").unwrap();
        reg.register("peer-a", "nous/messaging").unwrap();

        let governance_peers = reg.discover("nous/governance");
        assert_eq!(governance_peers.len(), 2);
        assert!(governance_peers.contains(&"peer-a".to_string()));
        assert!(governance_peers.contains(&"peer-b".to_string()));

        let messaging_peers = reg.discover("nous/messaging");
        assert_eq!(messaging_peers.len(), 1);
        assert!(messaging_peers.contains(&"peer-a".to_string()));
    }

    #[test]
    fn rendezvous_unregister() {
        let mut reg = RendezvousRegistry::new(Duration::from_secs(300), 10, 100);
        reg.register("peer-a", "ns1").unwrap();
        assert!(reg.unregister("peer-a", "ns1"));
        assert!(!reg.unregister("peer-a", "ns1")); // Already removed.
        assert!(reg.discover("ns1").is_empty());
    }

    #[test]
    fn rendezvous_unregister_all() {
        let mut reg = RendezvousRegistry::new(Duration::from_secs(300), 10, 100);
        reg.register("peer-a", "ns1").unwrap();
        reg.register("peer-a", "ns2").unwrap();
        reg.register("peer-a", "ns3").unwrap();

        reg.unregister_all("peer-a");
        assert!(reg.discover("ns1").is_empty());
        assert!(reg.discover("ns2").is_empty());
        assert!(reg.discover("ns3").is_empty());
        assert!(reg.peer_namespaces("peer-a").is_empty());
    }

    #[test]
    fn rendezvous_per_peer_limit() {
        let mut reg = RendezvousRegistry::new(Duration::from_secs(300), 2, 100);
        reg.register("peer-a", "ns1").unwrap();
        reg.register("peer-a", "ns2").unwrap();
        let result = reg.register("peer-a", "ns3");
        assert!(result.is_err());
    }

    #[test]
    fn rendezvous_per_namespace_limit() {
        let mut reg = RendezvousRegistry::new(Duration::from_secs(300), 10, 2);
        reg.register("peer-a", "ns1").unwrap();
        reg.register("peer-b", "ns1").unwrap();
        let result = reg.register("peer-c", "ns1");
        assert!(result.is_err());
    }

    #[test]
    fn rendezvous_re_register_within_limits() {
        let mut reg = RendezvousRegistry::new(Duration::from_secs(300), 1, 1);
        reg.register("peer-a", "ns1").unwrap();
        // Re-registering the same peer/namespace should succeed.
        reg.register("peer-a", "ns1").unwrap();
    }

    #[test]
    fn rendezvous_active_namespaces() {
        let mut reg = RendezvousRegistry::new(Duration::from_secs(300), 10, 100);
        reg.register("peer-a", "ns1").unwrap();
        reg.register("peer-b", "ns2").unwrap();
        let active = reg.active_namespaces();
        assert_eq!(active.len(), 2);
    }

    #[test]
    fn rendezvous_peer_namespaces() {
        let mut reg = RendezvousRegistry::new(Duration::from_secs(300), 10, 100);
        reg.register("peer-a", "ns1").unwrap();
        reg.register("peer-a", "ns2").unwrap();

        let ns = reg.peer_namespaces("peer-a");
        assert_eq!(ns.len(), 2);
    }

    #[test]
    fn rendezvous_gc_expired() {
        let mut reg = RendezvousRegistry::new(Duration::from_millis(50), 10, 100);
        reg.register("peer-a", "ns1").unwrap();
        assert_eq!(reg.discover("ns1").len(), 1);

        std::thread::sleep(Duration::from_millis(60));
        reg.gc_expired();
        assert!(reg.discover("ns1").is_empty());
    }

    #[test]
    fn rendezvous_discover_unknown_namespace() {
        let reg = RendezvousRegistry::new(Duration::from_secs(300), 10, 100);
        assert!(reg.discover("nonexistent").is_empty());
    }

    #[test]
    fn rendezvous_total_registrations() {
        let mut reg = RendezvousRegistry::new(Duration::from_secs(300), 10, 100);
        reg.register("peer-a", "ns1").unwrap();
        reg.register("peer-a", "ns2").unwrap();
        reg.register("peer-b", "ns1").unwrap();
        assert_eq!(reg.total_registrations(), 3);
    }
}
