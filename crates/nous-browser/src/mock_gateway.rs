//! Mock IPFS gateway for testing.
//!
//! Provides a [`MockGateway`] that implements [`Gateway`] with configurable
//! responses, and a [`CachingGateway`] wrapper that adds TTL-based caching
//! to any gateway implementation.

use crate::gateway::{FetchedContent, Gateway, GatewayError};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

/// A mock gateway that returns pre-configured responses.
///
/// Useful for testing code that depends on [`Gateway`] without requiring
/// network access or a running IPFS node.
#[derive(Debug, Clone)]
pub struct MockGateway {
    responses: Arc<Mutex<HashMap<String, MockResponse>>>,
    base_url: String,
    fetch_count: Arc<Mutex<u64>>,
}

#[derive(Debug, Clone)]
enum MockResponse {
    Content {
        data: Vec<u8>,
        content_type: Option<String>,
    },
    NotFound,
    Error(String),
}

impl MockGateway {
    pub fn new() -> Self {
        Self {
            responses: Arc::new(Mutex::new(HashMap::new())),
            base_url: "mock://ipfs".to_string(),
            fetch_count: Arc::new(Mutex::new(0)),
        }
    }

    /// Register content for a CID. Future `fetch()` calls for this CID
    /// will return the given data.
    pub fn register(&self, cid: &str, data: Vec<u8>, content_type: Option<&str>) {
        let mut map = self.responses.lock().unwrap();
        map.insert(
            cid.to_string(),
            MockResponse::Content {
                data,
                content_type: content_type.map(|s| s.to_string()),
            },
        );
    }

    /// Register a CID that will return NotFound.
    pub fn register_not_found(&self, cid: &str) {
        let mut map = self.responses.lock().unwrap();
        map.insert(cid.to_string(), MockResponse::NotFound);
    }

    /// Register a CID that will return an error.
    pub fn register_error(&self, cid: &str, message: &str) {
        let mut map = self.responses.lock().unwrap();
        map.insert(cid.to_string(), MockResponse::Error(message.to_string()));
    }

    /// Number of times `fetch()` has been called.
    pub fn fetch_count(&self) -> u64 {
        *self.fetch_count.lock().unwrap()
    }

    /// Remove all registered responses.
    pub fn clear(&self) {
        self.responses.lock().unwrap().clear();
        *self.fetch_count.lock().unwrap() = 0;
    }
}

impl Default for MockGateway {
    fn default() -> Self {
        Self::new()
    }
}

impl Gateway for MockGateway {
    fn fetch(&self, cid: &str) -> Result<FetchedContent, GatewayError> {
        if cid.is_empty() {
            return Err(GatewayError::InvalidCid("CID cannot be empty".into()));
        }

        *self.fetch_count.lock().unwrap() += 1;

        let map = self.responses.lock().unwrap();
        match map.get(cid) {
            Some(MockResponse::Content { data, content_type }) => Ok(FetchedContent {
                cid: cid.to_string(),
                gateway_url: format!("{}/ipfs/{}", self.base_url, cid),
                data: data.clone(),
                content_type: content_type.clone(),
                size: data.len() as u64,
            }),
            Some(MockResponse::NotFound) => Err(GatewayError::NotFound(cid.to_string())),
            Some(MockResponse::Error(msg)) => Err(GatewayError::RequestFailed(msg.clone())),
            None => Err(GatewayError::NotFound(format!(
                "no mock response registered for CID: {cid}"
            ))),
        }
    }

    fn exists(&self, cid: &str) -> Result<bool, GatewayError> {
        if cid.is_empty() {
            return Err(GatewayError::InvalidCid("CID cannot be empty".into()));
        }

        let map = self.responses.lock().unwrap();
        match map.get(cid) {
            Some(MockResponse::Content { .. }) => Ok(true),
            Some(MockResponse::NotFound) => Ok(false),
            Some(MockResponse::Error(msg)) => Err(GatewayError::RequestFailed(msg.clone())),
            None => Ok(false),
        }
    }

    fn gateway_url(&self) -> &str {
        &self.base_url
    }
}

/// A cached entry with expiration.
#[derive(Debug, Clone)]
struct CacheEntry {
    content: FetchedContent,
    inserted_at: Instant,
    ttl: Duration,
}

impl CacheEntry {
    fn is_expired(&self) -> bool {
        self.inserted_at.elapsed() > self.ttl
    }
}

/// A caching wrapper around any [`Gateway`] implementation.
///
/// Caches successful `fetch()` results with a configurable TTL.
/// Cache misses and errors are not cached.
pub struct CachingGateway<G: Gateway> {
    inner: G,
    cache: Mutex<HashMap<String, CacheEntry>>,
    default_ttl: Duration,
    max_entries: usize,
}

impl<G: Gateway> CachingGateway<G> {
    /// Wrap a gateway with caching. Default TTL is 5 minutes, max 1000 entries.
    pub fn new(inner: G) -> Self {
        Self {
            inner,
            cache: Mutex::new(HashMap::new()),
            default_ttl: Duration::from_secs(300),
            max_entries: 1000,
        }
    }

    /// Set the default TTL for cache entries.
    pub fn with_ttl(mut self, ttl: Duration) -> Self {
        self.default_ttl = ttl;
        self
    }

    /// Set the maximum number of cache entries.
    pub fn with_max_entries(mut self, max: usize) -> Self {
        self.max_entries = max;
        self
    }

    /// Number of entries currently in the cache.
    pub fn cache_size(&self) -> usize {
        self.cache.lock().unwrap().len()
    }

    /// Remove all expired entries.
    pub fn evict_expired(&self) -> usize {
        let mut cache = self.cache.lock().unwrap();
        let before = cache.len();
        cache.retain(|_, entry| !entry.is_expired());
        before - cache.len()
    }

    /// Clear the entire cache.
    pub fn clear_cache(&self) {
        self.cache.lock().unwrap().clear();
    }

    /// Check if a CID is cached and not expired.
    pub fn is_cached(&self, cid: &str) -> bool {
        let cache = self.cache.lock().unwrap();
        cache
            .get(cid)
            .map(|entry| !entry.is_expired())
            .unwrap_or(false)
    }
}

impl<G: Gateway> Gateway for CachingGateway<G> {
    fn fetch(&self, cid: &str) -> Result<FetchedContent, GatewayError> {
        // Check cache first
        {
            let cache = self.cache.lock().unwrap();
            if let Some(entry) = cache.get(cid)
                && !entry.is_expired()
            {
                return Ok(entry.content.clone());
            }
        }

        // Cache miss — fetch from inner gateway
        let content = self.inner.fetch(cid)?;

        // Store in cache
        {
            let mut cache = self.cache.lock().unwrap();

            // Evict expired entries if at capacity
            if cache.len() >= self.max_entries {
                cache.retain(|_, entry| !entry.is_expired());
            }

            // If still at capacity, evict oldest entry
            if cache.len() >= self.max_entries
                && let Some(oldest_key) = cache
                    .iter()
                    .min_by_key(|(_, entry)| entry.inserted_at)
                    .map(|(key, _)| key.clone())
            {
                cache.remove(&oldest_key);
            }

            cache.insert(
                cid.to_string(),
                CacheEntry {
                    content: content.clone(),
                    inserted_at: Instant::now(),
                    ttl: self.default_ttl,
                },
            );
        }

        Ok(content)
    }

    fn exists(&self, cid: &str) -> Result<bool, GatewayError> {
        // Check cache first
        {
            let cache = self.cache.lock().unwrap();
            if let Some(entry) = cache.get(cid)
                && !entry.is_expired()
            {
                return Ok(true);
            }
        }

        self.inner.exists(cid)
    }

    fn gateway_url(&self) -> &str {
        self.inner.gateway_url()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mock_fetch_registered_content() {
        let gw = MockGateway::new();
        gw.register("QmTest", b"hello world".to_vec(), Some("text/plain"));

        let content = gw.fetch("QmTest").unwrap();
        assert_eq!(content.data, b"hello world");
        assert_eq!(content.content_type, Some("text/plain".to_string()));
        assert_eq!(content.size, 11);
        assert_eq!(content.cid, "QmTest");
        assert!(content.gateway_url.contains("QmTest"));
    }

    #[test]
    fn mock_fetch_not_found() {
        let gw = MockGateway::new();
        gw.register_not_found("QmMissing");

        let result = gw.fetch("QmMissing");
        assert!(matches!(result.unwrap_err(), GatewayError::NotFound(_)));
    }

    #[test]
    fn mock_fetch_error() {
        let gw = MockGateway::new();
        gw.register_error("QmBroken", "disk failure");

        let result = gw.fetch("QmBroken");
        assert!(matches!(
            result.unwrap_err(),
            GatewayError::RequestFailed(_)
        ));
    }

    #[test]
    fn mock_fetch_unregistered_returns_not_found() {
        let gw = MockGateway::new();
        let result = gw.fetch("QmUnknown");
        assert!(matches!(result.unwrap_err(), GatewayError::NotFound(_)));
    }

    #[test]
    fn mock_fetch_empty_cid_is_error() {
        let gw = MockGateway::new();
        let result = gw.fetch("");
        assert!(matches!(result.unwrap_err(), GatewayError::InvalidCid(_)));
    }

    #[test]
    fn mock_exists_registered() {
        let gw = MockGateway::new();
        gw.register("QmExists", vec![1], None);
        assert!(gw.exists("QmExists").unwrap());
    }

    #[test]
    fn mock_exists_not_found() {
        let gw = MockGateway::new();
        gw.register_not_found("QmGone");
        assert!(!gw.exists("QmGone").unwrap());
    }

    #[test]
    fn mock_exists_unregistered() {
        let gw = MockGateway::new();
        assert!(!gw.exists("QmUnknown").unwrap());
    }

    #[test]
    fn mock_fetch_count() {
        let gw = MockGateway::new();
        gw.register("QmA", vec![1], None);
        assert_eq!(gw.fetch_count(), 0);

        let _ = gw.fetch("QmA");
        assert_eq!(gw.fetch_count(), 1);

        let _ = gw.fetch("QmA");
        let _ = gw.fetch("QmMissing");
        assert_eq!(gw.fetch_count(), 3);
    }

    #[test]
    fn mock_clear() {
        let gw = MockGateway::new();
        gw.register("QmA", vec![1], None);
        let _ = gw.fetch("QmA");

        gw.clear();
        assert_eq!(gw.fetch_count(), 0);
        assert!(!gw.exists("QmA").unwrap());
    }

    #[test]
    fn mock_gateway_url() {
        let gw = MockGateway::new();
        assert_eq!(gw.gateway_url(), "mock://ipfs");
    }

    #[test]
    fn mock_binary_content() {
        let gw = MockGateway::new();
        let binary = vec![0x00, 0xFF, 0x89, 0x50, 0x4E, 0x47]; // PNG header
        gw.register("QmImage", binary.clone(), Some("image/png"));

        let content = gw.fetch("QmImage").unwrap();
        assert_eq!(content.data, binary);
        assert_eq!(content.content_type, Some("image/png".to_string()));
    }

    #[test]
    fn mock_no_content_type() {
        let gw = MockGateway::new();
        gw.register("QmRaw", vec![42], None);

        let content = gw.fetch("QmRaw").unwrap();
        assert_eq!(content.content_type, None);
    }

    #[test]
    fn caching_gateway_caches_fetch() {
        let mock = MockGateway::new();
        mock.register("QmCached", b"cached data".to_vec(), Some("text/plain"));

        let caching = CachingGateway::new(mock.clone());

        // First fetch — cache miss, hits inner gateway
        let content = caching.fetch("QmCached").unwrap();
        assert_eq!(content.data, b"cached data");
        assert_eq!(mock.fetch_count(), 1);

        // Second fetch — cache hit, does not hit inner gateway
        let content = caching.fetch("QmCached").unwrap();
        assert_eq!(content.data, b"cached data");
        assert_eq!(mock.fetch_count(), 1); // still 1
    }

    #[test]
    fn caching_gateway_does_not_cache_errors() {
        let mock = MockGateway::new();
        mock.register_not_found("QmMissing");

        let caching = CachingGateway::new(mock.clone());

        let _ = caching.fetch("QmMissing");
        assert_eq!(caching.cache_size(), 0);
    }

    #[test]
    fn caching_gateway_ttl_expiry() {
        let mock = MockGateway::new();
        mock.register("QmShort", vec![1], None);

        let caching = CachingGateway::new(mock.clone()).with_ttl(Duration::from_millis(1));

        let _ = caching.fetch("QmShort").unwrap();
        assert_eq!(mock.fetch_count(), 1);

        // Wait for TTL to expire
        std::thread::sleep(Duration::from_millis(10));

        // Should re-fetch from inner
        let _ = caching.fetch("QmShort").unwrap();
        assert_eq!(mock.fetch_count(), 2);
    }

    #[test]
    fn caching_gateway_is_cached() {
        let mock = MockGateway::new();
        mock.register("QmCheck", vec![1], None);

        let caching = CachingGateway::new(mock);
        assert!(!caching.is_cached("QmCheck"));

        let _ = caching.fetch("QmCheck").unwrap();
        assert!(caching.is_cached("QmCheck"));
    }

    #[test]
    fn caching_gateway_clear() {
        let mock = MockGateway::new();
        mock.register("QmA", vec![1], None);

        let caching = CachingGateway::new(mock);
        let _ = caching.fetch("QmA").unwrap();
        assert_eq!(caching.cache_size(), 1);

        caching.clear_cache();
        assert_eq!(caching.cache_size(), 0);
    }

    #[test]
    fn caching_gateway_evict_expired() {
        let mock = MockGateway::new();
        mock.register("QmOld", vec![1], None);

        let caching = CachingGateway::new(mock).with_ttl(Duration::from_millis(1));

        let _ = caching.fetch("QmOld").unwrap();
        assert_eq!(caching.cache_size(), 1);

        std::thread::sleep(Duration::from_millis(10));

        let evicted = caching.evict_expired();
        assert_eq!(evicted, 1);
        assert_eq!(caching.cache_size(), 0);
    }

    #[test]
    fn caching_gateway_max_entries() {
        let mock = MockGateway::new();
        for i in 0..5 {
            mock.register(&format!("Qm{i}"), vec![i as u8], None);
        }

        let caching = CachingGateway::new(mock).with_max_entries(3);

        for i in 0..5 {
            let _ = caching.fetch(&format!("Qm{i}")).unwrap();
        }

        // Should have evicted some entries to stay at max
        assert!(caching.cache_size() <= 3);
    }

    #[test]
    fn caching_gateway_exists_from_cache() {
        let mock = MockGateway::new();
        mock.register("QmCached", vec![1], None);

        let caching = CachingGateway::new(mock.clone());

        // Populate cache
        let _ = caching.fetch("QmCached").unwrap();

        // exists() should use cache
        assert!(caching.exists("QmCached").unwrap());
    }

    #[test]
    fn caching_gateway_exists_delegates_on_miss() {
        let mock = MockGateway::new();
        mock.register("QmUncached", vec![1], None);

        let caching = CachingGateway::new(mock);

        // No cache entry — delegates to inner
        assert!(caching.exists("QmUncached").unwrap());
    }

    #[test]
    fn caching_gateway_url_delegates() {
        let mock = MockGateway::new();
        let caching = CachingGateway::new(mock);
        assert_eq!(caching.gateway_url(), "mock://ipfs");
    }

    #[test]
    fn mock_overwrite_response() {
        let gw = MockGateway::new();
        gw.register("QmA", vec![1], None);
        gw.register("QmA", vec![2, 3], Some("text/plain"));

        let content = gw.fetch("QmA").unwrap();
        assert_eq!(content.data, vec![2, 3]);
        assert_eq!(content.content_type, Some("text/plain".to_string()));
    }

    #[test]
    fn mock_large_content() {
        let gw = MockGateway::new();
        let large = vec![0xAB; 1024 * 1024]; // 1 MiB
        gw.register("QmLarge", large.clone(), Some("application/octet-stream"));

        let content = gw.fetch("QmLarge").unwrap();
        assert_eq!(content.data.len(), 1024 * 1024);
        assert_eq!(content.size, 1024 * 1024);
    }

    #[test]
    fn mock_exists_empty_cid_is_error() {
        let gw = MockGateway::new();
        assert!(matches!(
            gw.exists("").unwrap_err(),
            GatewayError::InvalidCid(_)
        ));
    }

    #[test]
    fn caching_gateway_thread_safe() {
        use std::thread;

        let mock = MockGateway::new();
        for i in 0..10 {
            mock.register(&format!("Qm{i}"), vec![i as u8], None);
        }

        let caching = Arc::new(CachingGateway::new(mock));

        let handles: Vec<_> = (0..10)
            .map(|i| {
                let caching = Arc::clone(&caching);
                thread::spawn(move || {
                    let cid = format!("Qm{i}");
                    caching.fetch(&cid).unwrap()
                })
            })
            .collect();

        for handle in handles {
            let content = handle.join().unwrap();
            assert_eq!(content.data.len(), 1);
        }
    }
}
