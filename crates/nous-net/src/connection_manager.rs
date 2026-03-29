use libp2p::PeerId;
use std::collections::HashMap;
use std::time::Instant;

/// Direction of a connection.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Direction {
    Inbound,
    Outbound,
}

/// Metadata for an active connection.
#[derive(Debug, Clone)]
pub struct ConnectionInfo {
    pub peer_id: PeerId,
    pub direction: Direction,
    pub established_at: Instant,
    pub score: i64,
    pub bytes_sent: u64,
    pub bytes_received: u64,
}

/// Manages connection limits and provides eviction decisions.
///
/// Tracks active connections and enforces a configurable maximum. When the
/// limit is hit, the manager identifies the lowest-scored connection for
/// eviction.
pub struct ConnectionManager {
    max_connections: usize,
    connections: HashMap<PeerId, ConnectionInfo>,
    protected_peers: std::collections::HashSet<PeerId>,
}

impl ConnectionManager {
    pub fn new(max_connections: usize) -> Self {
        Self {
            max_connections,
            connections: HashMap::new(),
            protected_peers: std::collections::HashSet::new(),
        }
    }

    /// Whether a new connection can be accepted without eviction.
    pub fn can_accept(&self) -> bool {
        self.connections.len() < self.max_connections
    }

    /// Register a new connection. Returns `Err` with the peer to evict if at
    /// capacity and an eviction candidate exists.
    pub fn register(
        &mut self,
        peer_id: PeerId,
        direction: Direction,
    ) -> Result<(), Option<PeerId>> {
        if self.connections.contains_key(&peer_id) {
            // Already connected — update direction.
            if let Some(info) = self.connections.get_mut(&peer_id) {
                info.direction = direction;
            }
            return Ok(());
        }

        if self.connections.len() >= self.max_connections {
            if let Some(evict) = self.eviction_candidate() {
                return Err(Some(evict));
            }
            return Err(None);
        }

        self.connections.insert(
            peer_id,
            ConnectionInfo {
                peer_id,
                direction,
                established_at: Instant::now(),
                score: 0,
                bytes_sent: 0,
                bytes_received: 0,
            },
        );
        Ok(())
    }

    /// Mark a peer as protected from eviction.
    pub fn protect(&mut self, peer_id: PeerId) {
        self.protected_peers.insert(peer_id);
    }

    /// Remove protection from a peer.
    pub fn unprotect(&mut self, peer_id: &PeerId) {
        self.protected_peers.remove(peer_id);
    }

    /// Remove a connection.
    pub fn disconnect(&mut self, peer_id: &PeerId) -> Option<ConnectionInfo> {
        self.protected_peers.remove(peer_id);
        self.connections.remove(peer_id)
    }

    /// Adjust a connection's score.
    pub fn adjust_score(&mut self, peer_id: &PeerId, delta: i64) {
        if let Some(info) = self.connections.get_mut(peer_id) {
            info.score = info.score.saturating_add(delta);
        }
    }

    /// Record bytes transferred.
    pub fn record_traffic(&mut self, peer_id: &PeerId, sent: u64, received: u64) {
        if let Some(info) = self.connections.get_mut(peer_id) {
            info.bytes_sent += sent;
            info.bytes_received += received;
        }
    }

    /// Number of active connections.
    pub fn connected_count(&self) -> usize {
        self.connections.len()
    }

    /// Check if a peer is connected.
    pub fn is_connected(&self, peer_id: &PeerId) -> bool {
        self.connections.contains_key(peer_id)
    }

    /// Get connection info for a peer.
    pub fn get_info(&self, peer_id: &PeerId) -> Option<&ConnectionInfo> {
        self.connections.get(peer_id)
    }

    /// All connected peer IDs.
    pub fn connected_peers(&self) -> Vec<PeerId> {
        self.connections.keys().copied().collect()
    }

    /// Find the lowest-scored unprotected peer for eviction.
    fn eviction_candidate(&self) -> Option<PeerId> {
        self.connections
            .values()
            .filter(|info| !self.protected_peers.contains(&info.peer_id))
            .min_by_key(|info| info.score)
            .map(|info| info.peer_id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_peer() -> PeerId {
        PeerId::random()
    }

    #[test]
    fn new_manager_accepts_connections() {
        let mgr = ConnectionManager::new(10);
        assert!(mgr.can_accept());
        assert_eq!(mgr.connected_count(), 0);
    }

    #[test]
    fn register_adds_connection() {
        let mut mgr = ConnectionManager::new(10);
        let peer = test_peer();
        assert!(mgr.register(peer, Direction::Outbound).is_ok());
        assert_eq!(mgr.connected_count(), 1);
        assert!(mgr.is_connected(&peer));
    }

    #[test]
    fn register_duplicate_updates_direction() {
        let mut mgr = ConnectionManager::new(10);
        let peer = test_peer();
        mgr.register(peer, Direction::Inbound).unwrap();
        mgr.register(peer, Direction::Outbound).unwrap();
        assert_eq!(mgr.connected_count(), 1);
        assert_eq!(mgr.get_info(&peer).unwrap().direction, Direction::Outbound);
    }

    #[test]
    fn register_at_capacity_returns_eviction_candidate() {
        let mut mgr = ConnectionManager::new(2);
        let p1 = test_peer();
        let p2 = test_peer();
        let p3 = test_peer();

        mgr.register(p1, Direction::Outbound).unwrap();
        mgr.register(p2, Direction::Outbound).unwrap();
        mgr.adjust_score(&p1, -10); // p1 lowest score

        let result = mgr.register(p3, Direction::Inbound);
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), Some(p1));
    }

    #[test]
    fn protected_peers_not_evicted() {
        let mut mgr = ConnectionManager::new(2);
        let p1 = test_peer();
        let p2 = test_peer();
        let p3 = test_peer();

        mgr.register(p1, Direction::Outbound).unwrap();
        mgr.register(p2, Direction::Outbound).unwrap();
        mgr.adjust_score(&p1, -10);
        mgr.protect(p1); // protect the low-score peer

        let result = mgr.register(p3, Direction::Inbound);
        assert!(result.is_err());
        // Should suggest p2 instead of protected p1.
        assert_eq!(result.unwrap_err(), Some(p2));
    }

    #[test]
    fn all_protected_returns_none() {
        let mut mgr = ConnectionManager::new(1);
        let p1 = test_peer();
        let p2 = test_peer();

        mgr.register(p1, Direction::Outbound).unwrap();
        mgr.protect(p1);

        let result = mgr.register(p2, Direction::Inbound);
        assert_eq!(result.unwrap_err(), None);
    }

    #[test]
    fn disconnect_removes_connection() {
        let mut mgr = ConnectionManager::new(10);
        let peer = test_peer();
        mgr.register(peer, Direction::Outbound).unwrap();

        let info = mgr.disconnect(&peer);
        assert!(info.is_some());
        assert_eq!(mgr.connected_count(), 0);
        assert!(!mgr.is_connected(&peer));
    }

    #[test]
    fn disconnect_removes_protection() {
        let mut mgr = ConnectionManager::new(10);
        let peer = test_peer();
        mgr.register(peer, Direction::Outbound).unwrap();
        mgr.protect(peer);

        mgr.disconnect(&peer);
        // Internally, protected set should be cleaned up.
        assert!(!mgr.protected_peers.contains(&peer));
    }

    #[test]
    fn adjust_score_works() {
        let mut mgr = ConnectionManager::new(10);
        let peer = test_peer();
        mgr.register(peer, Direction::Outbound).unwrap();

        mgr.adjust_score(&peer, 5);
        assert_eq!(mgr.get_info(&peer).unwrap().score, 5);

        mgr.adjust_score(&peer, -3);
        assert_eq!(mgr.get_info(&peer).unwrap().score, 2);
    }

    #[test]
    fn record_traffic_accumulates() {
        let mut mgr = ConnectionManager::new(10);
        let peer = test_peer();
        mgr.register(peer, Direction::Outbound).unwrap();

        mgr.record_traffic(&peer, 100, 200);
        mgr.record_traffic(&peer, 50, 150);

        let info = mgr.get_info(&peer).unwrap();
        assert_eq!(info.bytes_sent, 150);
        assert_eq!(info.bytes_received, 350);
    }

    #[test]
    fn connected_peers_returns_all() {
        let mut mgr = ConnectionManager::new(10);
        let p1 = test_peer();
        let p2 = test_peer();

        mgr.register(p1, Direction::Outbound).unwrap();
        mgr.register(p2, Direction::Inbound).unwrap();

        let peers = mgr.connected_peers();
        assert_eq!(peers.len(), 2);
        assert!(peers.contains(&p1));
        assert!(peers.contains(&p2));
    }

    #[test]
    fn unprotect_allows_eviction() {
        let mut mgr = ConnectionManager::new(2);
        let p1 = test_peer();
        let p2 = test_peer();
        let p3 = test_peer();

        mgr.register(p1, Direction::Outbound).unwrap();
        mgr.register(p2, Direction::Outbound).unwrap();
        mgr.adjust_score(&p1, -10);
        mgr.protect(p1);

        // While protected, p2 would be evicted.
        let result = mgr.register(p3, Direction::Inbound);
        assert_eq!(result.unwrap_err(), Some(p2));

        // Unprotect p1.
        mgr.unprotect(&p1);
        let result = mgr.register(p3, Direction::Inbound);
        assert_eq!(result.unwrap_err(), Some(p1)); // now p1 is candidate
    }

    #[test]
    fn get_info_returns_none_for_unknown() {
        let mgr = ConnectionManager::new(10);
        assert!(mgr.get_info(&test_peer()).is_none());
    }
}
