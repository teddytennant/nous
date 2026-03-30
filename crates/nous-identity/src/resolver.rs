//! DID resolution — resolve DIDs to their associated documents.
//!
//! Supports `did:key` (self-certifying), `did:nous` (network-resolved),
//! and `did:web` (DNS-based) methods. The resolver is pluggable via
//! the `DidResolver` trait.

use std::collections::HashMap;
use std::time::{Duration, Instant};

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use nous_core::{Error, Result};

use crate::did::{Document, VerificationMethod};

/// Supported DID methods.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum DidMethod {
    Key,
    Nous,
    Web,
    Unknown(String),
}

impl DidMethod {
    /// Parse the method from a DID string.
    pub fn from_did(did: &str) -> Result<Self> {
        if !did.starts_with("did:") {
            return Err(Error::Identity(format!("invalid DID format: {did}")));
        }

        let parts: Vec<&str> = did.splitn(3, ':').collect();
        if parts.len() < 3 {
            return Err(Error::Identity(format!("invalid DID format: {did}")));
        }

        Ok(match parts[1] {
            "key" => DidMethod::Key,
            "nous" => DidMethod::Nous,
            "web" => DidMethod::Web,
            other => DidMethod::Unknown(other.to_string()),
        })
    }

    /// Whether this method can be resolved locally without network access.
    pub fn is_local(&self) -> bool {
        matches!(self, DidMethod::Key)
    }

    pub fn as_str(&self) -> &str {
        match self {
            DidMethod::Key => "key",
            DidMethod::Nous => "nous",
            DidMethod::Web => "web",
            DidMethod::Unknown(s) => s,
        }
    }
}

/// Metadata about a DID resolution.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResolutionMetadata {
    pub content_type: String,
    pub duration_ms: u64,
    pub did_method: String,
    pub resolved_at: DateTime<Utc>,
    /// Whether the result came from cache.
    pub cached: bool,
    pub error: Option<String>,
}

/// The result of resolving a DID.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResolutionResult {
    pub document: Option<Document>,
    pub metadata: ResolutionMetadata,
}

impl ResolutionResult {
    fn success(document: Document, method: &DidMethod, duration: Duration, cached: bool) -> Self {
        Self {
            document: Some(document),
            metadata: ResolutionMetadata {
                content_type: "application/did+ld+json".into(),
                duration_ms: duration.as_millis() as u64,
                did_method: method.as_str().to_string(),
                resolved_at: Utc::now(),
                cached,
                error: None,
            },
        }
    }

    fn error(method: &DidMethod, error: String, duration: Duration) -> Self {
        Self {
            document: None,
            metadata: ResolutionMetadata {
                content_type: "application/did+ld+json".into(),
                duration_ms: duration.as_millis() as u64,
                did_method: method.as_str().to_string(),
                resolved_at: Utc::now(),
                cached: false,
                error: Some(error),
            },
        }
    }

    pub fn is_success(&self) -> bool {
        self.document.is_some() && self.metadata.error.is_none()
    }
}

/// A cached DID document with TTL.
#[derive(Debug, Clone)]
struct CacheEntry {
    document: Document,
    cached_at: Instant,
    ttl: Duration,
}

impl CacheEntry {
    fn is_expired(&self) -> bool {
        self.cached_at.elapsed() > self.ttl
    }
}

/// Configuration for the DID resolver.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResolverConfig {
    /// Whether to cache resolved documents.
    pub enable_cache: bool,
    /// Default TTL for cached documents.
    pub cache_ttl_secs: u64,
    /// Maximum cache entries.
    pub max_cache_size: usize,
    /// Per-method TTL overrides.
    pub method_ttl_overrides: HashMap<String, u64>,
}

impl Default for ResolverConfig {
    fn default() -> Self {
        Self {
            enable_cache: true,
            cache_ttl_secs: 300,
            max_cache_size: 1000,
            method_ttl_overrides: HashMap::new(),
        }
    }
}

/// DID resolver with caching and multi-method support.
pub struct DidResolver {
    config: ResolverConfig,
    cache: HashMap<String, CacheEntry>,
    /// Manual document registrations (for did:nous or testing).
    registered: HashMap<String, Document>,
    /// Resolution statistics.
    stats: ResolverStats,
}

/// Statistics about resolver usage.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ResolverStats {
    pub total_resolutions: u64,
    pub cache_hits: u64,
    pub cache_misses: u64,
    pub errors: u64,
    pub by_method: HashMap<String, u64>,
}

impl ResolverStats {
    pub fn cache_hit_rate(&self) -> f64 {
        let total = self.cache_hits + self.cache_misses;
        if total == 0 {
            return 0.0;
        }
        self.cache_hits as f64 / total as f64
    }
}

impl DidResolver {
    pub fn new(config: ResolverConfig) -> Self {
        Self {
            config,
            cache: HashMap::new(),
            registered: HashMap::new(),
            stats: ResolverStats::default(),
        }
    }

    /// Resolve a DID to its document.
    pub fn resolve(&mut self, did: &str) -> ResolutionResult {
        let start = Instant::now();
        self.stats.total_resolutions += 1;

        let method = match DidMethod::from_did(did) {
            Ok(m) => m,
            Err(e) => {
                self.stats.errors += 1;
                return ResolutionResult::error(
                    &DidMethod::Unknown("".into()),
                    e.to_string(),
                    start.elapsed(),
                );
            }
        };

        *self
            .stats
            .by_method
            .entry(method.as_str().to_string())
            .or_insert(0) += 1;

        // Check cache.
        if self.config.enable_cache {
            if let Some(entry) = self.cache.get(did) {
                if !entry.is_expired() {
                    self.stats.cache_hits += 1;
                    return ResolutionResult::success(
                        entry.document.clone(),
                        &method,
                        start.elapsed(),
                        true,
                    );
                }
                // Expired — remove and resolve fresh.
                self.cache.remove(did);
            }
            self.stats.cache_misses += 1;
        }

        // Resolve by method.
        let result = match &method {
            DidMethod::Key => self.resolve_did_key(did),
            DidMethod::Nous => self.resolve_did_nous(did),
            DidMethod::Web => {
                Err(Error::Identity("did:web resolution requires HTTP — not available in sync resolver".into()))
            }
            DidMethod::Unknown(m) => {
                Err(Error::Identity(format!("unsupported DID method: {m}")))
            }
        };

        match result {
            Ok(doc) => {
                // Cache the result.
                if self.config.enable_cache {
                    self.cache_document(did, doc.clone(), &method);
                }
                ResolutionResult::success(doc, &method, start.elapsed(), false)
            }
            Err(e) => {
                self.stats.errors += 1;
                ResolutionResult::error(&method, e.to_string(), start.elapsed())
            }
        }
    }

    /// Register a document for did:nous resolution.
    pub fn register(&mut self, did: &str, document: Document) {
        self.registered.insert(did.to_string(), document);
    }

    /// Unregister a document.
    pub fn unregister(&mut self, did: &str) -> bool {
        self.registered.remove(did).is_some()
    }

    /// Invalidate a cached document.
    pub fn invalidate(&mut self, did: &str) -> bool {
        self.cache.remove(did).is_some()
    }

    /// Clear all cached documents.
    pub fn clear_cache(&mut self) {
        self.cache.clear();
    }

    /// Get resolver statistics.
    pub fn stats(&self) -> &ResolverStats {
        &self.stats
    }

    /// Number of documents currently cached.
    pub fn cache_size(&self) -> usize {
        self.cache.len()
    }

    /// Number of registered documents.
    pub fn registered_count(&self) -> usize {
        self.registered.len()
    }

    fn resolve_did_key(&self, did: &str) -> Result<Document> {
        // did:key is self-certifying — the DID itself contains the public key.
        let verifying_key = nous_crypto::keys::did_to_public_key(did)?;
        let now = Utc::now();

        let signing_method = VerificationMethod {
            id: format!("{did}#signing"),
            r#type: "Ed25519VerificationKey2020".to_string(),
            controller: did.to_string(),
            public_key_multibase: format!(
                "z{}",
                bs58::encode(verifying_key.to_bytes()).into_string()
            ),
        };

        Ok(Document {
            context: vec![
                "https://www.w3.org/ns/did/v1".to_string(),
                "https://w3id.org/security/suites/ed25519-2020/v1".to_string(),
            ],
            id: did.to_string(),
            authentication: vec![format!("{did}#signing")],
            key_agreement: Vec::new(),
            verification_method: vec![signing_method],
            created: now,
            updated: now,
        })
    }

    fn resolve_did_nous(&self, did: &str) -> Result<Document> {
        self.registered
            .get(did)
            .cloned()
            .ok_or_else(|| Error::Identity(format!("did:nous not found: {did}")))
    }

    fn cache_document(&mut self, did: &str, document: Document, method: &DidMethod) {
        // Enforce cache size limit.
        if self.cache.len() >= self.config.max_cache_size {
            self.evict_expired_or_oldest();
        }

        let ttl_secs = self
            .config
            .method_ttl_overrides
            .get(method.as_str())
            .copied()
            .unwrap_or(self.config.cache_ttl_secs);

        self.cache.insert(
            did.to_string(),
            CacheEntry {
                document,
                cached_at: Instant::now(),
                ttl: Duration::from_secs(ttl_secs),
            },
        );
    }

    fn evict_expired_or_oldest(&mut self) {
        // First try to evict expired entries.
        self.cache.retain(|_, entry| !entry.is_expired());

        // If still full, remove the oldest entry.
        if self.cache.len() >= self.config.max_cache_size {
            if let Some(oldest_key) = self
                .cache
                .iter()
                .min_by_key(|(_, entry)| entry.cached_at)
                .map(|(k, _)| k.clone())
            {
                self.cache.remove(&oldest_key);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_resolver() -> DidResolver {
        DidResolver::new(ResolverConfig::default())
    }

    fn test_document(did: &str) -> Document {
        let keypair = nous_crypto::keys::KeyPair::generate();
        let mut doc = Document::from_keypair(&keypair);
        doc.id = did.to_string();
        doc
    }

    // --- DidMethod tests ---

    #[test]
    fn did_method_from_key() {
        let method = DidMethod::from_did("did:key:z6MkhaXgBZDvotDkL5257faiztiGiC2QtKLGpbnnEGta2doK").unwrap();
        assert_eq!(method, DidMethod::Key);
        assert!(method.is_local());
    }

    #[test]
    fn did_method_from_nous() {
        let method = DidMethod::from_did("did:nous:abc123").unwrap();
        assert_eq!(method, DidMethod::Nous);
        assert!(!method.is_local());
    }

    #[test]
    fn did_method_from_web() {
        let method = DidMethod::from_did("did:web:example.com").unwrap();
        assert_eq!(method, DidMethod::Web);
    }

    #[test]
    fn did_method_unknown() {
        let method = DidMethod::from_did("did:ion:abc123").unwrap();
        assert_eq!(method, DidMethod::Unknown("ion".into()));
    }

    #[test]
    fn did_method_invalid_format() {
        assert!(DidMethod::from_did("not-a-did").is_err());
        assert!(DidMethod::from_did("did:").is_err());
        assert!(DidMethod::from_did("did:key").is_err());
    }

    // --- Resolver tests ---

    #[test]
    fn resolve_did_key() {
        let keypair = nous_crypto::keys::KeyPair::generate();
        let did = nous_crypto::keys::public_key_to_did(&keypair.verifying_key());

        let mut resolver = test_resolver();
        let result = resolver.resolve(&did);

        assert!(result.is_success());
        let doc = result.document.unwrap();
        assert_eq!(doc.id, did);
        assert!(!result.metadata.cached);
    }

    #[test]
    fn resolve_did_nous_registered() {
        let mut resolver = test_resolver();
        let doc = test_document("did:nous:alice");
        resolver.register("did:nous:alice", doc.clone());

        let result = resolver.resolve("did:nous:alice");
        assert!(result.is_success());
        assert_eq!(result.document.unwrap().id, "did:nous:alice");
    }

    #[test]
    fn resolve_did_nous_not_found() {
        let mut resolver = test_resolver();
        let result = resolver.resolve("did:nous:unknown");
        assert!(!result.is_success());
        assert!(result.metadata.error.is_some());
    }

    #[test]
    fn resolve_did_web_unsupported_in_sync() {
        let mut resolver = test_resolver();
        let result = resolver.resolve("did:web:example.com");
        assert!(!result.is_success());
    }

    #[test]
    fn resolve_unknown_method() {
        let mut resolver = test_resolver();
        let result = resolver.resolve("did:ion:abc123");
        assert!(!result.is_success());
    }

    #[test]
    fn resolve_invalid_did() {
        let mut resolver = test_resolver();
        let result = resolver.resolve("not-a-did");
        assert!(!result.is_success());
    }

    // --- Cache tests ---

    #[test]
    fn cache_hit_on_second_resolve() {
        let keypair = nous_crypto::keys::KeyPair::generate();
        let did = nous_crypto::keys::public_key_to_did(&keypair.verifying_key());

        let mut resolver = test_resolver();

        let r1 = resolver.resolve(&did);
        assert!(!r1.metadata.cached);

        let r2 = resolver.resolve(&did);
        assert!(r2.metadata.cached);
        assert_eq!(resolver.stats().cache_hits, 1);
    }

    #[test]
    fn cache_invalidation() {
        let keypair = nous_crypto::keys::KeyPair::generate();
        let did = nous_crypto::keys::public_key_to_did(&keypair.verifying_key());

        let mut resolver = test_resolver();
        resolver.resolve(&did);
        assert_eq!(resolver.cache_size(), 1);

        resolver.invalidate(&did);
        assert_eq!(resolver.cache_size(), 0);
    }

    #[test]
    fn cache_clear() {
        let mut resolver = test_resolver();
        let doc = test_document("did:nous:a");
        resolver.register("did:nous:a", doc);
        resolver.resolve("did:nous:a");
        assert_eq!(resolver.cache_size(), 1);

        resolver.clear_cache();
        assert_eq!(resolver.cache_size(), 0);
    }

    #[test]
    fn cache_disabled() {
        let keypair = nous_crypto::keys::KeyPair::generate();
        let did = nous_crypto::keys::public_key_to_did(&keypair.verifying_key());

        let mut resolver = DidResolver::new(ResolverConfig {
            enable_cache: false,
            ..Default::default()
        });

        resolver.resolve(&did);
        resolver.resolve(&did);

        assert_eq!(resolver.cache_size(), 0);
        assert_eq!(resolver.stats().cache_hits, 0);
    }

    // --- Registration tests ---

    #[test]
    fn register_and_unregister() {
        let mut resolver = test_resolver();
        let doc = test_document("did:nous:bob");

        resolver.register("did:nous:bob", doc);
        assert_eq!(resolver.registered_count(), 1);

        assert!(resolver.unregister("did:nous:bob"));
        assert_eq!(resolver.registered_count(), 0);
        assert!(!resolver.unregister("did:nous:bob"));
    }

    // --- Stats tests ---

    #[test]
    fn stats_tracking() {
        let mut resolver = test_resolver();
        let doc = test_document("did:nous:stats");
        resolver.register("did:nous:stats", doc);

        resolver.resolve("did:nous:stats");
        resolver.resolve("did:nous:stats"); // Cache hit.
        resolver.resolve("did:nous:missing"); // Error.

        let stats = resolver.stats();
        assert_eq!(stats.total_resolutions, 3);
        assert_eq!(stats.cache_hits, 1);
        assert_eq!(stats.errors, 1);
        assert_eq!(*stats.by_method.get("nous").unwrap(), 3);
    }

    #[test]
    fn stats_cache_hit_rate() {
        let mut stats = ResolverStats::default();
        assert_eq!(stats.cache_hit_rate(), 0.0);

        stats.cache_hits = 3;
        stats.cache_misses = 1;
        assert!((stats.cache_hit_rate() - 0.75).abs() < 1e-6);
    }

    #[test]
    fn resolution_metadata_serializes() {
        let metadata = ResolutionMetadata {
            content_type: "application/did+ld+json".into(),
            duration_ms: 42,
            did_method: "key".into(),
            resolved_at: Utc::now(),
            cached: false,
            error: None,
        };
        let json = serde_json::to_string(&metadata).unwrap();
        let restored: ResolutionMetadata = serde_json::from_str(&json).unwrap();
        assert_eq!(restored.duration_ms, 42);
    }

    #[test]
    fn resolver_config_default() {
        let config = ResolverConfig::default();
        assert!(config.enable_cache);
        assert_eq!(config.cache_ttl_secs, 300);
        assert_eq!(config.max_cache_size, 1000);
    }

    #[test]
    fn did_method_serializes() {
        let method = DidMethod::Nous;
        let json = serde_json::to_string(&method).unwrap();
        let restored: DidMethod = serde_json::from_str(&json).unwrap();
        assert_eq!(restored, DidMethod::Nous);
    }
}
