use libp2p::PeerId;
use std::collections::HashMap;
use std::time::{Duration, Instant};

/// Token-bucket rate limiter keyed by peer.
///
/// Each peer gets `burst` tokens. Tokens refill at `refill_rate` tokens per
/// `refill_interval`. A message is allowed only if the peer has at least one
/// token remaining.
pub struct RateLimiter {
    burst: u32,
    refill_rate: u32,
    refill_interval: Duration,
    peers: HashMap<PeerId, PeerBucket>,
}

struct PeerBucket {
    tokens: u32,
    last_refill: Instant,
}

impl RateLimiter {
    /// Create a rate limiter.
    ///
    /// * `burst` — maximum tokens a peer can accumulate.
    /// * `refill_rate` — tokens added per refill interval.
    /// * `refill_interval` — how often tokens refill.
    pub fn new(burst: u32, refill_rate: u32, refill_interval: Duration) -> Self {
        Self {
            burst,
            refill_rate,
            refill_interval,
            peers: HashMap::new(),
        }
    }

    /// Default production limiter: 60 messages/minute burst, 10/sec refill.
    pub fn default_production() -> Self {
        Self::new(60, 10, Duration::from_secs(1))
    }

    /// Returns `true` if the peer is allowed to send, consuming one token.
    pub fn check(&mut self, peer: &PeerId) -> bool {
        self.check_at(peer, Instant::now())
    }

    /// Testable version with explicit timestamp.
    fn check_at(&mut self, peer: &PeerId, now: Instant) -> bool {
        let burst = self.burst;
        let refill_rate = self.refill_rate;
        let refill_interval = self.refill_interval;

        let bucket = self.peers.entry(*peer).or_insert(PeerBucket {
            tokens: burst,
            last_refill: now,
        });

        // Refill tokens based on elapsed time.
        let elapsed = now.duration_since(bucket.last_refill);
        if elapsed >= refill_interval {
            let intervals = (elapsed.as_nanos() / refill_interval.as_nanos()) as u32;
            let refilled = intervals.saturating_mul(refill_rate);
            bucket.tokens = (bucket.tokens + refilled).min(burst);
            bucket.last_refill = now;
        }

        if bucket.tokens > 0 {
            bucket.tokens -= 1;
            true
        } else {
            false
        }
    }

    /// Remove state for a disconnected peer.
    pub fn remove_peer(&mut self, peer: &PeerId) {
        self.peers.remove(peer);
    }

    /// How many tokens a peer has remaining.
    pub fn remaining(&self, peer: &PeerId) -> u32 {
        self.peers.get(peer).map(|b| b.tokens).unwrap_or(self.burst)
    }

    /// Number of tracked peers.
    pub fn tracked_peers(&self) -> usize {
        self.peers.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_peer() -> PeerId {
        PeerId::random()
    }

    #[test]
    fn new_peer_gets_full_burst() {
        let limiter = RateLimiter::new(10, 1, Duration::from_secs(1));
        let peer = test_peer();
        assert_eq!(limiter.remaining(&peer), 10);
    }

    #[test]
    fn check_consumes_token() {
        let mut limiter = RateLimiter::new(5, 1, Duration::from_secs(1));
        let peer = test_peer();
        let now = Instant::now();

        assert!(limiter.check_at(&peer, now));
        assert_eq!(limiter.remaining(&peer), 4);
    }

    #[test]
    fn exhausted_tokens_rejects() {
        let mut limiter = RateLimiter::new(3, 1, Duration::from_secs(1));
        let peer = test_peer();
        let now = Instant::now();

        assert!(limiter.check_at(&peer, now));
        assert!(limiter.check_at(&peer, now));
        assert!(limiter.check_at(&peer, now));
        assert!(!limiter.check_at(&peer, now)); // depleted
    }

    #[test]
    fn tokens_refill_after_interval() {
        let mut limiter = RateLimiter::new(3, 2, Duration::from_secs(1));
        let peer = test_peer();
        let now = Instant::now();

        // Exhaust all tokens.
        limiter.check_at(&peer, now);
        limiter.check_at(&peer, now);
        limiter.check_at(&peer, now);
        assert!(!limiter.check_at(&peer, now));

        // After 1 interval, 2 tokens refill.
        let later = now + Duration::from_secs(1);
        assert!(limiter.check_at(&peer, later));
        assert!(limiter.check_at(&peer, later));
        assert!(!limiter.check_at(&peer, later));
    }

    #[test]
    fn refill_caps_at_burst() {
        let mut limiter = RateLimiter::new(5, 100, Duration::from_secs(1));
        let peer = test_peer();
        let now = Instant::now();

        limiter.check_at(&peer, now); // 4 remaining
        let later = now + Duration::from_secs(10); // way past refill
        limiter.check_at(&peer, later);
        assert_eq!(limiter.remaining(&peer), 4); // capped at burst(5) - 1
    }

    #[test]
    fn independent_peer_buckets() {
        let mut limiter = RateLimiter::new(2, 1, Duration::from_secs(1));
        let peer_a = test_peer();
        let peer_b = test_peer();
        let now = Instant::now();

        limiter.check_at(&peer_a, now);
        limiter.check_at(&peer_a, now);
        assert!(!limiter.check_at(&peer_a, now)); // A exhausted

        assert!(limiter.check_at(&peer_b, now)); // B unaffected
    }

    #[test]
    fn remove_peer_resets_state() {
        let mut limiter = RateLimiter::new(3, 1, Duration::from_secs(1));
        let peer = test_peer();
        let now = Instant::now();

        limiter.check_at(&peer, now);
        limiter.check_at(&peer, now);
        assert_eq!(limiter.remaining(&peer), 1);

        limiter.remove_peer(&peer);
        assert_eq!(limiter.remaining(&peer), 3); // reset to burst
    }

    #[test]
    fn tracked_peers_count() {
        let mut limiter = RateLimiter::new(5, 1, Duration::from_secs(1));
        assert_eq!(limiter.tracked_peers(), 0);

        let p1 = test_peer();
        let p2 = test_peer();
        limiter.check(&p1);
        limiter.check(&p2);
        assert_eq!(limiter.tracked_peers(), 2);

        limiter.remove_peer(&p1);
        assert_eq!(limiter.tracked_peers(), 1);
    }

    #[test]
    fn multiple_refill_intervals_accumulate() {
        let mut limiter = RateLimiter::new(10, 1, Duration::from_secs(1));
        let peer = test_peer();
        let now = Instant::now();

        // Consume 5 tokens.
        for _ in 0..5 {
            limiter.check_at(&peer, now);
        }
        assert_eq!(limiter.remaining(&peer), 5);

        // After 3 intervals, 3 tokens refill.
        let later = now + Duration::from_secs(3);
        limiter.check_at(&peer, later);
        assert_eq!(limiter.remaining(&peer), 7); // 5 + 3 - 1
    }

    #[test]
    fn default_production_has_reasonable_values() {
        let limiter = RateLimiter::default_production();
        let peer = test_peer();
        assert_eq!(limiter.remaining(&peer), 60);
    }
}
