//! Content Security Policy engine for dApp sandboxing.
//!
//! Parses and evaluates CSP headers to enforce resource loading policies
//! for decentralized applications. Supports standard CSP directives and
//! extends them with Nous-specific source expressions for IPFS, IPNS,
//! and Arweave origins.

use serde::{Deserialize, Serialize};

/// A CSP source expression.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum CspSource {
    /// `'none'` — block all sources.
    None,
    /// `'self'` — same origin only.
    SelfOrigin,
    /// `'unsafe-inline'` — allow inline scripts/styles.
    UnsafeInline,
    /// `'unsafe-eval'` — allow eval().
    UnsafeEval,
    /// A specific host: `example.com`, `*.example.com`.
    Host(String),
    /// A scheme: `https:`, `ipfs:`, `data:`.
    Scheme(String),
    /// A nonce: `'nonce-abc123'`.
    Nonce(String),
    /// A hash: `'sha256-...'`.
    Hash { algorithm: String, value: String },
}

impl CspSource {
    /// Parse a single source expression from a CSP header value.
    pub fn parse(token: &str) -> Self {
        let lower = token.to_lowercase();

        if lower == "'none'" {
            Self::None
        } else if lower == "'self'" {
            Self::SelfOrigin
        } else if lower == "'unsafe-inline'" {
            Self::UnsafeInline
        } else if lower == "'unsafe-eval'" {
            Self::UnsafeEval
        } else if let Some(nonce) = lower
            .strip_prefix("'nonce-")
            .and_then(|s| s.strip_suffix('\''))
        {
            Self::Nonce(nonce.to_string())
        } else if let Some(hash_str) = lower
            .strip_prefix("'sha256-")
            .and_then(|s| s.strip_suffix('\''))
        {
            Self::Hash {
                algorithm: "sha256".to_string(),
                value: hash_str.to_string(),
            }
        } else if let Some(hash_str) = lower
            .strip_prefix("'sha384-")
            .and_then(|s| s.strip_suffix('\''))
        {
            Self::Hash {
                algorithm: "sha384".to_string(),
                value: hash_str.to_string(),
            }
        } else if let Some(hash_str) = lower
            .strip_prefix("'sha512-")
            .and_then(|s| s.strip_suffix('\''))
        {
            Self::Hash {
                algorithm: "sha512".to_string(),
                value: hash_str.to_string(),
            }
        } else if lower.ends_with(':') {
            Self::Scheme(lower.trim_end_matches(':').to_string())
        } else {
            Self::Host(lower)
        }
    }

    /// Check if a URL matches this source expression.
    pub fn matches(&self, url: &str, page_origin: &str) -> bool {
        let lower = url.to_lowercase();

        match self {
            Self::None => false,
            Self::SelfOrigin => {
                let origin = extract_origin(&lower);
                origin == page_origin.to_lowercase()
            }
            Self::UnsafeInline | Self::UnsafeEval => {
                // These control inline/eval, not URL-based resources.
                // Always return false for URL matching — they're checked
                // separately by the policy evaluator.
                false
            }
            Self::Host(pattern) => host_matches(pattern, &lower),
            Self::Scheme(scheme) => lower.starts_with(&format!("{scheme}:")),
            Self::Nonce(_) | Self::Hash { .. } => {
                // Nonce/hash matching requires the actual script content,
                // not the URL. Return false for URL-based checks.
                false
            }
        }
    }
}

/// A CSP directive (e.g., `script-src`, `default-src`).
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum CspDirective {
    DefaultSrc,
    ScriptSrc,
    StyleSrc,
    ImgSrc,
    ConnectSrc,
    FontSrc,
    FrameSrc,
    MediaSrc,
    ObjectSrc,
    WorkerSrc,
    ChildSrc,
    FormAction,
    BaseUri,
    FrameAncestors,
}

impl CspDirective {
    /// Parse a directive name from a CSP header.
    pub fn parse(name: &str) -> Option<Self> {
        match name.to_lowercase().as_str() {
            "default-src" => Some(Self::DefaultSrc),
            "script-src" => Some(Self::ScriptSrc),
            "style-src" => Some(Self::StyleSrc),
            "img-src" => Some(Self::ImgSrc),
            "connect-src" => Some(Self::ConnectSrc),
            "font-src" => Some(Self::FontSrc),
            "frame-src" => Some(Self::FrameSrc),
            "media-src" => Some(Self::MediaSrc),
            "object-src" => Some(Self::ObjectSrc),
            "worker-src" => Some(Self::WorkerSrc),
            "child-src" => Some(Self::ChildSrc),
            "form-action" => Some(Self::FormAction),
            "base-uri" => Some(Self::BaseUri),
            "frame-ancestors" => Some(Self::FrameAncestors),
            _ => None,
        }
    }

    /// The directive name as it appears in a CSP header.
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::DefaultSrc => "default-src",
            Self::ScriptSrc => "script-src",
            Self::StyleSrc => "style-src",
            Self::ImgSrc => "img-src",
            Self::ConnectSrc => "connect-src",
            Self::FontSrc => "font-src",
            Self::FrameSrc => "frame-src",
            Self::MediaSrc => "media-src",
            Self::ObjectSrc => "object-src",
            Self::WorkerSrc => "worker-src",
            Self::ChildSrc => "child-src",
            Self::FormAction => "form-action",
            Self::BaseUri => "base-uri",
            Self::FrameAncestors => "frame-ancestors",
        }
    }

    /// The fallback directive if this one isn't specified.
    /// Most directives fall back to `default-src`.
    pub fn fallback(&self) -> Option<Self> {
        match self {
            Self::DefaultSrc => None,
            Self::FrameAncestors | Self::FormAction | Self::BaseUri => None,
            _ => Some(Self::DefaultSrc),
        }
    }
}

/// A parsed Content Security Policy.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CspPolicy {
    directives: Vec<(CspDirective, Vec<CspSource>)>,
    report_only: bool,
}

impl CspPolicy {
    /// Parse a CSP header value into a policy.
    ///
    /// Format: `directive-name source1 source2; directive-name source3`
    pub fn parse(header: &str) -> Self {
        let mut directives = Vec::new();

        for part in header.split(';') {
            let trimmed = part.trim();
            if trimmed.is_empty() {
                continue;
            }

            let mut tokens = trimmed.split_whitespace();
            let directive_name = match tokens.next() {
                Some(name) => name,
                None => continue,
            };

            if let Some(directive) = CspDirective::parse(directive_name) {
                let sources: Vec<CspSource> = tokens.map(CspSource::parse).collect();
                directives.push((directive, sources));
            }
        }

        Self {
            directives,
            report_only: false,
        }
    }

    /// Create a strict default policy for dApps.
    ///
    /// Blocks everything except same-origin and IPFS/IPNS resources.
    pub fn dapp_default() -> Self {
        Self {
            directives: vec![
                (
                    CspDirective::DefaultSrc,
                    vec![CspSource::SelfOrigin, CspSource::Scheme("ipfs".into())],
                ),
                (CspDirective::ScriptSrc, vec![CspSource::SelfOrigin]),
                (
                    CspDirective::StyleSrc,
                    vec![CspSource::SelfOrigin, CspSource::UnsafeInline],
                ),
                (
                    CspDirective::ImgSrc,
                    vec![
                        CspSource::SelfOrigin,
                        CspSource::Scheme("ipfs".into()),
                        CspSource::Scheme("data".into()),
                    ],
                ),
                (
                    CspDirective::ConnectSrc,
                    vec![
                        CspSource::SelfOrigin,
                        CspSource::Scheme("ipfs".into()),
                        CspSource::Scheme("ipns".into()),
                        CspSource::Scheme("wss".into()),
                    ],
                ),
                (CspDirective::ObjectSrc, vec![CspSource::None]),
                (CspDirective::FrameAncestors, vec![CspSource::None]),
            ],
            report_only: false,
        }
    }

    /// Set whether this policy is report-only (violations are logged but not blocked).
    pub fn set_report_only(&mut self, report_only: bool) {
        self.report_only = report_only;
    }

    /// Whether this policy is report-only.
    pub fn is_report_only(&self) -> bool {
        self.report_only
    }

    /// Get the sources for a specific directive.
    /// Falls back to `default-src` if the directive isn't specified.
    pub fn sources_for(&self, directive: &CspDirective) -> Option<&[CspSource]> {
        // Check for the exact directive first
        for (d, sources) in &self.directives {
            if d == directive {
                return Some(sources);
            }
        }

        // Fall back
        if let Some(fallback) = directive.fallback() {
            for (d, sources) in &self.directives {
                if *d == fallback {
                    return Some(sources);
                }
            }
        }

        None
    }

    /// Check if a resource URL is allowed by the policy for a given directive.
    pub fn allows(&self, directive: &CspDirective, url: &str, page_origin: &str) -> bool {
        let sources = match self.sources_for(directive) {
            Some(sources) => sources,
            None => return true, // No directive = allow
        };

        // If 'none' is present, block everything
        if sources.contains(&CspSource::None) {
            return false;
        }

        // Check if any source matches
        sources
            .iter()
            .any(|source| source.matches(url, page_origin))
    }

    /// Evaluate a resource load and return a violation if blocked.
    pub fn evaluate(
        &self,
        directive: &CspDirective,
        url: &str,
        page_origin: &str,
    ) -> Option<CspViolation> {
        if self.allows(directive, url, page_origin) {
            return None;
        }

        Some(CspViolation {
            directive: directive.clone(),
            blocked_url: url.to_string(),
            page_origin: page_origin.to_string(),
            report_only: self.report_only,
        })
    }

    /// Check if inline scripts/styles are allowed.
    pub fn allows_inline(&self, directive: &CspDirective) -> bool {
        let sources = match self.sources_for(directive) {
            Some(sources) => sources,
            None => return true,
        };

        sources.contains(&CspSource::UnsafeInline)
    }

    /// Check if eval() is allowed.
    pub fn allows_eval(&self) -> bool {
        let sources = match self.sources_for(&CspDirective::ScriptSrc) {
            Some(sources) => sources,
            None => return true,
        };

        sources.contains(&CspSource::UnsafeEval)
    }

    /// Check if a nonce is present in the policy.
    pub fn has_nonce(&self, directive: &CspDirective, nonce: &str) -> bool {
        let sources = match self.sources_for(directive) {
            Some(sources) => sources,
            None => return false,
        };

        sources
            .iter()
            .any(|s| matches!(s, CspSource::Nonce(n) if n == nonce))
    }

    /// Serialize the policy back to a CSP header value.
    pub fn to_header(&self) -> String {
        self.directives
            .iter()
            .map(|(directive, sources)| {
                let sources_str: Vec<String> = sources
                    .iter()
                    .map(|s| match s {
                        CspSource::None => "'none'".to_string(),
                        CspSource::SelfOrigin => "'self'".to_string(),
                        CspSource::UnsafeInline => "'unsafe-inline'".to_string(),
                        CspSource::UnsafeEval => "'unsafe-eval'".to_string(),
                        CspSource::Host(h) => h.clone(),
                        CspSource::Scheme(s) => format!("{s}:"),
                        CspSource::Nonce(n) => format!("'nonce-{n}'"),
                        CspSource::Hash { algorithm, value } => {
                            format!("'{algorithm}-{value}'")
                        }
                    })
                    .collect();

                format!("{} {}", directive.as_str(), sources_str.join(" "))
            })
            .collect::<Vec<_>>()
            .join("; ")
    }

    /// Number of directives in the policy.
    pub fn directive_count(&self) -> usize {
        self.directives.len()
    }
}

/// A CSP violation report.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CspViolation {
    pub directive: CspDirective,
    pub blocked_url: String,
    pub page_origin: String,
    pub report_only: bool,
}

impl CspViolation {
    /// Human-readable description of the violation.
    pub fn description(&self) -> String {
        let mode = if self.report_only {
            "would block"
        } else {
            "blocked"
        };
        format!(
            "CSP {}: '{}' violates {} for origin '{}'",
            mode,
            self.blocked_url,
            self.directive.as_str(),
            self.page_origin
        )
    }
}

/// Extract the origin (scheme + host) from a URL.
fn extract_origin(url: &str) -> String {
    // Find scheme
    if let Some(scheme_end) = url.find("://") {
        let after_scheme = &url[scheme_end + 3..];
        let host_end = after_scheme
            .find('/')
            .unwrap_or(after_scheme.len())
            .min(after_scheme.find('?').unwrap_or(after_scheme.len()));
        let host = &after_scheme[..host_end];
        format!("{}://{}", &url[..scheme_end], host)
    } else {
        url.to_string()
    }
}

/// Check if a host pattern matches a URL.
fn host_matches(pattern: &str, url: &str) -> bool {
    let host = extract_host(url);

    if let Some(suffix) = pattern.strip_prefix("*.") {
        // Wildcard: *.example.com matches sub.example.com and example.com
        host == suffix || host.ends_with(&format!(".{suffix}"))
    } else {
        host == pattern
    }
}

/// Extract the hostname from a URL.
fn extract_host(url: &str) -> String {
    if let Some(scheme_end) = url.find("://") {
        let after_scheme = &url[scheme_end + 3..];
        let host_end = after_scheme
            .find('/')
            .unwrap_or(after_scheme.len())
            .min(after_scheme.find(':').unwrap_or(after_scheme.len()));
        after_scheme[..host_end].to_string()
    } else {
        url.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // --- CspSource parsing ---

    #[test]
    fn parse_none() {
        assert_eq!(CspSource::parse("'none'"), CspSource::None);
    }

    #[test]
    fn parse_self() {
        assert_eq!(CspSource::parse("'self'"), CspSource::SelfOrigin);
    }

    #[test]
    fn parse_unsafe_inline() {
        assert_eq!(CspSource::parse("'unsafe-inline'"), CspSource::UnsafeInline);
    }

    #[test]
    fn parse_unsafe_eval() {
        assert_eq!(CspSource::parse("'unsafe-eval'"), CspSource::UnsafeEval);
    }

    #[test]
    fn parse_nonce() {
        assert_eq!(
            CspSource::parse("'nonce-abc123'"),
            CspSource::Nonce("abc123".into())
        );
    }

    #[test]
    fn parse_hash() {
        let source = CspSource::parse("'sha256-abcdef'");
        assert_eq!(
            source,
            CspSource::Hash {
                algorithm: "sha256".into(),
                value: "abcdef".into()
            }
        );
    }

    #[test]
    fn parse_scheme() {
        assert_eq!(
            CspSource::parse("https:"),
            CspSource::Scheme("https".into())
        );
        assert_eq!(CspSource::parse("ipfs:"), CspSource::Scheme("ipfs".into()));
    }

    #[test]
    fn parse_host() {
        assert_eq!(
            CspSource::parse("example.com"),
            CspSource::Host("example.com".into())
        );
        assert_eq!(
            CspSource::parse("*.example.com"),
            CspSource::Host("*.example.com".into())
        );
    }

    #[test]
    fn parse_case_insensitive() {
        assert_eq!(CspSource::parse("'SELF'"), CspSource::SelfOrigin);
        assert_eq!(CspSource::parse("'NONE'"), CspSource::None);
    }

    // --- CspSource matching ---

    #[test]
    fn self_matches_same_origin() {
        let source = CspSource::SelfOrigin;
        assert!(source.matches("https://example.com/page", "https://example.com"));
        assert!(!source.matches("https://other.com/page", "https://example.com"));
    }

    #[test]
    fn scheme_matches() {
        let source = CspSource::Scheme("ipfs".into());
        assert!(source.matches("ipfs://QmTest123", "https://example.com"));
        assert!(!source.matches("https://example.com", "https://example.com"));
    }

    #[test]
    fn host_matches_exact() {
        let source = CspSource::Host("cdn.example.com".into());
        assert!(source.matches("https://cdn.example.com/file.js", "https://example.com"));
        assert!(!source.matches("https://other.com/file.js", "https://example.com"));
    }

    #[test]
    fn host_matches_wildcard() {
        let source = CspSource::Host("*.example.com".into());
        assert!(source.matches("https://cdn.example.com/file.js", "https://example.com"));
        assert!(source.matches("https://sub.cdn.example.com/file.js", "https://example.com"));
        assert!(!source.matches("https://other.com/file.js", "https://example.com"));
    }

    #[test]
    fn none_blocks_everything() {
        let source = CspSource::None;
        assert!(!source.matches("https://anything.com", "https://example.com"));
    }

    // --- CspDirective ---

    #[test]
    fn directive_parse() {
        assert_eq!(
            CspDirective::parse("script-src"),
            Some(CspDirective::ScriptSrc)
        );
        assert_eq!(
            CspDirective::parse("default-src"),
            Some(CspDirective::DefaultSrc)
        );
        assert_eq!(CspDirective::parse("unknown-src"), None);
    }

    #[test]
    fn directive_as_str_roundtrip() {
        let directives = [
            CspDirective::DefaultSrc,
            CspDirective::ScriptSrc,
            CspDirective::StyleSrc,
            CspDirective::ImgSrc,
            CspDirective::ConnectSrc,
        ];
        for d in &directives {
            assert_eq!(CspDirective::parse(d.as_str()), Some(d.clone()));
        }
    }

    #[test]
    fn directive_fallback() {
        assert_eq!(
            CspDirective::ScriptSrc.fallback(),
            Some(CspDirective::DefaultSrc)
        );
        assert_eq!(CspDirective::DefaultSrc.fallback(), None);
        assert_eq!(CspDirective::FrameAncestors.fallback(), None);
    }

    // --- CspPolicy parsing ---

    #[test]
    fn parse_simple_policy() {
        let policy = CspPolicy::parse("default-src 'self'; script-src 'none'");
        assert_eq!(policy.directive_count(), 2);
    }

    #[test]
    fn parse_complex_policy() {
        let policy = CspPolicy::parse(
            "default-src 'self'; script-src 'self' 'unsafe-inline'; \
             img-src 'self' data: ipfs:; connect-src 'self' wss:",
        );
        assert_eq!(policy.directive_count(), 4);
    }

    #[test]
    fn parse_empty_policy() {
        let policy = CspPolicy::parse("");
        assert_eq!(policy.directive_count(), 0);
    }

    #[test]
    fn parse_ignores_unknown_directives() {
        let policy = CspPolicy::parse("default-src 'self'; fake-directive 'none'");
        assert_eq!(policy.directive_count(), 1);
    }

    // --- CspPolicy evaluation ---

    #[test]
    fn allows_self_origin() {
        let policy = CspPolicy::parse("script-src 'self'");
        assert!(policy.allows(
            &CspDirective::ScriptSrc,
            "https://example.com/app.js",
            "https://example.com",
        ));
        assert!(!policy.allows(
            &CspDirective::ScriptSrc,
            "https://evil.com/bad.js",
            "https://example.com",
        ));
    }

    #[test]
    fn allows_scheme() {
        let policy = CspPolicy::parse("img-src ipfs: data:");
        assert!(policy.allows(
            &CspDirective::ImgSrc,
            "ipfs://QmImage123",
            "https://example.com",
        ));
        assert!(policy.allows(
            &CspDirective::ImgSrc,
            "data:image/png;base64,abc",
            "https://example.com",
        ));
        assert!(!policy.allows(
            &CspDirective::ImgSrc,
            "https://cdn.com/image.png",
            "https://example.com",
        ));
    }

    #[test]
    fn none_blocks_all() {
        let policy = CspPolicy::parse("object-src 'none'");
        assert!(!policy.allows(
            &CspDirective::ObjectSrc,
            "https://anything.com/plugin",
            "https://example.com",
        ));
    }

    #[test]
    fn falls_back_to_default_src() {
        let policy = CspPolicy::parse("default-src 'self'");
        // No script-src specified, falls back to default-src
        assert!(policy.allows(
            &CspDirective::ScriptSrc,
            "https://example.com/app.js",
            "https://example.com",
        ));
        assert!(!policy.allows(
            &CspDirective::ScriptSrc,
            "https://evil.com/bad.js",
            "https://example.com",
        ));
    }

    #[test]
    fn no_directive_allows_all() {
        let policy = CspPolicy::parse("script-src 'self'");
        // img-src not specified, no default-src either → allow
        assert!(policy.allows(
            &CspDirective::ImgSrc,
            "https://anything.com/image.png",
            "https://example.com",
        ));
    }

    #[test]
    fn evaluate_returns_violation() {
        let policy = CspPolicy::parse("script-src 'self'");
        let violation = policy.evaluate(
            &CspDirective::ScriptSrc,
            "https://evil.com/bad.js",
            "https://example.com",
        );
        assert!(violation.is_some());
        let v = violation.unwrap();
        assert_eq!(v.directive, CspDirective::ScriptSrc);
        assert_eq!(v.blocked_url, "https://evil.com/bad.js");
        assert!(!v.report_only);
    }

    #[test]
    fn evaluate_returns_none_when_allowed() {
        let policy = CspPolicy::parse("script-src 'self'");
        let violation = policy.evaluate(
            &CspDirective::ScriptSrc,
            "https://example.com/app.js",
            "https://example.com",
        );
        assert!(violation.is_none());
    }

    #[test]
    fn report_only_mode() {
        let mut policy = CspPolicy::parse("script-src 'self'");
        policy.set_report_only(true);

        let violation = policy.evaluate(
            &CspDirective::ScriptSrc,
            "https://evil.com/bad.js",
            "https://example.com",
        );
        assert!(violation.is_some());
        assert!(violation.unwrap().report_only);
    }

    // --- Inline and eval ---

    #[test]
    fn allows_inline() {
        let policy = CspPolicy::parse("style-src 'self' 'unsafe-inline'");
        assert!(policy.allows_inline(&CspDirective::StyleSrc));

        let strict = CspPolicy::parse("style-src 'self'");
        assert!(!strict.allows_inline(&CspDirective::StyleSrc));
    }

    #[test]
    fn allows_eval() {
        let policy = CspPolicy::parse("script-src 'self' 'unsafe-eval'");
        assert!(policy.allows_eval());

        let strict = CspPolicy::parse("script-src 'self'");
        assert!(!strict.allows_eval());
    }

    #[test]
    fn has_nonce() {
        let policy = CspPolicy::parse("script-src 'nonce-abc123'");
        assert!(policy.has_nonce(&CspDirective::ScriptSrc, "abc123"));
        assert!(!policy.has_nonce(&CspDirective::ScriptSrc, "wrong"));
    }

    // --- dApp default policy ---

    #[test]
    fn dapp_default_allows_self() {
        let policy = CspPolicy::dapp_default();
        assert!(policy.allows(
            &CspDirective::ScriptSrc,
            "ipfs://QmApp/app.js",
            "ipfs://QmApp",
        ));
    }

    #[test]
    fn dapp_default_allows_ipfs_images() {
        let policy = CspPolicy::dapp_default();
        assert!(policy.allows(
            &CspDirective::ImgSrc,
            "ipfs://QmImage/logo.png",
            "ipfs://QmApp",
        ));
    }

    #[test]
    fn dapp_default_blocks_objects() {
        let policy = CspPolicy::dapp_default();
        assert!(!policy.allows(
            &CspDirective::ObjectSrc,
            "https://anything.com/plugin.swf",
            "ipfs://QmApp",
        ));
    }

    #[test]
    fn dapp_default_blocks_frame_ancestors() {
        let policy = CspPolicy::dapp_default();
        assert!(!policy.allows(
            &CspDirective::FrameAncestors,
            "https://framer.com",
            "ipfs://QmApp",
        ));
    }

    #[test]
    fn dapp_default_allows_wss_connect() {
        let policy = CspPolicy::dapp_default();
        assert!(policy.allows(
            &CspDirective::ConnectSrc,
            "wss://relay.example.com",
            "ipfs://QmApp",
        ));
    }

    // --- Serialization ---

    #[test]
    fn to_header_roundtrip() {
        let policy = CspPolicy::parse("default-src 'self'; script-src 'none'");
        let header = policy.to_header();
        assert!(header.contains("default-src 'self'"));
        assert!(header.contains("script-src 'none'"));
    }

    #[test]
    fn policy_serializes() {
        let policy = CspPolicy::dapp_default();
        let json = serde_json::to_string(&policy).unwrap();
        let restored: CspPolicy = serde_json::from_str(&json).unwrap();
        assert_eq!(restored.directive_count(), policy.directive_count());
    }

    // --- Violation ---

    #[test]
    fn violation_description() {
        let v = CspViolation {
            directive: CspDirective::ScriptSrc,
            blocked_url: "https://evil.com/bad.js".into(),
            page_origin: "https://example.com".into(),
            report_only: false,
        };
        let desc = v.description();
        assert!(desc.contains("blocked"));
        assert!(desc.contains("script-src"));
        assert!(desc.contains("evil.com"));
    }

    #[test]
    fn violation_report_only_description() {
        let v = CspViolation {
            directive: CspDirective::ScriptSrc,
            blocked_url: "https://evil.com/bad.js".into(),
            page_origin: "https://example.com".into(),
            report_only: true,
        };
        assert!(v.description().contains("would block"));
    }

    // --- Helper functions ---

    #[test]
    fn extract_origin_https() {
        assert_eq!(
            extract_origin("https://example.com/page?q=1"),
            "https://example.com"
        );
    }

    #[test]
    fn extract_origin_ipfs() {
        assert_eq!(extract_origin("ipfs://QmTest123/file"), "ipfs://QmTest123");
    }

    #[test]
    fn extract_host_from_url() {
        assert_eq!(extract_host("https://example.com/path"), "example.com");
        assert_eq!(
            extract_host("https://sub.example.com:8080/path"),
            "sub.example.com"
        );
    }

    #[test]
    fn host_wildcard_matching() {
        assert!(host_matches(
            "*.example.com",
            "https://cdn.example.com/file"
        ));
        assert!(host_matches("*.example.com", "https://example.com/file"));
        assert!(!host_matches("*.example.com", "https://other.com/file"));
    }
}
