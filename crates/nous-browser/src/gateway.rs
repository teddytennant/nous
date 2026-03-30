//! IPFS gateway trait and implementations.
//!
//! Defines a [`Gateway`] trait for fetching content from decentralized storage
//! networks. Includes a [`PublicIpfsGateway`] that resolves IPFS/IPNS URLs
//! through a public HTTP gateway (e.g., `https://ipfs.io`) and a
//! [`StubEnsResolver`] that documents the ENS interface without requiring
//! an Ethereum RPC connection.

use serde::{Deserialize, Serialize};
use thiserror::Error;

/// Errors specific to gateway operations.
#[derive(Debug, Error)]
pub enum GatewayError {
    /// The requested content was not found.
    #[error("content not found: {0}")]
    NotFound(String),

    /// The gateway request failed (network error, timeout, etc.).
    #[error("gateway request failed: {0}")]
    RequestFailed(String),

    /// The CID or content identifier is malformed.
    #[error("invalid content identifier: {0}")]
    InvalidCid(String),

    /// ENS resolution is not available (requires Ethereum RPC).
    #[error("ENS resolution unavailable: {0}")]
    EnsUnavailable(String),

    /// The content exceeds the maximum allowed size.
    #[error("content too large: {size} bytes (max {max} bytes)")]
    ContentTooLarge { size: u64, max: u64 },
}

/// Metadata about fetched content.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FetchedContent {
    /// The original CID or URL requested.
    pub cid: String,
    /// The resolved gateway URL used to fetch.
    pub gateway_url: String,
    /// The raw content bytes.
    pub data: Vec<u8>,
    /// Content type from the gateway response, if provided.
    pub content_type: Option<String>,
    /// Content size in bytes.
    pub size: u64,
}

/// Trait for fetching content from decentralized storage gateways.
///
/// Implementors provide access to IPFS, IPNS, or other content-addressed
/// networks. The trait is intentionally synchronous to avoid requiring an
/// async runtime -- implementations can use blocking HTTP internally.
pub trait Gateway: Send + Sync {
    /// Fetch content by its CID (Content Identifier).
    ///
    /// For IPFS, the CID is the multihash of the content (e.g., `QmTest...`
    /// or `bafybeig...`). For IPNS, it's a peer ID or DNSLink name.
    fn fetch(&self, cid: &str) -> Result<FetchedContent, GatewayError>;

    /// Check if a CID exists without downloading the full content.
    fn exists(&self, cid: &str) -> Result<bool, GatewayError>;

    /// Return the base URL of the gateway.
    fn gateway_url(&self) -> &str;
}

/// An IPFS gateway that resolves content through a public HTTP gateway.
///
/// By default uses `https://ipfs.io` but can be configured to use any
/// IPFS HTTP gateway (e.g., `https://dweb.link`, `https://cloudflare-ipfs.com`,
/// or a local node at `http://localhost:8080`).
///
/// # Note
///
/// This implementation uses synchronous HTTP via `reqwest::blocking` to
/// keep the [`Gateway`] trait synchronous. For production use with
/// high-throughput requirements, consider implementing the [`Gateway`] trait
/// with an async HTTP client.
#[derive(Debug, Clone)]
pub struct PublicIpfsGateway {
    /// Base URL of the IPFS gateway.
    base_url: String,
    /// Maximum content size to fetch (bytes). Defaults to 50 MiB.
    max_size: u64,
}

impl PublicIpfsGateway {
    /// Create a gateway pointing at `https://ipfs.io`.
    pub fn new() -> Self {
        Self {
            base_url: "https://ipfs.io".to_string(),
            max_size: 50 * 1024 * 1024,
        }
    }

    /// Create a gateway with a custom base URL.
    pub fn with_url(base_url: impl Into<String>) -> Self {
        Self {
            base_url: base_url.into(),
            max_size: 50 * 1024 * 1024,
        }
    }

    /// Set the maximum content size to fetch.
    pub fn with_max_size(mut self, max_bytes: u64) -> Self {
        self.max_size = max_bytes;
        self
    }

    /// Build the full gateway URL for a CID.
    pub fn url_for_cid(&self, cid: &str) -> String {
        format!("{}/ipfs/{}", self.base_url, cid)
    }

    /// Build the full gateway URL for an IPNS name.
    pub fn url_for_ipns(&self, name: &str) -> String {
        format!("{}/ipns/{}", self.base_url, name)
    }
}

impl Default for PublicIpfsGateway {
    fn default() -> Self {
        Self::new()
    }
}

impl Gateway for PublicIpfsGateway {
    fn fetch(&self, cid: &str) -> Result<FetchedContent, GatewayError> {
        if cid.is_empty() {
            return Err(GatewayError::InvalidCid("CID cannot be empty".into()));
        }

        let gateway_url = self.url_for_cid(cid);

        let response = reqwest::blocking::get(&gateway_url)
            .map_err(|e| GatewayError::RequestFailed(e.to_string()))?;

        let status = response.status();
        if status == reqwest::StatusCode::NOT_FOUND {
            return Err(GatewayError::NotFound(cid.to_string()));
        }
        if !status.is_success() {
            return Err(GatewayError::RequestFailed(format!(
                "gateway returned HTTP {status}"
            )));
        }

        let content_type = response
            .headers()
            .get(reqwest::header::CONTENT_TYPE)
            .and_then(|v| v.to_str().ok())
            .map(|s| s.to_string());

        // Check Content-Length header before downloading the body when available.
        if let Some(len) = response.content_length()
            && len > self.max_size
        {
            return Err(GatewayError::ContentTooLarge {
                size: len,
                max: self.max_size,
            });
        }

        let data = response
            .bytes()
            .map_err(|e| GatewayError::RequestFailed(e.to_string()))?;

        let size = data.len() as u64;
        if size > self.max_size {
            return Err(GatewayError::ContentTooLarge {
                size,
                max: self.max_size,
            });
        }

        Ok(FetchedContent {
            cid: cid.to_string(),
            gateway_url,
            data: data.to_vec(),
            content_type,
            size,
        })
    }

    fn exists(&self, cid: &str) -> Result<bool, GatewayError> {
        if cid.is_empty() {
            return Err(GatewayError::InvalidCid("CID cannot be empty".into()));
        }

        let gateway_url = self.url_for_cid(cid);

        let client = reqwest::blocking::Client::new();
        let response = client
            .head(&gateway_url)
            .send()
            .map_err(|e| GatewayError::RequestFailed(e.to_string()))?;

        Ok(response.status().is_success())
    }

    fn gateway_url(&self) -> &str {
        &self.base_url
    }
}

/// A stub ENS resolver that documents the interface.
///
/// ENS (Ethereum Name Service) resolution requires an Ethereum JSON-RPC
/// connection to read the ENS registry contract. This stub provides the
/// correct interface so that a real implementation can be swapped in when
/// an Ethereum provider is available.
///
/// # Future Implementation
///
/// A real implementation would:
/// 1. Connect to an Ethereum RPC endpoint (e.g., Infura, Alchemy, local node)
/// 2. Call the ENS registry contract to resolve the name
/// 3. If the content hash is an IPFS CID, return it
/// 4. Cache results with appropriate TTL
#[derive(Debug, Clone)]
pub struct StubEnsResolver;

impl StubEnsResolver {
    pub fn new() -> Self {
        Self
    }

    /// Attempt to resolve an ENS name to a content hash.
    ///
    /// Always returns [`GatewayError::EnsUnavailable`] because ENS resolution
    /// requires an Ethereum RPC connection not included in this crate.
    pub fn resolve(&self, ens_name: &str) -> Result<String, GatewayError> {
        Err(GatewayError::EnsUnavailable(format!(
            "ENS resolution for '{ens_name}' requires an Ethereum RPC provider — \
             configure one via nous-net or implement a custom resolver"
        )))
    }

    /// Check if a name looks like a valid ENS name.
    pub fn is_ens_name(name: &str) -> bool {
        name.ends_with(".eth") && name.len() > 4 && !name.contains(' ')
    }
}

impl Default for StubEnsResolver {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn public_gateway_default_url() {
        let gw = PublicIpfsGateway::new();
        assert_eq!(gw.gateway_url(), "https://ipfs.io");
    }

    #[test]
    fn public_gateway_custom_url() {
        let gw = PublicIpfsGateway::with_url("http://localhost:8080");
        assert_eq!(gw.gateway_url(), "http://localhost:8080");
    }

    #[test]
    fn url_for_cid() {
        let gw = PublicIpfsGateway::new();
        assert_eq!(
            gw.url_for_cid("QmTest123"),
            "https://ipfs.io/ipfs/QmTest123"
        );
    }

    #[test]
    fn url_for_ipns() {
        let gw = PublicIpfsGateway::new();
        assert_eq!(
            gw.url_for_ipns("example.com"),
            "https://ipfs.io/ipns/example.com"
        );
    }

    #[test]
    fn fetch_empty_cid_is_error() {
        let gw = PublicIpfsGateway::new();
        let result = gw.fetch("");
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), GatewayError::InvalidCid(_)));
    }

    #[test]
    fn fetch_unreachable_gateway_returns_request_failed() {
        // Point at a port that nothing is listening on.
        let gw = PublicIpfsGateway::with_url("http://127.0.0.1:19999");
        let result = gw.fetch("QmTest123");
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            GatewayError::RequestFailed(_)
        ));
    }

    #[test]
    fn exists_empty_cid_is_error() {
        let gw = PublicIpfsGateway::new();
        assert!(matches!(
            gw.exists("").unwrap_err(),
            GatewayError::InvalidCid(_)
        ));
    }

    #[test]
    fn exists_unreachable_gateway_returns_request_failed() {
        let gw = PublicIpfsGateway::with_url("http://127.0.0.1:19999");
        let result = gw.exists("QmTest123");
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            GatewayError::RequestFailed(_)
        ));
    }

    #[test]
    fn with_max_size() {
        let gw = PublicIpfsGateway::new().with_max_size(1024);
        assert_eq!(gw.max_size, 1024);
    }

    #[test]
    fn url_construction_custom_gateway() {
        let gw = PublicIpfsGateway::with_url("https://dweb.link");
        assert_eq!(
            gw.url_for_cid("bafybeigdyrzt5sfp7udm7hu76uh7y26nf3efuylqabf3oclgtqy55fbzdi"),
            "https://dweb.link/ipfs/bafybeigdyrzt5sfp7udm7hu76uh7y26nf3efuylqabf3oclgtqy55fbzdi"
        );
        assert_eq!(
            gw.url_for_ipns("docs.ipfs.tech"),
            "https://dweb.link/ipns/docs.ipfs.tech"
        );
    }

    #[test]
    fn default_max_size_is_50mib() {
        let gw = PublicIpfsGateway::new();
        assert_eq!(gw.max_size, 50 * 1024 * 1024);
    }

    #[test]
    fn ens_stub_returns_unavailable() {
        let resolver = StubEnsResolver::new();
        let result = resolver.resolve("vitalik.eth");
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            GatewayError::EnsUnavailable(_)
        ));
    }

    #[test]
    fn is_ens_name() {
        assert!(StubEnsResolver::is_ens_name("vitalik.eth"));
        assert!(StubEnsResolver::is_ens_name("app.uniswap.eth"));
        assert!(!StubEnsResolver::is_ens_name(".eth"));
        assert!(!StubEnsResolver::is_ens_name("example.com"));
        assert!(!StubEnsResolver::is_ens_name("has spaces.eth"));
    }

    #[test]
    fn gateway_error_display() {
        let err = GatewayError::NotFound("QmMissing".into());
        assert_eq!(err.to_string(), "content not found: QmMissing");

        let err = GatewayError::ContentTooLarge { size: 100, max: 50 };
        assert!(err.to_string().contains("100"));
        assert!(err.to_string().contains("50"));
    }

    #[test]
    fn fetched_content_serializes() {
        let content = FetchedContent {
            cid: "QmTest".into(),
            gateway_url: "https://ipfs.io/ipfs/QmTest".into(),
            data: vec![1, 2, 3],
            content_type: Some("text/plain".into()),
            size: 3,
        };
        let json = serde_json::to_string(&content).unwrap();
        let restored: FetchedContent = serde_json::from_str(&json).unwrap();
        assert_eq!(restored.cid, "QmTest");
        assert_eq!(restored.data, vec![1, 2, 3]);
    }

    #[test]
    fn gateway_debug() {
        let gw = PublicIpfsGateway::new();
        let debug = format!("{gw:?}");
        assert!(debug.contains("PublicIpfsGateway"));
        assert!(debug.contains("ipfs.io"));
    }

    #[test]
    fn ens_resolver_debug() {
        let resolver = StubEnsResolver::new();
        let debug = format!("{resolver:?}");
        assert!(debug.contains("StubEnsResolver"));
    }
}
