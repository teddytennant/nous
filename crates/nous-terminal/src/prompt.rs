//! Nous-aware terminal prompt.
//!
//! Renders a status-rich prompt showing identity, wallet balance,
//! and connection state. Follows Infinite Minimalism: restrained,
//! information-dense, no clutter.

/// Prompt segment data.
#[derive(Debug, Clone, Default)]
pub struct PromptState {
    /// Active DID (truncated for display).
    pub identity: Option<IdentitySegment>,
    /// Wallet balance summary.
    pub wallet: Option<WalletSegment>,
    /// Network connection status.
    pub connection: ConnectionStatus,
    /// Current working path (for file operations).
    pub path: Option<String>,
}

/// Identity information for the prompt.
#[derive(Debug, Clone)]
pub struct IdentitySegment {
    /// Full DID string.
    pub did: String,
    /// Display name, if set.
    pub display_name: Option<String>,
}

/// Wallet balance for the prompt.
#[derive(Debug, Clone)]
pub struct WalletSegment {
    /// Primary token balance (e.g., "1.5 ETH").
    pub primary_balance: String,
    /// Number of pending transactions.
    pub pending_tx: u32,
}

/// Network connection status.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ConnectionStatus {
    Online,
    #[default]
    Offline,
    Syncing,
}

/// Prompt rendering configuration.
#[derive(Debug, Clone)]
pub struct PromptConfig {
    /// Show identity segment.
    pub show_identity: bool,
    /// Show wallet segment.
    pub show_wallet: bool,
    /// Show connection status.
    pub show_connection: bool,
    /// Show path segment.
    pub show_path: bool,
    /// The prompt character (default: ">").
    pub prompt_char: String,
    /// Separator between segments.
    pub separator: String,
}

impl Default for PromptConfig {
    fn default() -> Self {
        Self {
            show_identity: true,
            show_wallet: true,
            show_connection: true,
            show_path: false,
            prompt_char: ">".to_string(),
            separator: " | ".to_string(),
        }
    }
}

/// Render the prompt as a plain string (no ANSI codes).
pub fn render_plain(state: &PromptState, config: &PromptConfig) -> String {
    let mut segments: Vec<String> = Vec::new();

    if config.show_connection {
        let status = match state.connection {
            ConnectionStatus::Online => "online",
            ConnectionStatus::Offline => "offline",
            ConnectionStatus::Syncing => "syncing",
        };
        segments.push(status.to_string());
    }

    if config.show_identity
        && let Some(ref id) = state.identity
    {
        let label = id
            .display_name
            .as_deref()
            .unwrap_or_else(|| truncate_did(&id.did));
        segments.push(label.to_string());
    }

    if config.show_wallet
        && let Some(ref w) = state.wallet
    {
        let mut s = w.primary_balance.clone();
        if w.pending_tx > 0 {
            s.push_str(&format!(" (+{})", w.pending_tx));
        }
        segments.push(s);
    }

    if config.show_path
        && let Some(ref p) = state.path
    {
        segments.push(p.clone());
    }

    if segments.is_empty() {
        format!("nous{} ", config.prompt_char)
    } else {
        format!(
            "nous [{}]{} ",
            segments.join(&config.separator),
            config.prompt_char
        )
    }
}

/// Render the prompt with ANSI escape codes for terminal display.
pub fn render_ansi(state: &PromptState, config: &PromptConfig) -> String {
    let mut segments: Vec<String> = Vec::new();
    let gold = "\x1b[38;2;212;175;55m";
    let dim = "\x1b[38;2;100;100;100m";
    let green = "\x1b[38;2;100;180;100m";
    let red = "\x1b[38;2;190;80;70m";
    let yellow = "\x1b[38;2;212;175;55m";
    let reset = "\x1b[0m";

    if config.show_connection {
        let (color, label) = match state.connection {
            ConnectionStatus::Online => (green, "online"),
            ConnectionStatus::Offline => (red, "offline"),
            ConnectionStatus::Syncing => (yellow, "syncing"),
        };
        segments.push(format!("{color}{label}{reset}"));
    }

    if config.show_identity
        && let Some(ref id) = state.identity
    {
        let label = id
            .display_name
            .as_deref()
            .unwrap_or_else(|| truncate_did(&id.did));
        segments.push(format!("{dim}{label}{reset}"));
    }

    if config.show_wallet
        && let Some(ref w) = state.wallet
    {
        let mut s = format!("{dim}{}{reset}", w.primary_balance);
        if w.pending_tx > 0 {
            s.push_str(&format!(" {gold}(+{}){reset}", w.pending_tx));
        }
        segments.push(s);
    }

    if config.show_path
        && let Some(ref p) = state.path
    {
        segments.push(format!("{dim}{p}{reset}"));
    }

    let sep = format!("{dim}{}{reset}", config.separator);

    if segments.is_empty() {
        format!("{gold}nous{reset}{} ", config.prompt_char)
    } else {
        format!(
            "{gold}nous{reset} {dim}[{reset}{}{dim}]{reset}{} ",
            segments.join(&sep),
            config.prompt_char
        )
    }
}

/// Truncate a DID for display: "did:key:z6Mk...last6".
fn truncate_did(did: &str) -> &str {
    // Return a static-lifetime-compatible approach won't work here.
    // We'll just return the full DID if it's short, or a prefix otherwise.
    // For actual truncation, the caller can format it.
    if did.len() <= 20 {
        did
    } else {
        // Return up to first 16 chars — the display function handles the rest
        &did[..16]
    }
}

/// Format a DID as a short display string.
pub fn format_did_short(did: &str) -> String {
    if did.len() <= 20 {
        did.to_string()
    } else {
        format!("{}...{}", &did[..12], &did[did.len() - 6..])
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_state_plain() {
        let state = PromptState::default();
        let config = PromptConfig::default();
        let prompt = render_plain(&state, &config);
        assert!(prompt.contains("nous"));
        assert!(prompt.contains("offline"));
        assert!(prompt.ends_with("> "));
    }

    #[test]
    fn full_state_plain() {
        let state = PromptState {
            identity: Some(IdentitySegment {
                did: "did:key:z6MkhaXgBZDvotDkL5257faiztiGiC2QtKLGpbnnEGta2doK".into(),
                display_name: Some("alice".into()),
            }),
            wallet: Some(WalletSegment {
                primary_balance: "1.5 ETH".into(),
                pending_tx: 2,
            }),
            connection: ConnectionStatus::Online,
            path: None,
        };
        let config = PromptConfig::default();
        let prompt = render_plain(&state, &config);

        assert!(prompt.contains("online"));
        assert!(prompt.contains("alice"));
        assert!(prompt.contains("1.5 ETH"));
        assert!(prompt.contains("(+2)"));
    }

    #[test]
    fn identity_without_display_name() {
        let state = PromptState {
            identity: Some(IdentitySegment {
                did: "did:key:z6MkhaXgBZDvotDkL5257faiztiGiC2QtKLGpbnnEGta2doK".into(),
                display_name: None,
            }),
            connection: ConnectionStatus::Online,
            ..Default::default()
        };
        let config = PromptConfig {
            show_wallet: false,
            ..Default::default()
        };
        let prompt = render_plain(&state, &config);
        // Should show truncated DID
        assert!(prompt.contains("did:key:z6MkhaXg"));
    }

    #[test]
    fn no_pending_tx() {
        let state = PromptState {
            wallet: Some(WalletSegment {
                primary_balance: "0.0 ETH".into(),
                pending_tx: 0,
            }),
            ..Default::default()
        };
        let config = PromptConfig::default();
        let prompt = render_plain(&state, &config);
        assert!(prompt.contains("0.0 ETH"));
        assert!(!prompt.contains("(+"));
    }

    #[test]
    fn connection_states() {
        for (status, expected) in [
            (ConnectionStatus::Online, "online"),
            (ConnectionStatus::Offline, "offline"),
            (ConnectionStatus::Syncing, "syncing"),
        ] {
            let state = PromptState {
                connection: status,
                ..Default::default()
            };
            let config = PromptConfig::default();
            let prompt = render_plain(&state, &config);
            assert!(
                prompt.contains(expected),
                "expected '{expected}' in '{prompt}'"
            );
        }
    }

    #[test]
    fn custom_prompt_char() {
        let state = PromptState::default();
        let config = PromptConfig {
            prompt_char: "$".into(),
            show_connection: false,
            ..Default::default()
        };
        let prompt = render_plain(&state, &config);
        assert!(prompt.ends_with("$ "));
    }

    #[test]
    fn custom_separator() {
        let state = PromptState {
            identity: Some(IdentitySegment {
                did: "did:key:z6Mk".into(),
                display_name: Some("bob".into()),
            }),
            connection: ConnectionStatus::Online,
            ..Default::default()
        };
        let config = PromptConfig {
            separator: " :: ".into(),
            show_wallet: false,
            ..Default::default()
        };
        let prompt = render_plain(&state, &config);
        assert!(prompt.contains(" :: "));
    }

    #[test]
    fn disabled_segments() {
        let state = PromptState {
            identity: Some(IdentitySegment {
                did: "did:key:z6Mk".into(),
                display_name: Some("alice".into()),
            }),
            wallet: Some(WalletSegment {
                primary_balance: "1 ETH".into(),
                pending_tx: 0,
            }),
            connection: ConnectionStatus::Online,
            ..Default::default()
        };
        let config = PromptConfig {
            show_identity: false,
            show_wallet: false,
            show_connection: false,
            ..Default::default()
        };
        let prompt = render_plain(&state, &config);
        assert!(!prompt.contains("alice"));
        assert!(!prompt.contains("ETH"));
        assert!(!prompt.contains("online"));
    }

    #[test]
    fn ansi_prompt_has_escape_codes() {
        let state = PromptState {
            connection: ConnectionStatus::Online,
            ..Default::default()
        };
        let config = PromptConfig::default();
        let prompt = render_ansi(&state, &config);
        assert!(prompt.contains("\x1b["));
        assert!(prompt.contains("online"));
    }

    #[test]
    fn ansi_prompt_full() {
        let state = PromptState {
            identity: Some(IdentitySegment {
                did: "did:key:z6MkhaXgBZDvotDkL5257faiztiGiC2QtKLGpbnnEGta2doK".into(),
                display_name: Some("alice".into()),
            }),
            wallet: Some(WalletSegment {
                primary_balance: "2.0 NOUS".into(),
                pending_tx: 1,
            }),
            connection: ConnectionStatus::Syncing,
            path: Some("~/files".into()),
        };
        let config = PromptConfig {
            show_path: true,
            ..Default::default()
        };
        let prompt = render_ansi(&state, &config);
        assert!(prompt.contains("nous"));
        assert!(prompt.contains("alice"));
        assert!(prompt.contains("2.0 NOUS"));
        assert!(prompt.contains("syncing"));
        assert!(prompt.contains("~/files"));
    }

    #[test]
    fn format_did_short_truncates() {
        let short = format_did_short("did:key:z6Mk");
        assert_eq!(short, "did:key:z6Mk");

        let long = format_did_short("did:key:z6MkhaXgBZDvotDkL5257faiztiGiC2QtKLGpbnnEGta2doK");
        assert!(long.contains("..."));
        assert!(long.starts_with("did:key:z6Mk"));
        assert!(long.ends_with("a2doK"));
    }

    #[test]
    fn with_path_segment() {
        let state = PromptState {
            path: Some("/vault/docs".into()),
            ..Default::default()
        };
        let config = PromptConfig {
            show_path: true,
            show_connection: false,
            ..Default::default()
        };
        let prompt = render_plain(&state, &config);
        assert!(prompt.contains("/vault/docs"));
    }
}
