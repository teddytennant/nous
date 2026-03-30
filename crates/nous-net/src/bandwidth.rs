use serde::{Deserialize, Serialize};
use std::collections::{BinaryHeap, HashMap, VecDeque};
use std::time::{Duration, Instant};

/// Priority levels for outbound messages.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum Priority {
    /// Background sync, anti-entropy, bulk data.
    Low = 0,
    /// Regular messages, social feed updates.
    Normal = 1,
    /// Direct messages, governance votes, payment confirmations.
    High = 2,
    /// Identity operations, security alerts, connection control.
    Critical = 3,
}

impl Priority {
    pub fn all() -> &'static [Priority] {
        &[
            Priority::Low,
            Priority::Normal,
            Priority::High,
            Priority::Critical,
        ]
    }
}

/// A queued outbound message with priority.
#[derive(Debug, Clone)]
pub struct QueuedMessage {
    pub id: u64,
    pub priority: Priority,
    pub peer_id: String,
    pub data: Vec<u8>,
    pub enqueued_at: Instant,
}

impl PartialEq for QueuedMessage {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
    }
}

impl Eq for QueuedMessage {}

impl PartialOrd for QueuedMessage {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for QueuedMessage {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        // Higher priority first, then FIFO (lower id = earlier).
        self.priority
            .cmp(&other.priority)
            .then(other.id.cmp(&self.id))
    }
}

/// Priority-based message queue with backpressure support.
pub struct PriorityQueue {
    heap: BinaryHeap<QueuedMessage>,
    next_id: u64,
    max_size: usize,
    /// Total bytes currently queued.
    queued_bytes: u64,
    /// Maximum bytes before backpressure activates.
    max_bytes: u64,
}

impl PriorityQueue {
    pub fn new(max_size: usize, max_bytes: u64) -> Self {
        Self {
            heap: BinaryHeap::new(),
            next_id: 0,
            max_size,
            queued_bytes: 0,
            max_bytes,
        }
    }

    /// Enqueue a message. Returns the message ID, or error if backpressure
    /// prevents enqueueing (only Critical messages bypass backpressure).
    pub fn enqueue(
        &mut self,
        priority: Priority,
        peer_id: String,
        data: Vec<u8>,
    ) -> Result<u64, BackpressureError> {
        // Backpressure check: reject non-critical if over limits.
        if priority != Priority::Critical {
            if self.heap.len() >= self.max_size {
                return Err(BackpressureError::QueueFull);
            }
            if self.queued_bytes + data.len() as u64 > self.max_bytes {
                return Err(BackpressureError::ByteLimitExceeded);
            }
        }

        let id = self.next_id;
        self.next_id += 1;
        self.queued_bytes += data.len() as u64;

        self.heap.push(QueuedMessage {
            id,
            priority,
            peer_id,
            data,
            enqueued_at: Instant::now(),
        });

        Ok(id)
    }

    /// Dequeue the highest-priority message.
    pub fn dequeue(&mut self) -> Option<QueuedMessage> {
        let msg = self.heap.pop()?;
        self.queued_bytes = self.queued_bytes.saturating_sub(msg.data.len() as u64);
        Some(msg)
    }

    /// Peek at the highest-priority message without removing it.
    pub fn peek(&self) -> Option<&QueuedMessage> {
        self.heap.peek()
    }

    /// Whether backpressure is active (queue near capacity).
    pub fn is_under_pressure(&self) -> bool {
        self.heap.len() >= self.max_size * 80 / 100
            || self.queued_bytes >= self.max_bytes * 80 / 100
    }

    pub fn len(&self) -> usize {
        self.heap.len()
    }

    pub fn is_empty(&self) -> bool {
        self.heap.is_empty()
    }

    pub fn queued_bytes(&self) -> u64 {
        self.queued_bytes
    }

    /// Drop all messages below a given priority (used during congestion).
    pub fn drop_below(&mut self, min_priority: Priority) -> usize {
        let before = self.heap.len();
        let retained: Vec<QueuedMessage> = self
            .heap
            .drain()
            .filter(|msg| msg.priority >= min_priority)
            .collect();

        self.queued_bytes = retained.iter().map(|m| m.data.len() as u64).sum();
        self.heap = retained.into_iter().collect();
        before - self.heap.len()
    }
}

/// Backpressure error when enqueue is rejected.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BackpressureError {
    QueueFull,
    ByteLimitExceeded,
}

impl std::fmt::Display for BackpressureError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            BackpressureError::QueueFull => write!(f, "message queue is full"),
            BackpressureError::ByteLimitExceeded => write!(f, "byte limit exceeded"),
        }
    }
}

impl std::error::Error for BackpressureError {}

/// Sliding window for tracking bytes over time.
#[derive(Debug)]
struct SlidingWindow {
    /// (timestamp, bytes) entries.
    entries: VecDeque<(Instant, u64)>,
    window_duration: Duration,
}

impl SlidingWindow {
    fn new(window_duration: Duration) -> Self {
        Self {
            entries: VecDeque::new(),
            window_duration,
        }
    }

    fn record(&mut self, bytes: u64) {
        let now = Instant::now();
        self.entries.push_back((now, bytes));
        self.gc();
    }

    fn total_bytes(&mut self) -> u64 {
        self.gc();
        self.entries.iter().map(|(_, b)| b).sum()
    }

    fn bytes_per_second(&mut self) -> f64 {
        let total = self.total_bytes();
        let secs = self.window_duration.as_secs_f64();
        if secs == 0.0 {
            return 0.0;
        }
        total as f64 / secs
    }

    fn gc(&mut self) {
        let cutoff = Instant::now() - self.window_duration;
        while self.entries.front().is_some_and(|(t, _)| *t < cutoff) {
            self.entries.pop_front();
        }
    }
}

/// Per-peer bandwidth tracking.
#[derive(Debug)]
pub struct PeerBandwidth {
    pub peer_id: String,
    sent: SlidingWindow,
    received: SlidingWindow,
    /// Total bytes sent since tracking started.
    pub total_sent: u64,
    /// Total bytes received since tracking started.
    pub total_received: u64,
}

impl PeerBandwidth {
    fn new(peer_id: String, window: Duration) -> Self {
        Self {
            peer_id,
            sent: SlidingWindow::new(window),
            received: SlidingWindow::new(window),
            total_sent: 0,
            total_received: 0,
        }
    }

    /// Record bytes sent to this peer.
    pub fn record_sent(&mut self, bytes: u64) {
        self.sent.record(bytes);
        self.total_sent += bytes;
    }

    /// Record bytes received from this peer.
    pub fn record_received(&mut self, bytes: u64) {
        self.received.record(bytes);
        self.total_received += bytes;
    }

    /// Current send rate in bytes/second.
    pub fn send_rate(&mut self) -> f64 {
        self.sent.bytes_per_second()
    }

    /// Current receive rate in bytes/second.
    pub fn receive_rate(&mut self) -> f64 {
        self.received.bytes_per_second()
    }

    /// Total bandwidth in the current window.
    pub fn window_total(&mut self) -> u64 {
        self.sent.total_bytes() + self.received.total_bytes()
    }
}

/// Global bandwidth manager.
///
/// Tracks per-peer and aggregate bandwidth, enforces rate limits,
/// and provides backpressure signals.
pub struct BandwidthManager {
    peers: HashMap<String, PeerBandwidth>,
    /// Global sent/received tracking.
    global_sent: SlidingWindow,
    global_received: SlidingWindow,
    /// Per-peer rate limit in bytes/second.
    per_peer_limit: Option<u64>,
    /// Global rate limit in bytes/second.
    global_limit: Option<u64>,
    /// Measurement window duration.
    window: Duration,
    /// Total bytes sent since start.
    pub total_sent: u64,
    /// Total bytes received since start.
    pub total_received: u64,
}

impl BandwidthManager {
    pub fn new(window: Duration) -> Self {
        Self {
            peers: HashMap::new(),
            global_sent: SlidingWindow::new(window),
            global_received: SlidingWindow::new(window),
            per_peer_limit: None,
            global_limit: None,
            window,
            total_sent: 0,
            total_received: 0,
        }
    }

    /// Set a per-peer bandwidth limit in bytes/second.
    pub fn set_per_peer_limit(&mut self, bytes_per_sec: u64) {
        self.per_peer_limit = Some(bytes_per_sec);
    }

    /// Set a global bandwidth limit in bytes/second.
    pub fn set_global_limit(&mut self, bytes_per_sec: u64) {
        self.global_limit = Some(bytes_per_sec);
    }

    /// Record bytes sent to a peer.
    pub fn record_sent(&mut self, peer_id: &str, bytes: u64) {
        let peer = self
            .peers
            .entry(peer_id.to_string())
            .or_insert_with(|| PeerBandwidth::new(peer_id.to_string(), self.window));
        peer.record_sent(bytes);
        self.global_sent.record(bytes);
        self.total_sent += bytes;
    }

    /// Record bytes received from a peer.
    pub fn record_received(&mut self, peer_id: &str, bytes: u64) {
        let peer = self
            .peers
            .entry(peer_id.to_string())
            .or_insert_with(|| PeerBandwidth::new(peer_id.to_string(), self.window));
        peer.record_received(bytes);
        self.global_received.record(bytes);
        self.total_received += bytes;
    }

    /// Check if sending to a peer is allowed under rate limits.
    pub fn can_send(&mut self, peer_id: &str, bytes: u64) -> bool {
        // Check global limit.
        if let Some(limit) = self.global_limit {
            let current = self.global_sent.bytes_per_second();
            if current + bytes as f64 > limit as f64 {
                return false;
            }
        }

        // Check per-peer limit.
        if let Some(limit) = self.per_peer_limit
            && let Some(peer) = self.peers.get_mut(peer_id)
        {
            let current = peer.send_rate();
            if current + bytes as f64 > limit as f64 {
                return false;
            }
        }

        true
    }

    /// Get the global send rate in bytes/second.
    pub fn global_send_rate(&mut self) -> f64 {
        self.global_sent.bytes_per_second()
    }

    /// Get the global receive rate in bytes/second.
    pub fn global_receive_rate(&mut self) -> f64 {
        self.global_received.bytes_per_second()
    }

    /// Get per-peer bandwidth stats.
    pub fn peer_stats(&self, peer_id: &str) -> Option<&PeerBandwidth> {
        self.peers.get(peer_id)
    }

    /// Get the top N peers by bandwidth usage.
    pub fn top_peers_by_bandwidth(&mut self, n: usize) -> Vec<(&str, u64)> {
        let mut peers: Vec<(&str, u64)> = self
            .peers
            .iter_mut()
            .map(|(id, bw)| (id.as_str(), bw.window_total()))
            .collect();
        peers.sort_by(|a, b| b.1.cmp(&a.1));
        peers.truncate(n);
        peers
    }

    /// Remove tracking for a disconnected peer.
    pub fn remove_peer(&mut self, peer_id: &str) {
        self.peers.remove(peer_id);
    }

    pub fn tracked_peer_count(&self) -> usize {
        self.peers.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // --- Priority tests ---

    #[test]
    fn priority_ordering() {
        assert!(Priority::Critical > Priority::High);
        assert!(Priority::High > Priority::Normal);
        assert!(Priority::Normal > Priority::Low);
    }

    #[test]
    fn priority_all() {
        assert_eq!(Priority::all().len(), 4);
    }

    // --- PriorityQueue tests ---

    #[test]
    fn queue_dequeues_by_priority() {
        let mut q = PriorityQueue::new(100, 1_000_000);
        q.enqueue(Priority::Low, "p1".into(), b"low".to_vec())
            .unwrap();
        q.enqueue(Priority::Critical, "p2".into(), b"crit".to_vec())
            .unwrap();
        q.enqueue(Priority::Normal, "p3".into(), b"norm".to_vec())
            .unwrap();

        let first = q.dequeue().unwrap();
        assert_eq!(first.priority, Priority::Critical);

        let second = q.dequeue().unwrap();
        assert_eq!(second.priority, Priority::Normal);

        let third = q.dequeue().unwrap();
        assert_eq!(third.priority, Priority::Low);
    }

    #[test]
    fn queue_fifo_within_same_priority() {
        let mut q = PriorityQueue::new(100, 1_000_000);
        q.enqueue(Priority::Normal, "p1".into(), b"first".to_vec())
            .unwrap();
        q.enqueue(Priority::Normal, "p2".into(), b"second".to_vec())
            .unwrap();
        q.enqueue(Priority::Normal, "p3".into(), b"third".to_vec())
            .unwrap();

        assert_eq!(q.dequeue().unwrap().peer_id, "p1");
        assert_eq!(q.dequeue().unwrap().peer_id, "p2");
        assert_eq!(q.dequeue().unwrap().peer_id, "p3");
    }

    #[test]
    fn queue_backpressure_on_count() {
        let mut q = PriorityQueue::new(2, 1_000_000);
        q.enqueue(Priority::Normal, "p1".into(), b"a".to_vec())
            .unwrap();
        q.enqueue(Priority::Normal, "p2".into(), b"b".to_vec())
            .unwrap();

        // Third normal message rejected.
        let result = q.enqueue(Priority::Normal, "p3".into(), b"c".to_vec());
        assert_eq!(result, Err(BackpressureError::QueueFull));

        // But critical bypasses backpressure.
        let result = q.enqueue(Priority::Critical, "p4".into(), b"d".to_vec());
        assert!(result.is_ok());
    }

    #[test]
    fn queue_backpressure_on_bytes() {
        let mut q = PriorityQueue::new(1000, 10); // 10 byte limit.
        q.enqueue(Priority::Normal, "p1".into(), vec![0; 8])
            .unwrap();

        // Would exceed 10 bytes.
        let result = q.enqueue(Priority::Normal, "p2".into(), vec![0; 5]);
        assert_eq!(result, Err(BackpressureError::ByteLimitExceeded));
    }

    #[test]
    fn queue_tracks_bytes() {
        let mut q = PriorityQueue::new(100, 1_000_000);
        q.enqueue(Priority::Normal, "p1".into(), vec![0; 100])
            .unwrap();
        q.enqueue(Priority::Normal, "p2".into(), vec![0; 200])
            .unwrap();
        assert_eq!(q.queued_bytes(), 300);

        q.dequeue();
        assert_eq!(q.queued_bytes(), 200);
    }

    #[test]
    fn queue_drop_below_priority() {
        let mut q = PriorityQueue::new(100, 1_000_000);
        q.enqueue(Priority::Low, "p1".into(), b"low".to_vec())
            .unwrap();
        q.enqueue(Priority::Normal, "p2".into(), b"norm".to_vec())
            .unwrap();
        q.enqueue(Priority::High, "p3".into(), b"high".to_vec())
            .unwrap();
        q.enqueue(Priority::Critical, "p4".into(), b"crit".to_vec())
            .unwrap();

        let dropped = q.drop_below(Priority::High);
        assert_eq!(dropped, 2); // Low and Normal dropped.
        assert_eq!(q.len(), 2);
    }

    #[test]
    fn queue_is_under_pressure() {
        let mut q = PriorityQueue::new(10, 1_000_000);
        for i in 0..8 {
            q.enqueue(Priority::Normal, format!("p{i}"), b"x".to_vec())
                .unwrap();
        }
        assert!(q.is_under_pressure()); // 80% full.
    }

    #[test]
    fn queue_peek() {
        let mut q = PriorityQueue::new(100, 1_000_000);
        assert!(q.peek().is_none());

        q.enqueue(Priority::High, "p1".into(), b"data".to_vec())
            .unwrap();
        assert_eq!(q.peek().unwrap().priority, Priority::High);
        assert_eq!(q.len(), 1); // Peek doesn't remove.
    }

    // --- BandwidthManager tests ---

    #[test]
    fn bandwidth_records_sent_received() {
        let mut bw = BandwidthManager::new(Duration::from_secs(60));
        bw.record_sent("peer-a", 1000);
        bw.record_received("peer-a", 500);

        assert_eq!(bw.total_sent, 1000);
        assert_eq!(bw.total_received, 500);
        assert_eq!(bw.tracked_peer_count(), 1);
    }

    #[test]
    fn bandwidth_per_peer_stats() {
        let mut bw = BandwidthManager::new(Duration::from_secs(60));
        bw.record_sent("peer-a", 1000);
        bw.record_sent("peer-a", 2000);

        let stats = bw.peer_stats("peer-a").unwrap();
        assert_eq!(stats.total_sent, 3000);
    }

    #[test]
    fn bandwidth_global_limit_blocks() {
        let mut bw = BandwidthManager::new(Duration::from_secs(1));
        bw.set_global_limit(1000); // 1000 bytes/sec.

        // Record 900 bytes — should still allow small sends.
        bw.record_sent("peer-a", 900);

        // Trying to send 200 more would exceed limit.
        assert!(!bw.can_send("peer-a", 200));
    }

    #[test]
    fn bandwidth_per_peer_limit_blocks() {
        let mut bw = BandwidthManager::new(Duration::from_secs(1));
        bw.set_per_peer_limit(500);

        bw.record_sent("peer-a", 400);
        assert!(!bw.can_send("peer-a", 200)); // Would exceed 500.
        assert!(bw.can_send("peer-b", 200)); // Different peer, fine.
    }

    #[test]
    fn bandwidth_no_limit_allows_all() {
        let mut bw = BandwidthManager::new(Duration::from_secs(60));
        bw.record_sent("peer-a", 1_000_000);
        assert!(bw.can_send("peer-a", 1_000_000)); // No limits set.
    }

    #[test]
    fn bandwidth_remove_peer() {
        let mut bw = BandwidthManager::new(Duration::from_secs(60));
        bw.record_sent("peer-a", 100);
        assert_eq!(bw.tracked_peer_count(), 1);

        bw.remove_peer("peer-a");
        assert_eq!(bw.tracked_peer_count(), 0);
    }

    #[test]
    fn bandwidth_top_peers() {
        let mut bw = BandwidthManager::new(Duration::from_secs(60));
        bw.record_sent("peer-a", 100);
        bw.record_sent("peer-b", 500);
        bw.record_sent("peer-c", 200);

        let top = bw.top_peers_by_bandwidth(2);
        assert_eq!(top.len(), 2);
        assert_eq!(top[0].0, "peer-b"); // Highest bandwidth.
    }

    #[test]
    fn backpressure_error_display() {
        assert_eq!(
            BackpressureError::QueueFull.to_string(),
            "message queue is full"
        );
        assert_eq!(
            BackpressureError::ByteLimitExceeded.to_string(),
            "byte limit exceeded"
        );
    }
}
