//! ENS (Ethereum Name Service) resolver.
//!
//! Resolves `.eth` names to IPFS content hashes by calling the ENS registry
//! and resolver contracts via Ethereum JSON-RPC (`eth_call`). No full
//! Ethereum client is required — only an RPC endpoint URL.

use sha3::{Digest, Keccak256};

use crate::gateway::GatewayError;

/// The ENS registry contract address (same on mainnet, Goerli, Sepolia).
const ENS_REGISTRY: &str = "0x00000000000C2E074eC69A0dFb2997BA6C7d2e1e";

/// Function selector for `resolver(bytes32)`.
const RESOLVER_SELECTOR: &str = "0178b8bf";

/// Function selector for `contenthash(bytes32)`.
const CONTENTHASH_SELECTOR: &str = "bc1c58d1";

/// Default public Ethereum RPC endpoint.
const DEFAULT_RPC: &str = "https://ethereum.publicnode.com";

/// An ENS resolver that resolves `.eth` names to content hashes via Ethereum
/// JSON-RPC.
///
/// # Example
///
/// ```no_run
/// use nous_browser::ens::EnsResolver;
///
/// let resolver = EnsResolver::new("https://ethereum.publicnode.com");
/// let cid = resolver.resolve("vitalik.eth").unwrap();
/// println!("Content hash: {cid}");
/// ```
#[derive(Debug, Clone)]
pub struct EnsResolver {
    rpc_url: String,
    client: reqwest::blocking::Client,
}

impl EnsResolver {
    /// Create a resolver using a specific Ethereum RPC endpoint.
    pub fn new(rpc_url: impl Into<String>) -> Self {
        Self {
            rpc_url: rpc_url.into(),
            client: reqwest::blocking::Client::new(),
        }
    }

    /// Create a resolver using the default public RPC endpoint.
    pub fn default_rpc() -> Self {
        Self::new(DEFAULT_RPC)
    }

    /// Return the RPC URL this resolver uses.
    pub fn rpc_url(&self) -> &str {
        &self.rpc_url
    }

    /// Check if a name looks like a valid ENS name.
    pub fn is_ens_name(name: &str) -> bool {
        name.ends_with(".eth") && name.len() > 4 && !name.contains(' ')
    }

    /// Resolve an ENS name to an IPFS content hash (CID string).
    ///
    /// Returns the CID as a base58-encoded CIDv0 (`Qm...`) for IPFS content,
    /// or an error if the name has no content hash set.
    pub fn resolve(&self, ens_name: &str) -> Result<String, GatewayError> {
        if !Self::is_ens_name(ens_name) {
            return Err(GatewayError::InvalidCid(format!(
                "'{ens_name}' is not a valid ENS name"
            )));
        }

        // Step 1: Compute namehash
        let node = namehash(ens_name);
        let node_hex = hex::encode(node);

        // Step 2: Get resolver address from ENS registry
        let resolver_addr = self.get_resolver(&node_hex)?;
        if resolver_addr == "0000000000000000000000000000000000000000" {
            return Err(GatewayError::NotFound(format!(
                "no resolver set for '{ens_name}'"
            )));
        }

        // Step 3: Get content hash from the resolver
        let contenthash = self.get_contenthash(&resolver_addr, &node_hex)?;
        if contenthash.is_empty() {
            return Err(GatewayError::NotFound(format!(
                "no content hash set for '{ens_name}'"
            )));
        }

        // Step 4: Decode content hash to CID
        decode_contenthash(&contenthash)
    }

    /// Call `resolver(bytes32)` on the ENS registry.
    fn get_resolver(&self, node_hex: &str) -> Result<String, GatewayError> {
        let data = format!("0x{RESOLVER_SELECTOR}{node_hex}");
        let result = self.eth_call(ENS_REGISTRY, &data)?;
        // Result is a 32-byte ABI-encoded address (last 20 bytes are the address)
        let result = result.strip_prefix("0x").unwrap_or(&result);
        if result.len() < 64 {
            return Err(GatewayError::RequestFailed(
                "invalid resolver response".into(),
            ));
        }
        // Address is the last 40 hex chars of the 64-char word
        Ok(result[24..64].to_string())
    }

    /// Call `contenthash(bytes32)` on the resolver contract.
    fn get_contenthash(
        &self,
        resolver_addr: &str,
        node_hex: &str,
    ) -> Result<Vec<u8>, GatewayError> {
        let data = format!("0x{CONTENTHASH_SELECTOR}{node_hex}");
        let to = format!("0x{resolver_addr}");
        let result = self.eth_call(&to, &data)?;
        let result = result.strip_prefix("0x").unwrap_or(&result);

        // ABI-encoded bytes: first 32 bytes = offset, next 32 bytes = length, then data
        if result.len() < 128 {
            return Ok(Vec::new());
        }

        let length = usize::from_str_radix(&result[64..128], 16).map_err(|_| {
            GatewayError::RequestFailed("invalid contenthash length encoding".into())
        })?;

        if length == 0 {
            return Ok(Vec::new());
        }

        let data_hex = &result[128..];
        let data_len = length.min(data_hex.len() / 2);
        hex::decode(&data_hex[..data_len * 2])
            .map_err(|_| GatewayError::RequestFailed("invalid contenthash hex".into()))
    }

    /// Execute an `eth_call` JSON-RPC request.
    fn eth_call(&self, to: &str, data: &str) -> Result<String, GatewayError> {
        let body = serde_json::json!({
            "jsonrpc": "2.0",
            "method": "eth_call",
            "params": [{
                "to": to,
                "data": data
            }, "latest"],
            "id": 1
        });

        let response = self
            .client
            .post(&self.rpc_url)
            .header("Content-Type", "application/json")
            .json(&body)
            .send()
            .map_err(|e| GatewayError::RequestFailed(format!("RPC request failed: {e}")))?;

        if !response.status().is_success() {
            return Err(GatewayError::RequestFailed(format!(
                "RPC returned HTTP {}",
                response.status()
            )));
        }

        let json: serde_json::Value = response
            .json()
            .map_err(|e| GatewayError::RequestFailed(format!("invalid RPC response: {e}")))?;

        if let Some(error) = json.get("error") {
            return Err(GatewayError::RequestFailed(format!("RPC error: {}", error)));
        }

        json["result"]
            .as_str()
            .map(|s| s.to_string())
            .ok_or_else(|| GatewayError::RequestFailed("missing result in RPC response".into()))
    }
}

impl Default for EnsResolver {
    fn default() -> Self {
        Self::default_rpc()
    }
}

// ── Namehash (EIP-137) ───────────────────────────────────────────────────

/// Compute the ENS namehash per EIP-137.
///
/// `namehash("")` = `[0u8; 32]`
/// `namehash("eth")` = `keccak256(namehash("") ++ keccak256("eth"))`
/// `namehash("vitalik.eth")` = `keccak256(namehash("eth") ++ keccak256("vitalik"))`
pub fn namehash(name: &str) -> [u8; 32] {
    if name.is_empty() {
        return [0u8; 32];
    }

    let mut node = [0u8; 32];
    for label in name.rsplit('.') {
        let label_hash = keccak256(label.as_bytes());
        let mut combined = [0u8; 64];
        combined[..32].copy_from_slice(&node);
        combined[32..].copy_from_slice(&label_hash);
        node = keccak256(&combined);
    }
    node
}

fn keccak256(data: &[u8]) -> [u8; 32] {
    let mut hasher = Keccak256::new();
    hasher.update(data);
    hasher.finalize().into()
}

// ── Content hash decoding (EIP-1577) ────────────────────────────────────

/// Decode an EIP-1577 content hash to a CID string.
///
/// Supports:
/// - IPFS (prefix `0xe3010170`): returns base58-encoded CIDv0 (`Qm...`)
/// - IPNS (prefix `0xe5010172`): returns base36-encoded CIDv1
fn decode_contenthash(bytes: &[u8]) -> Result<String, GatewayError> {
    if bytes.len() < 4 {
        return Err(GatewayError::RequestFailed("content hash too short".into()));
    }

    // IPFS: e3 01 01 70 <multihash>
    if bytes.starts_with(&[0xe3, 0x01, 0x01, 0x70]) {
        let multihash = &bytes[4..];
        if multihash.len() < 2 {
            return Err(GatewayError::RequestFailed("invalid IPFS multihash".into()));
        }
        // Base58-encode the multihash to get CIDv0 (Qm...)
        Ok(bs58::encode(multihash).into_string())
    }
    // IPNS: e5 01 01 72 <peer-id-multihash>
    else if bytes.starts_with(&[0xe5, 0x01, 0x01, 0x72]) {
        let peer_id = &bytes[4..];
        Ok(bs58::encode(peer_id).into_string())
    }
    // Swarm: e4 01 01 1b 20 <32-byte-hash>
    else if bytes.starts_with(&[0xe4]) {
        let hash = &bytes[1..];
        Ok(format!("bzz://{}", hex::encode(hash)))
    } else {
        Err(GatewayError::RequestFailed(format!(
            "unsupported content hash codec: 0x{}",
            hex::encode(&bytes[..bytes.len().min(4)])
        )))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ── Namehash tests ────────────────────────────────────────────────

    #[test]
    fn namehash_empty() {
        assert_eq!(namehash(""), [0u8; 32]);
    }

    #[test]
    fn namehash_eth() {
        // Well-known: namehash("eth") per EIP-137
        let hash = namehash("eth");
        assert_eq!(
            hex::encode(hash),
            "93cdeb708b7545dc668eb9280176169d1c33cfd8ed6f04690a0bcc88a93fc4ae"
        );
    }

    #[test]
    fn namehash_vitalik_eth() {
        let hash = namehash("vitalik.eth");
        // Known value from ENS documentation
        assert_eq!(
            hex::encode(hash),
            "ee6c4522aab0003e8d14cd40a6af439055fd2577951148c14b6cea9a53475835"
        );
    }

    #[test]
    fn namehash_subdomain() {
        let hash = namehash("sub.domain.eth");
        // Just verify it's deterministic and non-zero
        assert_ne!(hash, [0u8; 32]);
        assert_eq!(hash, namehash("sub.domain.eth"));
    }

    #[test]
    fn namehash_different_names_differ() {
        assert_ne!(namehash("alice.eth"), namehash("bob.eth"));
    }

    // ── Content hash decoding tests ──────────────────────────────────

    #[test]
    fn decode_ipfs_contenthash() {
        // IPFS prefix + sha2-256 multihash (hash function 0x12, length 0x20, 32 zero bytes)
        let mut bytes = vec![0xe3, 0x01, 0x01, 0x70, 0x12, 0x20];
        bytes.extend_from_slice(&[0xab; 32]);

        let cid = decode_contenthash(&bytes).unwrap();
        // Should be a base58-encoded CIDv0 starting with Qm
        assert!(cid.starts_with('Q'), "expected base58 CIDv0, got: {cid}");
    }

    #[test]
    fn decode_ipns_contenthash() {
        let mut bytes = vec![0xe5, 0x01, 0x01, 0x72];
        bytes.extend_from_slice(&[0x00, 0x24, 0x08, 0x01]); // mock peer ID prefix
        bytes.extend_from_slice(&[0xcd; 32]);

        let result = decode_contenthash(&bytes).unwrap();
        assert!(!result.is_empty());
    }

    #[test]
    fn decode_swarm_contenthash() {
        let mut bytes = vec![0xe4, 0x01, 0x01, 0x1b, 0x20];
        bytes.extend_from_slice(&[0xff; 32]);

        let result = decode_contenthash(&bytes).unwrap();
        assert!(result.starts_with("bzz://"));
    }

    #[test]
    fn decode_too_short() {
        assert!(decode_contenthash(&[0xe3, 0x01]).is_err());
    }

    #[test]
    fn decode_unknown_codec() {
        assert!(decode_contenthash(&[0x99, 0x01, 0x01, 0x70]).is_err());
    }

    // ── EnsResolver unit tests ───────────────────────────────────────

    #[test]
    fn is_ens_name() {
        assert!(EnsResolver::is_ens_name("vitalik.eth"));
        assert!(EnsResolver::is_ens_name("app.uniswap.eth"));
        assert!(!EnsResolver::is_ens_name(".eth"));
        assert!(!EnsResolver::is_ens_name("example.com"));
        assert!(!EnsResolver::is_ens_name("has spaces.eth"));
        assert!(!EnsResolver::is_ens_name(""));
    }

    #[test]
    fn invalid_name_returns_error() {
        let resolver = EnsResolver::new("http://127.0.0.1:19999");
        assert!(resolver.resolve("notens.com").is_err());
    }

    #[test]
    fn resolver_debug() {
        let resolver = EnsResolver::new("https://ethereum.publicnode.com");
        let debug = format!("{resolver:?}");
        assert!(debug.contains("EnsResolver"));
        assert!(debug.contains("publicnode"));
    }

    #[test]
    fn resolver_rpc_url() {
        let resolver = EnsResolver::new("https://example.com");
        assert_eq!(resolver.rpc_url(), "https://example.com");
    }

    #[test]
    fn default_resolver_uses_public_rpc() {
        let resolver = EnsResolver::default();
        assert!(resolver.rpc_url().contains("publicnode"));
    }

    #[test]
    fn unreachable_rpc_returns_error() {
        let resolver = EnsResolver::new("http://127.0.0.1:19999");
        let result = resolver.resolve("vitalik.eth");
        assert!(result.is_err());
        match result.unwrap_err() {
            GatewayError::RequestFailed(msg) => {
                assert!(msg.contains("RPC request failed"), "unexpected: {msg}");
            }
            other => panic!("expected RequestFailed, got: {other:?}"),
        }
    }

    // ── keccak256 test ───────────────────────────────────────────────

    #[test]
    fn keccak256_empty() {
        let hash = keccak256(b"");
        assert_eq!(
            hex::encode(hash),
            "c5d2460186f7233c927e7db2dcc703c0e500b653ca82273b7bfad8045d85a470"
        );
    }
}
