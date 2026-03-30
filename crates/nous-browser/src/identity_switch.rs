use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SiteIdentity {
    pub domain: String,
    pub did: String,
    pub label: Option<String>,
}

/// Routes different domains to different DIDs for per-site identity isolation.
///
/// Supports exact domain matching, wildcard patterns (`*.example.com`),
/// and subdomain inheritance (settings for `example.com` apply to
/// `sub.example.com` unless overridden).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IdentityRouter {
    default_did: String,
    site_map: HashMap<String, String>,
    labels: HashMap<String, String>,
    /// Wildcard patterns: `*.example.com` → DID
    wildcards: Vec<(String, String)>,
    /// Whether subdomains inherit their parent domain's identity.
    subdomain_inheritance: bool,
}

impl IdentityRouter {
    pub fn new(default_did: impl Into<String>) -> Self {
        Self {
            default_did: default_did.into(),
            site_map: HashMap::new(),
            labels: HashMap::new(),
            wildcards: Vec::new(),
            subdomain_inheritance: true,
        }
    }

    /// Set whether subdomains inherit their parent domain's identity.
    pub fn set_subdomain_inheritance(&mut self, enabled: bool) {
        self.subdomain_inheritance = enabled;
    }

    pub fn set_identity(&mut self, domain: &str, did: &str, label: Option<&str>) {
        if domain.starts_with("*.") {
            // Wildcard pattern — store separately
            self.wildcards.retain(|(pattern, _)| pattern != domain);
            self.wildcards.push((domain.to_string(), did.to_string()));
        } else {
            self.site_map.insert(domain.to_string(), did.to_string());
        }
        if let Some(l) = label {
            self.labels.insert(did.to_string(), l.to_string());
        }
    }

    pub fn remove_identity(&mut self, domain: &str) -> bool {
        if domain.starts_with("*.") {
            let before = self.wildcards.len();
            self.wildcards.retain(|(pattern, _)| pattern != domain);
            self.wildcards.len() < before
        } else {
            self.site_map.remove(domain).is_some()
        }
    }

    /// Resolve the DID for a domain.
    ///
    /// Resolution order:
    /// 1. Exact match in `site_map`
    /// 2. Wildcard patterns (most specific first)
    /// 3. Subdomain inheritance (if enabled) — walk up the domain hierarchy
    /// 4. Default DID
    pub fn resolve(&self, domain: &str) -> &str {
        // 1. Exact match
        if let Some(did) = self.site_map.get(domain) {
            return did;
        }

        // 2. Wildcard patterns (longest suffix match = most specific)
        if let Some(did) = self.match_wildcard(domain) {
            return did;
        }

        // 3. Subdomain inheritance
        if self.subdomain_inheritance
            && let Some(did) = self.inherit_from_parent(domain)
        {
            return did;
        }

        &self.default_did
    }

    /// Check if a domain has an explicit (non-default, non-inherited) identity.
    pub fn has_explicit_identity(&self, domain: &str) -> bool {
        self.site_map.contains_key(domain)
    }

    pub fn label_for(&self, did: &str) -> Option<&str> {
        self.labels.get(did).map(|s| s.as_str())
    }

    pub fn all_identities(&self) -> Vec<SiteIdentity> {
        let mut identities: Vec<SiteIdentity> = self
            .site_map
            .iter()
            .map(|(domain, did)| SiteIdentity {
                domain: domain.clone(),
                did: did.clone(),
                label: self.labels.get(did).cloned(),
            })
            .collect();

        // Include wildcard patterns
        for (pattern, did) in &self.wildcards {
            identities.push(SiteIdentity {
                domain: pattern.clone(),
                did: did.clone(),
                label: self.labels.get(did).cloned(),
            });
        }

        identities
    }

    pub fn default_did(&self) -> &str {
        &self.default_did
    }

    pub fn set_default(&mut self, did: impl Into<String>) {
        self.default_did = did.into();
    }

    pub fn unique_dids(&self) -> Vec<&str> {
        let mut dids: Vec<&str> = self.site_map.values().map(|s| s.as_str()).collect();
        for (_, did) in &self.wildcards {
            dids.push(did);
        }
        dids.push(&self.default_did);
        dids.sort();
        dids.dedup();
        dids
    }

    /// Number of explicit domain mappings (exact + wildcard).
    pub fn mapping_count(&self) -> usize {
        self.site_map.len() + self.wildcards.len()
    }

    /// Find all domains mapped to a specific DID.
    pub fn domains_for_did(&self, did: &str) -> Vec<String> {
        let mut domains: Vec<String> = self
            .site_map
            .iter()
            .filter(|(_, v)| v.as_str() == did)
            .map(|(k, _)| k.clone())
            .collect();

        for (pattern, d) in &self.wildcards {
            if d == did {
                domains.push(pattern.clone());
            }
        }

        domains.sort();
        domains
    }

    /// Match a domain against wildcard patterns.
    /// Returns the DID from the most specific (longest suffix) matching pattern.
    fn match_wildcard(&self, domain: &str) -> Option<&str> {
        let mut best_match: Option<(&str, usize)> = None;

        for (pattern, did) in &self.wildcards {
            if let Some(suffix) = pattern.strip_prefix("*.")
                && (domain == suffix || domain.ends_with(&format!(".{suffix}")))
            {
                let specificity = suffix.len();
                if best_match.is_none_or(|(_, s)| specificity > s) {
                    best_match = Some((did.as_str(), specificity));
                }
            }
        }

        best_match.map(|(did, _)| did)
    }

    /// Walk up the domain hierarchy looking for a parent with an explicit identity.
    fn inherit_from_parent(&self, domain: &str) -> Option<&str> {
        let mut current = domain;
        loop {
            let dot_pos = current.find('.')?;

            let parent = &current[dot_pos + 1..];
            if parent.is_empty() || !parent.contains('.') {
                // Reached TLD — stop
                return None;
            }

            if let Some(did) = self.site_map.get(parent) {
                return Some(did);
            }

            current = parent;
        }
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

    // --- Wildcard matching ---

    #[test]
    fn wildcard_matches_subdomain() {
        let mut router = IdentityRouter::new("did:key:default");
        router.set_identity("*.google.com", "did:key:anon", Some("Anonymous"));

        assert_eq!(router.resolve("mail.google.com"), "did:key:anon");
        assert_eq!(router.resolve("drive.google.com"), "did:key:anon");
        assert_eq!(router.resolve("google.com"), "did:key:anon");
        assert_eq!(router.resolve("other.com"), "did:key:default");
    }

    #[test]
    fn wildcard_most_specific_wins() {
        let mut router = IdentityRouter::new("did:key:default");
        router.set_identity("*.google.com", "did:key:broad", None);
        router.set_identity("*.docs.google.com", "did:key:specific", None);

        // More specific wildcard wins
        assert_eq!(router.resolve("editor.docs.google.com"), "did:key:specific");
        // Broader wildcard for other subdomains
        assert_eq!(router.resolve("mail.google.com"), "did:key:broad");
    }

    #[test]
    fn exact_overrides_wildcard() {
        let mut router = IdentityRouter::new("did:key:default");
        router.set_identity("*.google.com", "did:key:anon", None);
        router.set_identity("mail.google.com", "did:key:mail", None);

        // Exact match takes priority over wildcard
        assert_eq!(router.resolve("mail.google.com"), "did:key:mail");
        assert_eq!(router.resolve("drive.google.com"), "did:key:anon");
    }

    #[test]
    fn remove_wildcard() {
        let mut router = IdentityRouter::new("did:key:default");
        router.set_identity("*.example.com", "did:key:anon", None);

        assert_eq!(router.resolve("sub.example.com"), "did:key:anon");
        assert!(router.remove_identity("*.example.com"));
        assert_eq!(router.resolve("sub.example.com"), "did:key:default");
    }

    #[test]
    fn remove_nonexistent_wildcard() {
        let mut router = IdentityRouter::new("did:key:default");
        assert!(!router.remove_identity("*.nothing.com"));
    }

    // --- Subdomain inheritance ---

    #[test]
    fn subdomain_inherits_parent() {
        let mut router = IdentityRouter::new("did:key:default");
        router.set_identity("example.com", "did:key:example", None);

        // Subdomain inherits parent identity
        assert_eq!(router.resolve("sub.example.com"), "did:key:example");
        assert_eq!(router.resolve("deep.sub.example.com"), "did:key:example");
    }

    #[test]
    fn subdomain_inheritance_disabled() {
        let mut router = IdentityRouter::new("did:key:default");
        router.set_subdomain_inheritance(false);
        router.set_identity("example.com", "did:key:example", None);

        // Without inheritance, subdomains get default
        assert_eq!(router.resolve("sub.example.com"), "did:key:default");
    }

    #[test]
    fn subdomain_explicit_overrides_inheritance() {
        let mut router = IdentityRouter::new("did:key:default");
        router.set_identity("example.com", "did:key:parent", None);
        router.set_identity("sub.example.com", "did:key:child", None);

        // Explicit mapping takes priority
        assert_eq!(router.resolve("sub.example.com"), "did:key:child");
        assert_eq!(router.resolve("other.example.com"), "did:key:parent");
    }

    #[test]
    fn no_inheritance_from_tld() {
        let mut router = IdentityRouter::new("did:key:default");
        // Even with inheritance, don't inherit from bare TLD
        router.set_identity("com", "did:key:com", None);
        assert_eq!(router.resolve("example.com"), "did:key:default");
    }

    // --- Utility methods ---

    #[test]
    fn mapping_count() {
        let mut router = IdentityRouter::new("did:key:default");
        assert_eq!(router.mapping_count(), 0);

        router.set_identity("a.com", "did:key:a", None);
        router.set_identity("*.b.com", "did:key:b", None);
        assert_eq!(router.mapping_count(), 2);
    }

    #[test]
    fn has_explicit_identity() {
        let mut router = IdentityRouter::new("did:key:default");
        router.set_identity("explicit.com", "did:key:x", None);

        assert!(router.has_explicit_identity("explicit.com"));
        assert!(!router.has_explicit_identity("other.com"));
    }

    #[test]
    fn domains_for_did() {
        let mut router = IdentityRouter::new("did:key:default");
        router.set_identity("a.com", "did:key:shared", None);
        router.set_identity("b.com", "did:key:shared", None);
        router.set_identity("c.com", "did:key:other", None);
        router.set_identity("*.d.com", "did:key:shared", None);

        let domains = router.domains_for_did("did:key:shared");
        assert_eq!(domains.len(), 3);
        assert!(domains.contains(&"a.com".to_string()));
        assert!(domains.contains(&"b.com".to_string()));
        assert!(domains.contains(&"*.d.com".to_string()));
    }

    #[test]
    fn wildcard_in_all_identities() {
        let mut router = IdentityRouter::new("did:key:default");
        router.set_identity("a.com", "did:key:a", None);
        router.set_identity("*.b.com", "did:key:b", None);

        let all = router.all_identities();
        assert_eq!(all.len(), 2);
        assert!(all.iter().any(|i| i.domain == "*.b.com"));
    }

    #[test]
    fn wildcard_in_unique_dids() {
        let mut router = IdentityRouter::new("did:key:default");
        router.set_identity("*.example.com", "did:key:wild", None);

        let dids = router.unique_dids();
        assert!(dids.contains(&"did:key:wild"));
        assert!(dids.contains(&"did:key:default"));
    }

    #[test]
    fn overwrite_wildcard() {
        let mut router = IdentityRouter::new("did:key:default");
        router.set_identity("*.example.com", "did:key:old", None);
        router.set_identity("*.example.com", "did:key:new", None);

        assert_eq!(router.resolve("sub.example.com"), "did:key:new");
        // Should not have duplicate wildcard entries
        assert_eq!(router.wildcards.len(), 1);
    }

    #[test]
    fn serializes_with_wildcards() {
        let mut router = IdentityRouter::new("did:key:default");
        router.set_identity("*.google.com", "did:key:anon", Some("Anon"));
        router.set_identity("github.com", "did:key:dev", None);

        let json = serde_json::to_string(&router).unwrap();
        let restored: IdentityRouter = serde_json::from_str(&json).unwrap();

        assert_eq!(restored.resolve("mail.google.com"), "did:key:anon");
        assert_eq!(restored.resolve("github.com"), "did:key:dev");
    }

    #[test]
    fn complex_resolution_priority() {
        let mut router = IdentityRouter::new("did:key:default");
        router.set_identity("example.com", "did:key:parent", None);
        router.set_identity("*.example.com", "did:key:wildcard", None);
        router.set_identity("special.example.com", "did:key:exact", None);

        // Exact match > wildcard > inheritance > default
        assert_eq!(router.resolve("special.example.com"), "did:key:exact");
        assert_eq!(router.resolve("other.example.com"), "did:key:wildcard");
        assert_eq!(router.resolve("example.com"), "did:key:parent");
        assert_eq!(router.resolve("unrelated.com"), "did:key:default");
    }
}
