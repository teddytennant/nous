use nous_core::Error;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResolvedUrl {
    pub original: String,
    pub protocol: Protocol,
    pub resolved: String,
    pub content_type: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Protocol {
    Http,
    Https,
    Ipfs,
    Ipns,
    Arweave,
    Ens,
    Did,
    Nous,
}

impl Protocol {
    pub fn from_url(url: &str) -> Self {
        if url.starts_with("ipfs://") {
            Self::Ipfs
        } else if url.starts_with("ipns://") {
            Self::Ipns
        } else if url.starts_with("ar://") {
            Self::Arweave
        } else if url.ends_with(".eth") || url.ends_with(".eth/") || url.contains(".eth/") {
            Self::Ens
        } else if url.starts_with("did:") {
            Self::Did
        } else if url.starts_with("nous://") {
            Self::Nous
        } else if url.starts_with("https://") {
            Self::Https
        } else {
            Self::Http
        }
    }

    pub fn is_decentralized(&self) -> bool {
        matches!(
            self,
            Self::Ipfs | Self::Ipns | Self::Arweave | Self::Ens | Self::Did | Self::Nous
        )
    }
}

pub struct UrlResolver {
    ipfs_gateway: String,
    ens_cache: HashMap<String, String>,
}

impl UrlResolver {
    pub fn new(ipfs_gateway: &str) -> Self {
        Self {
            ipfs_gateway: ipfs_gateway.to_string(),
            ens_cache: HashMap::new(),
        }
    }

    pub fn resolve(&self, url: &str) -> Result<ResolvedUrl, Error> {
        let protocol = Protocol::from_url(url);

        let resolved = match protocol {
            Protocol::Ipfs => {
                let cid = url
                    .strip_prefix("ipfs://")
                    .ok_or_else(|| Error::InvalidInput("invalid IPFS URL".into()))?;
                format!("{}/ipfs/{}", self.ipfs_gateway, cid)
            }
            Protocol::Ipns => {
                let name = url
                    .strip_prefix("ipns://")
                    .ok_or_else(|| Error::InvalidInput("invalid IPNS URL".into()))?;
                format!("{}/ipns/{}", self.ipfs_gateway, name)
            }
            Protocol::Arweave => {
                let tx_id = url
                    .strip_prefix("ar://")
                    .ok_or_else(|| Error::InvalidInput("invalid Arweave URL".into()))?;
                format!("https://arweave.net/{tx_id}")
            }
            Protocol::Ens => {
                let name = url.trim_end_matches('/');
                if let Some(resolved) = self.ens_cache.get(name) {
                    resolved.clone()
                } else {
                    format!("{}/ipns/{name}", self.ipfs_gateway)
                }
            }
            Protocol::Nous => {
                let path = url
                    .strip_prefix("nous://")
                    .ok_or_else(|| Error::InvalidInput("invalid nous URL".into()))?;
                format!("local://{path}")
            }
            _ => url.to_string(),
        };

        Ok(ResolvedUrl {
            original: url.to_string(),
            protocol,
            resolved,
            content_type: None,
        })
    }

    pub fn cache_ens(&mut self, name: &str, resolved: &str) {
        self.ens_cache
            .insert(name.to_string(), resolved.to_string());
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn resolver() -> UrlResolver {
        UrlResolver::new("http://localhost:8080")
    }

    #[test]
    fn detect_ipfs_protocol() {
        assert_eq!(Protocol::from_url("ipfs://QmTest"), Protocol::Ipfs);
    }

    #[test]
    fn detect_ens_protocol() {
        assert_eq!(Protocol::from_url("vitalik.eth"), Protocol::Ens);
        assert_eq!(Protocol::from_url("app.uniswap.eth/"), Protocol::Ens);
    }

    #[test]
    fn detect_arweave_protocol() {
        assert_eq!(Protocol::from_url("ar://txid123"), Protocol::Arweave);
    }

    #[test]
    fn detect_https() {
        assert_eq!(Protocol::from_url("https://example.com"), Protocol::Https);
    }

    #[test]
    fn detect_nous_protocol() {
        assert_eq!(Protocol::from_url("nous://feed"), Protocol::Nous);
    }

    #[test]
    fn decentralized_protocols() {
        assert!(Protocol::Ipfs.is_decentralized());
        assert!(Protocol::Ens.is_decentralized());
        assert!(!Protocol::Https.is_decentralized());
    }

    #[test]
    fn resolve_ipfs() {
        let r = resolver();
        let result = r.resolve("ipfs://QmTest123").unwrap();
        assert_eq!(result.resolved, "http://localhost:8080/ipfs/QmTest123");
    }

    #[test]
    fn resolve_ipns() {
        let r = resolver();
        let result = r.resolve("ipns://example.com").unwrap();
        assert_eq!(result.resolved, "http://localhost:8080/ipns/example.com");
    }

    #[test]
    fn resolve_arweave() {
        let r = resolver();
        let result = r.resolve("ar://txid123").unwrap();
        assert_eq!(result.resolved, "https://arweave.net/txid123");
    }

    #[test]
    fn resolve_ens_with_cache() {
        let mut r = resolver();
        r.cache_ens("vitalik.eth", "ipfs://QmVitalik");
        let result = r.resolve("vitalik.eth").unwrap();
        assert_eq!(result.resolved, "ipfs://QmVitalik");
    }

    #[test]
    fn resolve_https_passthrough() {
        let r = resolver();
        let result = r.resolve("https://example.com").unwrap();
        assert_eq!(result.resolved, "https://example.com");
    }

    #[test]
    fn resolved_url_serializes() {
        let r = resolver();
        let result = r.resolve("ipfs://QmTest").unwrap();
        let json = serde_json::to_string(&result).unwrap();
        let _: ResolvedUrl = serde_json::from_str(&json).unwrap();
    }
}
