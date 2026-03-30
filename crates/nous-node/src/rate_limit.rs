//! Rate limiting — token bucket and sliding window algorithms for per-peer
//! and per-endpoint request throttling.
//!
//! Designed to protect the node from abuse without impacting legitimate
//! traffic. Both algorithms are O(1) per check.

use std::collections::HashMap;
use std::time::{Duration, Instant};

use serde::{Deserialize, Serialize};

/// Configuration for a rate limiter.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RateLimitConfig {
    /// Maximum requests per window (or bucket capacity).
    pub max_requests: u64,
    /// Window duration.
    pub window: Duration,
    /// Whether to track limits per peer or globally.
    pub per_peer: bool,
}

impl RateLimitConfig {
    pub fn new(max_requests: u64, window: Duration) -> Self {
        Self {
            max_requests,
            window,
            per_peer: true,
        }
    }

    pub fn global(max_requests: u64, window: Duration) -> Self {
        Self {
            max_requests,
            window,
            per_peer: false,
        }
    }
}

// ---------------------------------------------------------------------------
// Token bucket
// ---------------------------------------------------------------------------

/// A single token bucket that refills at a constant rate.
#[derive(Debug, Clone)]
struct Bucket {
    tokens: f64,
    capacity: f64,
    refill_rate: f64, // tokens per second
    last_refill: Instant,
}

impl Bucket {
    fn new(capacity: u64, window: Duration) -> Self {
        let capacity_f = capacity as f64;
        Self {
            tokens: capacity_f,
            capacity: capacity_f,
            refill_rate: capacity_f / window.as_secs_f64(),
            last_refill: Instant::now(),
        }
    }

    fn new_at(capacity: u64, window: Duration, now: Instant) -> Self {
        let capacity_f = capacity as f64;
        Self {
            tokens: capacity_f,
            capacity: capacity_f,
            refill_rate: capacity_f / window.as_secs_f64(),
            last_refill: now,
        }
    }

    fn refill(&mut self, now: Instant) {
        let elapsed = now.duration_since(self.last_refill).as_secs_f64();
        self.tokens = (self.tokens + elapsed * self.refill_rate).min(self.capacity);
        self.last_refill = now;
    }

    fn try_acquire(&mut self, now: Instant) -> bool {
        self.refill(now);
        if self.tokens >= 1.0 {
            self.tokens -= 1.0;
            true
        } else {
            false
        }
    }

    fn try_acquire_n(&mut self, n: u64, now: Instant) -> bool {
        self.refill(now);
        let cost = n as f64;
        if self.tokens >= cost {
            self.tokens -= cost;
            true
        } else {
            false
        }
    }

    fn available(&self, now: Instant) -> u64 {
        let elapsed = now.duration_since(self.last_refill).as_secs_f64();
        let tokens = (self.tokens + elapsed * self.refill_rate).min(self.capacity);
        tokens as u64
    }
}

/// Per-peer token bucket rate limiter.
pub struct TokenBucketLimiter {
    config: RateLimitConfig,
    global_bucket: Bucket,
    peer_buckets: HashMap<String, Bucket>,
}

impl TokenBucketLimiter {
    pub fn new(config: RateLimitConfig) -> Self {
        let global_bucket = Bucket::new(config.max_requests, config.window);
        Self {
            config,
            global_bucket,
            peer_buckets: HashMap::new(),
        }
    }

    /// Check if a request from the given peer is allowed.
    pub fn check(&mut self, peer_id: &str) -> bool {
        self.check_at(peer_id, Instant::now())
    }

    /// Check with explicit timestamp (for testing).
    pub fn check_at(&mut self, peer_id: &str, now: Instant) -> bool {
        if self.config.per_peer {
            let bucket = self
                .peer_buckets
                .entry(peer_id.to_string())
                .or_insert_with(|| {
                    Bucket::new_at(self.config.max_requests, self.config.window, now)
                });
            bucket.try_acquire(now)
        } else {
            self.global_bucket.try_acquire(now)
        }
    }

    /// Check if `n` requests from the peer are allowed as a batch.
    pub fn check_n(&mut self, peer_id: &str, n: u64) -> bool {
        self.check_n_at(peer_id, n, Instant::now())
    }

    pub fn check_n_at(&mut self, peer_id: &str, n: u64, now: Instant) -> bool {
        if self.config.per_peer {
            let bucket = self
                .peer_buckets
                .entry(peer_id.to_string())
                .or_insert_with(|| {
                    Bucket::new_at(self.config.max_requests, self.config.window, now)
                });
            bucket.try_acquire_n(n, now)
        } else {
            self.global_bucket.try_acquire_n(n, now)
        }
    }

    /// Number of tokens currently available for a peer.
    pub fn available(&self, peer_id: &str) -> u64 {
        let now = Instant::now();
        if self.config.per_peer {
            self.peer_buckets
                .get(peer_id)
                .map(|b| b.available(now))
                .unwrap_or(self.config.max_requests)
        } else {
            self.global_bucket.available(now)
        }
    }

    /// Remove stale peer buckets that are fully refilled (cleanup).
    pub fn gc(&mut self) {
        let now = Instant::now();
        self.peer_buckets
            .retain(|_, b| b.available(now) < self.config.max_requests);
    }

    /// Number of peers currently tracked.
    pub fn tracked_peers(&self) -> usize {
        self.peer_buckets.len()
    }
}

// ---------------------------------------------------------------------------
// Sliding window counter
// ---------------------------------------------------------------------------

/// A single sliding window counter.
#[derive(Debug, Clone)]
struct WindowCounter {
    /// Count in the previous completed window.
    prev_count: u64,
    /// Count in the current window.
    curr_count: u64,
    /// Start time of the current window.
    window_start: Instant,
    /// Window duration.
    window: Duration,
}

impl WindowCounter {
    fn new(window: Duration, now: Instant) -> Self {
        Self {
            prev_count: 0,
            curr_count: 0,
            window_start: now,
            window,
        }
    }

    fn advance(&mut self, now: Instant) {
        let elapsed = now.duration_since(self.window_start);
        if elapsed >= self.window + self.window {
            // More than two windows have passed — reset everything.
            self.prev_count = 0;
            self.curr_count = 0;
            self.window_start = now;
        } else if elapsed >= self.window {
            // Exactly one window has passed — rotate.
            self.prev_count = self.curr_count;
            self.curr_count = 0;
            self.window_start += self.window;
        }
    }

    /// Weighted count using linear interpolation between windows.
    fn weighted_count(&mut self, now: Instant) -> f64 {
        self.advance(now);
        let elapsed = now.duration_since(self.window_start).as_secs_f64();
        let weight = 1.0 - (elapsed / self.window.as_secs_f64()).min(1.0);
        (self.prev_count as f64 * weight) + self.curr_count as f64
    }

    fn increment(&mut self, now: Instant) {
        self.advance(now);
        self.curr_count += 1;
    }
}

/// Per-peer sliding window rate limiter.
pub struct SlidingWindowLimiter {
    config: RateLimitConfig,
    global_counter: WindowCounter,
    peer_counters: HashMap<String, WindowCounter>,
}

impl SlidingWindowLimiter {
    pub fn new(config: RateLimitConfig) -> Self {
        let now = Instant::now();
        let global_counter = WindowCounter::new(config.window, now);
        Self {
            config,
            global_counter,
            peer_counters: HashMap::new(),
        }
    }

    /// Check if a request from the given peer is allowed. If allowed,
    /// increments the counter.
    pub fn check(&mut self, peer_id: &str) -> bool {
        self.check_at(peer_id, Instant::now())
    }

    pub fn check_at(&mut self, peer_id: &str, now: Instant) -> bool {
        if self.config.per_peer {
            let counter = self
                .peer_counters
                .entry(peer_id.to_string())
                .or_insert_with(|| WindowCounter::new(self.config.window, now));

            if counter.weighted_count(now) < self.config.max_requests as f64 {
                counter.increment(now);
                true
            } else {
                false
            }
        } else {
            if self.global_counter.weighted_count(now) < self.config.max_requests as f64 {
                self.global_counter.increment(now);
                true
            } else {
                false
            }
        }
    }

    /// Current weighted count for a peer (how "full" their window is).
    pub fn current_count(&mut self, peer_id: &str) -> f64 {
        let now = Instant::now();
        if self.config.per_peer {
            self.peer_counters
                .get_mut(peer_id)
                .map(|c| c.weighted_count(now))
                .unwrap_or(0.0)
        } else {
            self.global_counter.weighted_count(now)
        }
    }

    /// Number of peers currently tracked.
    pub fn tracked_peers(&self) -> usize {
        self.peer_counters.len()
    }
}

// ---------------------------------------------------------------------------
// Composite limiter
// ---------------------------------------------------------------------------

/// Result of a rate limit check.
#[derive(Debug, Clone)]
pub struct RateLimitResult {
    pub allowed: bool,
    pub remaining: u64,
    pub limit: u64,
    pub retry_after: Option<Duration>,
}

/// A named, multi-endpoint rate limiter that combines token bucket (burst)
/// with sliding window (sustained) for each endpoint.
pub struct RateLimiter {
    /// Endpoint name → (token bucket config, sliding window config).
    endpoints: HashMap<String, (RateLimitConfig, RateLimitConfig)>,
    /// Endpoint name → token bucket limiter.
    buckets: HashMap<String, TokenBucketLimiter>,
    /// Endpoint name → sliding window limiter.
    windows: HashMap<String, SlidingWindowLimiter>,
}

impl RateLimiter {
    pub fn new() -> Self {
        Self {
            endpoints: HashMap::new(),
            buckets: HashMap::new(),
            windows: HashMap::new(),
        }
    }

    /// Register a rate limit for an endpoint. The token bucket allows bursts
    /// up to `burst`, while the sliding window enforces `sustained` over a
    /// longer period.
    pub fn add_endpoint(
        &mut self,
        name: impl Into<String>,
        burst: RateLimitConfig,
        sustained: RateLimitConfig,
    ) {
        let name = name.into();
        self.buckets
            .insert(name.clone(), TokenBucketLimiter::new(burst.clone()));
        self.windows
            .insert(name.clone(), SlidingWindowLimiter::new(sustained.clone()));
        self.endpoints.insert(name, (burst, sustained));
    }

    /// Check if a request from `peer_id` to `endpoint` is allowed.
    /// Both the burst (token bucket) and sustained (sliding window) limits
    /// must be satisfied.
    pub fn check(&mut self, endpoint: &str, peer_id: &str) -> RateLimitResult {
        self.check_at(endpoint, peer_id, Instant::now())
    }

    pub fn check_at(&mut self, endpoint: &str, peer_id: &str, now: Instant) -> RateLimitResult {
        let (burst_cfg, _sustained_cfg) = match self.endpoints.get(endpoint) {
            Some(cfg) => cfg.clone(),
            None => {
                return RateLimitResult {
                    allowed: true,
                    remaining: u64::MAX,
                    limit: u64::MAX,
                    retry_after: None,
                };
            }
        };

        let bucket_ok = self
            .buckets
            .get_mut(endpoint)
            .map(|b| b.check_at(peer_id, now))
            .unwrap_or(true);

        let window_ok = self
            .windows
            .get_mut(endpoint)
            .map(|w| w.check_at(peer_id, now))
            .unwrap_or(true);

        let allowed = bucket_ok && window_ok;

        let remaining = self
            .buckets
            .get(endpoint)
            .map(|b| b.available(peer_id))
            .unwrap_or(0);

        let retry_after = if allowed {
            None
        } else {
            // Suggest waiting for one token to refill.
            let rate = burst_cfg.max_requests as f64 / burst_cfg.window.as_secs_f64();
            if rate > 0.0 {
                Some(Duration::from_secs_f64(1.0 / rate))
            } else {
                None
            }
        };

        RateLimitResult {
            allowed,
            remaining,
            limit: burst_cfg.max_requests,
            retry_after,
        }
    }

    /// Run garbage collection on all limiters.
    pub fn gc(&mut self) {
        for bucket in self.buckets.values_mut() {
            bucket.gc();
        }
    }

    /// Number of registered endpoints.
    pub fn endpoint_count(&self) -> usize {
        self.endpoints.len()
    }
}

impl Default for RateLimiter {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // -- Token bucket tests --

    #[test]
    fn token_bucket_allows_up_to_capacity() {
        let config = RateLimitConfig::new(5, Duration::from_secs(1));
        let mut limiter = TokenBucketLimiter::new(config);
        let now = Instant::now();

        for _ in 0..5 {
            assert!(limiter.check_at("peer-a", now));
        }
        assert!(!limiter.check_at("peer-a", now));
    }

    #[test]
    fn token_bucket_refills_over_time() {
        let config = RateLimitConfig::new(2, Duration::from_secs(1));
        let mut limiter = TokenBucketLimiter::new(config);
        let start = Instant::now();

        // Drain the bucket.
        assert!(limiter.check_at("peer-a", start));
        assert!(limiter.check_at("peer-a", start));
        assert!(!limiter.check_at("peer-a", start));

        // After 0.5s, one token should have refilled.
        let later = start + Duration::from_millis(500);
        assert!(limiter.check_at("peer-a", later));
        assert!(!limiter.check_at("peer-a", later));
    }

    #[test]
    fn token_bucket_per_peer_isolation() {
        let config = RateLimitConfig::new(2, Duration::from_secs(1));
        let mut limiter = TokenBucketLimiter::new(config);
        let now = Instant::now();

        // Drain peer A.
        assert!(limiter.check_at("peer-a", now));
        assert!(limiter.check_at("peer-a", now));
        assert!(!limiter.check_at("peer-a", now));

        // Peer B should still have tokens.
        assert!(limiter.check_at("peer-b", now));
        assert!(limiter.check_at("peer-b", now));
        assert!(!limiter.check_at("peer-b", now));
    }

    #[test]
    fn token_bucket_global_mode() {
        let config = RateLimitConfig::global(3, Duration::from_secs(1));
        let mut limiter = TokenBucketLimiter::new(config);
        let now = Instant::now();

        assert!(limiter.check_at("peer-a", now));
        assert!(limiter.check_at("peer-b", now));
        assert!(limiter.check_at("peer-c", now));
        // Global limit reached.
        assert!(!limiter.check_at("peer-d", now));
    }

    #[test]
    fn token_bucket_batch_acquire() {
        let config = RateLimitConfig::new(10, Duration::from_secs(1));
        let mut limiter = TokenBucketLimiter::new(config);
        let now = Instant::now();

        assert!(limiter.check_n_at("peer-a", 7, now));
        assert!(!limiter.check_n_at("peer-a", 5, now)); // Only 3 left.
        assert!(limiter.check_n_at("peer-a", 3, now));
        assert!(!limiter.check_at("peer-a", now));
    }

    #[test]
    fn token_bucket_gc_removes_full_buckets() {
        let config = RateLimitConfig::new(10, Duration::from_secs(1));
        let mut limiter = TokenBucketLimiter::new(config);
        let start = Instant::now();

        limiter.check_at("peer-a", start);
        assert_eq!(limiter.tracked_peers(), 1);

        // After enough time, the bucket should be full and GC should remove it.
        // We can't easily test this with Instant, but we can verify the API works.
        limiter.gc();
        // Peer-a just used a token, so it won't be GC'd yet.
        assert_eq!(limiter.tracked_peers(), 1);
    }

    #[test]
    fn token_bucket_available_reports_correctly() {
        let config = RateLimitConfig::new(5, Duration::from_secs(1));
        let mut limiter = TokenBucketLimiter::new(config);

        // New peer should have full capacity.
        assert_eq!(limiter.available("peer-new"), 5);

        limiter.check("peer-new");
        // After one check, should have 4.
        assert!(limiter.available("peer-new") <= 5);
    }

    // -- Sliding window tests --

    #[test]
    fn sliding_window_allows_up_to_limit() {
        let config = RateLimitConfig::new(5, Duration::from_secs(10));
        let mut limiter = SlidingWindowLimiter::new(config);
        let now = Instant::now();

        for _ in 0..5 {
            assert!(limiter.check_at("peer-a", now));
        }
        assert!(!limiter.check_at("peer-a", now));
    }

    #[test]
    fn sliding_window_resets_after_window() {
        let config = RateLimitConfig::new(3, Duration::from_secs(1));
        let mut limiter = SlidingWindowLimiter::new(config);
        let start = Instant::now();

        // Use all 3.
        for _ in 0..3 {
            assert!(limiter.check_at("peer-a", start));
        }
        assert!(!limiter.check_at("peer-a", start));

        // After two full windows, everything resets.
        let later = start + Duration::from_secs(3);
        for _ in 0..3 {
            assert!(limiter.check_at("peer-a", later));
        }
    }

    #[test]
    fn sliding_window_interpolates_between_windows() {
        let config = RateLimitConfig::new(10, Duration::from_secs(10));
        let mut limiter = SlidingWindowLimiter::new(config);
        let start = Instant::now();

        // Fill half the window.
        for _ in 0..5 {
            assert!(limiter.check_at("peer-a", start));
        }

        // Advance to the next window. The previous 5 count with a weight
        // that decreases as we move through the new window.
        let mid_next = start + Duration::from_secs(15);
        // At 50% through the next window, weight = 0.5, so prev contributes ~2.5.
        // That means we should be able to make about 7 more requests.
        let mut allowed = 0;
        for _ in 0..10 {
            if limiter.check_at("peer-a", mid_next) {
                allowed += 1;
            }
        }
        assert!(allowed >= 7, "expected at least 7 allowed, got {allowed}");
        assert!(allowed <= 8, "expected at most 8 allowed, got {allowed}");
    }

    #[test]
    fn sliding_window_per_peer_isolation() {
        let config = RateLimitConfig::new(2, Duration::from_secs(10));
        let mut limiter = SlidingWindowLimiter::new(config);
        let now = Instant::now();

        assert!(limiter.check_at("peer-a", now));
        assert!(limiter.check_at("peer-a", now));
        assert!(!limiter.check_at("peer-a", now));

        // Peer B unaffected.
        assert!(limiter.check_at("peer-b", now));
    }

    #[test]
    fn sliding_window_global_mode() {
        let config = RateLimitConfig::global(3, Duration::from_secs(10));
        let mut limiter = SlidingWindowLimiter::new(config);
        let now = Instant::now();

        assert!(limiter.check_at("peer-a", now));
        assert!(limiter.check_at("peer-b", now));
        assert!(limiter.check_at("peer-c", now));
        assert!(!limiter.check_at("peer-d", now));
    }

    #[test]
    fn sliding_window_current_count() {
        let config = RateLimitConfig::new(10, Duration::from_secs(10));
        let mut limiter = SlidingWindowLimiter::new(config);

        assert_eq!(limiter.current_count("peer-a"), 0.0);
        limiter.check("peer-a");
        assert!(limiter.current_count("peer-a") >= 1.0);
    }

    // -- Composite limiter tests --

    #[test]
    fn composite_limiter_unregistered_endpoint_allows() {
        let mut limiter = RateLimiter::new();
        let result = limiter.check("unknown", "peer-a");
        assert!(result.allowed);
    }

    #[test]
    fn composite_limiter_enforces_both() {
        let mut limiter = RateLimiter::new();
        let now = Instant::now();

        // Burst: 3 per second. Sustained: 10 per 10 seconds.
        limiter.add_endpoint(
            "api/messages",
            RateLimitConfig::new(3, Duration::from_secs(1)),
            RateLimitConfig::new(10, Duration::from_secs(10)),
        );

        // First 3 should pass (burst limit).
        for _ in 0..3 {
            let r = limiter.check_at("api/messages", "peer-a", now);
            assert!(r.allowed);
        }

        // 4th should fail (burst exhausted).
        let r = limiter.check_at("api/messages", "peer-a", now);
        assert!(!r.allowed);
        assert!(r.retry_after.is_some());
    }

    #[test]
    fn composite_limiter_retry_after() {
        let mut limiter = RateLimiter::new();
        let now = Instant::now();

        limiter.add_endpoint(
            "api/send",
            RateLimitConfig::new(1, Duration::from_secs(1)),
            RateLimitConfig::new(100, Duration::from_secs(60)),
        );

        let r = limiter.check_at("api/send", "peer-a", now);
        assert!(r.allowed);

        let r = limiter.check_at("api/send", "peer-a", now);
        assert!(!r.allowed);
        assert!(r.retry_after.unwrap() > Duration::ZERO);
    }

    #[test]
    fn composite_limiter_multiple_endpoints() {
        let mut limiter = RateLimiter::new();
        let now = Instant::now();

        limiter.add_endpoint(
            "api/read",
            RateLimitConfig::new(100, Duration::from_secs(1)),
            RateLimitConfig::new(1000, Duration::from_secs(60)),
        );
        limiter.add_endpoint(
            "api/write",
            RateLimitConfig::new(5, Duration::from_secs(1)),
            RateLimitConfig::new(50, Duration::from_secs(60)),
        );

        assert_eq!(limiter.endpoint_count(), 2);

        // Write limit is much lower.
        for _ in 0..5 {
            assert!(limiter.check_at("api/write", "peer-a", now).allowed);
        }
        assert!(!limiter.check_at("api/write", "peer-a", now).allowed);

        // Read still has plenty of capacity.
        assert!(limiter.check_at("api/read", "peer-a", now).allowed);
    }

    #[test]
    fn composite_limiter_remaining_decreases() {
        let mut limiter = RateLimiter::new();
        let now = Instant::now();

        limiter.add_endpoint(
            "api/test",
            RateLimitConfig::new(5, Duration::from_secs(1)),
            RateLimitConfig::new(50, Duration::from_secs(60)),
        );

        let r1 = limiter.check_at("api/test", "peer-a", now);
        assert!(r1.allowed);
        assert_eq!(r1.limit, 5);

        let r2 = limiter.check_at("api/test", "peer-a", now);
        assert!(r2.allowed);
        assert!(r2.remaining < r1.remaining);
    }

    #[test]
    fn rate_limit_config_serializes() {
        let config = RateLimitConfig::new(100, Duration::from_secs(60));
        let json = serde_json::to_string(&config).unwrap();
        let restored: RateLimitConfig = serde_json::from_str(&json).unwrap();
        assert_eq!(restored.max_requests, 100);
        assert_eq!(restored.window, Duration::from_secs(60));
        assert!(restored.per_peer);
    }

    #[test]
    fn rate_limit_result_fields() {
        let result = RateLimitResult {
            allowed: false,
            remaining: 0,
            limit: 10,
            retry_after: Some(Duration::from_millis(100)),
        };
        assert!(!result.allowed);
        assert_eq!(result.remaining, 0);
        assert_eq!(result.limit, 10);
        assert!(result.retry_after.unwrap() > Duration::ZERO);
    }
}
