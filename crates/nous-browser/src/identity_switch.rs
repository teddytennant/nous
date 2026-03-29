use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SiteIdentity {
    pub domain: String,
    pub did: String,
    pub label: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IdentityRouter {
    default_did: String,
    site_map: HashMap<String, String>,
    labels: HashMap<String, String>,
}

impl IdentityRouter {
    pub fn new(default_did: impl Into<String>) -> Self {
        Self {
            default_did: default_did.into(),
            site_map: HashMap::new(),
            labels: HashMap::new(),
        }
    }

    pub fn set_identity(&mut self, domain: &str, did: &str, label: Option<&str>) {
        self.site_map.insert(domain.to_string(), did.to_string());
        if let Some(l) = label {
            self.labels.insert(did.to_string(), l.to_string());
        }
    }

    pub fn remove_identity(&mut self, domain: &str) -> bool {
        self.site_map.remove(domain).is_some()
    }

    pub fn resolve(&self, domain: &str) -> &str {
        self.site_map
            .get(domain)
            .map(|s| s.as_str())
            .unwrap_or(&self.default_did)
    }

    pub fn label_for(&self, did: &str) -> Option<&str> {
        self.labels.get(did).map(|s| s.as_str())
    }

    pub fn all_identities(&self) -> Vec<SiteIdentity> {
        self.site_map
            .iter()
            .map(|(domain, did)| SiteIdentity {
                domain: domain.clone(),
                did: did.clone(),
                label: self.labels.get(did).cloned(),
            })
            .collect()
    }

    pub fn default_did(&self) -> &str {
        &self.default_did
    }

    pub fn set_default(&mut self, did: impl Into<String>) {
        self.default_did = did.into();
    }

    pub fn unique_dids(&self) -> Vec<&str> {
        let mut dids: Vec<&str> = self.site_map.values().map(|s| s.as_str()).collect();
        dids.push(&self.default_did);
        dids.sort();
        dids.dedup();
        dids
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_identity() {
        let router = IdentityRouter::new("did:key:default");
        assert_eq!(router.resolve("example.com"), "did:key:default");
    }

    #[test]
    fn per_site_identity() {
        let mut router = IdentityRouter::new("did:key:default");
        router.set_identity("twitter.com", "did:key:anon", Some("Anonymous"));
        router.set_identity("github.com", "did:key:dev", Some("Developer"));

        assert_eq!(router.resolve("twitter.com"), "did:key:anon");
        assert_eq!(router.resolve("github.com"), "did:key:dev");
        assert_eq!(router.resolve("other.com"), "did:key:default");
    }

    #[test]
    fn remove_identity() {
        let mut router = IdentityRouter::new("did:key:default");
        router.set_identity("site.com", "did:key:custom", None);
        assert_eq!(router.resolve("site.com"), "did:key:custom");

        assert!(router.remove_identity("site.com"));
        assert_eq!(router.resolve("site.com"), "did:key:default");
    }

    #[test]
    fn remove_nonexistent() {
        let mut router = IdentityRouter::new("did:key:default");
        assert!(!router.remove_identity("nothing.com"));
    }

    #[test]
    fn labels() {
        let mut router = IdentityRouter::new("did:key:default");
        router.set_identity("site.com", "did:key:anon", Some("Anonymous"));
        assert_eq!(router.label_for("did:key:anon"), Some("Anonymous"));
        assert_eq!(router.label_for("did:key:other"), None);
    }

    #[test]
    fn all_identities() {
        let mut router = IdentityRouter::new("did:key:default");
        router.set_identity("a.com", "did:key:a", None);
        router.set_identity("b.com", "did:key:b", None);
        assert_eq!(router.all_identities().len(), 2);
    }

    #[test]
    fn unique_dids() {
        let mut router = IdentityRouter::new("did:key:default");
        router.set_identity("a.com", "did:key:a", None);
        router.set_identity("b.com", "did:key:a", None); // same DID
        let dids = router.unique_dids();
        assert_eq!(dids.len(), 2); // default + did:key:a
    }

    #[test]
    fn set_default() {
        let mut router = IdentityRouter::new("did:key:old");
        router.set_default("did:key:new");
        assert_eq!(router.default_did(), "did:key:new");
    }

    #[test]
    fn serializes() {
        let mut router = IdentityRouter::new("did:key:default");
        router.set_identity("site.com", "did:key:anon", Some("Anon"));
        let json = serde_json::to_string(&router).unwrap();
        let restored: IdentityRouter = serde_json::from_str(&json).unwrap();
        assert_eq!(restored.resolve("site.com"), "did:key:anon");
    }
}
