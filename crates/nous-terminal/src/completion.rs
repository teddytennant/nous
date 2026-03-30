//! Tab completion for Nous terminal commands.
//!
//! Provides prefix-based completion for command names, subcommands,
//! and argument values. The completion engine is stateless — it takes
//! the current input and cursor position and returns candidates.

use crate::command::CommandRegistry;

/// A completion candidate.
#[derive(Debug, Clone, PartialEq)]
pub struct Completion {
    /// The completed text to insert.
    pub text: String,
    /// Display text (may include description).
    pub display: String,
    /// What kind of thing was completed.
    pub kind: CompletionKind,
}

/// What type of entity was completed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CompletionKind {
    Command,
    Subcommand,
    Flag,
    Argument,
}

/// Tab completion engine.
#[derive(Debug)]
pub struct Completer {
    /// Additional known DID values for argument completion.
    known_dids: Vec<String>,
    /// Additional known channel IDs for argument completion.
    known_channels: Vec<String>,
}

impl Completer {
    /// Create a new completer with empty argument caches.
    pub fn new() -> Self {
        Self {
            known_dids: Vec::new(),
            known_channels: Vec::new(),
        }
    }

    /// Update the known DIDs for argument completion.
    pub fn set_known_dids(&mut self, dids: Vec<String>) {
        self.known_dids = dids;
    }

    /// Update the known channel IDs for argument completion.
    pub fn set_known_channels(&mut self, channels: Vec<String>) {
        self.known_channels = channels;
    }

    /// Generate completions for the given input.
    ///
    /// Returns a list of candidates sorted alphabetically.
    pub fn complete(&self, input: &str, registry: &CommandRegistry) -> Vec<Completion> {
        let trimmed = input.trim_start();
        if !trimmed.starts_with('/') {
            return Vec::new();
        }

        let without_slash = &trimmed[1..];
        let parts: Vec<&str> = without_slash.split_whitespace().collect();

        match parts.len() {
            0 => {
                // Just "/" typed — show all commands
                self.complete_command("", registry)
            }
            1 if !input.ends_with(' ') => {
                // Completing command name: "/wal" → "/wallet"
                self.complete_command(parts[0], registry)
            }
            1 => {
                // Command complete, space typed: "/wallet " → show subcommands
                self.complete_subcommand(parts[0], "", registry)
            }
            2 if !input.ends_with(' ') => {
                // Completing subcommand: "/wallet bal" → "/wallet balance"
                let prefix = parts[1];
                if prefix.starts_with('-') {
                    self.complete_flag(parts[0], prefix)
                } else {
                    self.complete_subcommand(parts[0], prefix, registry)
                }
            }
            _ => {
                // After subcommand — complete flags or arguments
                let last = parts.last().unwrap();
                if !input.ends_with(' ') && last.starts_with('-') {
                    self.complete_flag(parts[0], last)
                } else {
                    let prev = if parts.len() >= 2 {
                        parts[parts.len() - 2]
                    } else {
                        ""
                    };
                    self.complete_argument(
                        parts[0],
                        prev,
                        if input.ends_with(' ') { "" } else { last },
                    )
                }
            }
        }
    }

    /// Apply the first matching completion to input.
    ///
    /// Returns the completed input string or `None` if no completions.
    pub fn apply_first(&self, input: &str, registry: &CommandRegistry) -> Option<String> {
        let completions = self.complete(input, registry);
        let first = completions.first()?;

        let trimmed = input.trim_start();
        let parts: Vec<&str> = trimmed[1..].split_whitespace().collect();

        match parts.len() {
            0 => Some(format!("/{} ", first.text)),
            1 if !input.ends_with(' ') => Some(format!("/{} ", first.text)),
            _ if !input.ends_with(' ') => {
                // Replace the last token with the completion
                let last_space = input.rfind(' ').unwrap_or(0);
                let prefix = &input[..=last_space];
                Some(format!("{}{} ", prefix, first.text))
            }
            _ => Some(format!("{}{} ", input, first.text)),
        }
    }

    /// Find the longest common prefix among all completions.
    pub fn common_prefix(&self, input: &str, registry: &CommandRegistry) -> Option<String> {
        let completions = self.complete(input, registry);
        if completions.is_empty() {
            return None;
        }
        if completions.len() == 1 {
            return self.apply_first(input, registry);
        }

        let texts: Vec<&str> = completions.iter().map(|c| c.text.as_str()).collect();
        let prefix = longest_common_prefix(&texts);
        if prefix.is_empty() || completions.iter().any(|c| c.text == prefix) {
            // Common prefix is a complete match — don't expand further
            return None;
        }

        let trimmed = input.trim_start();
        let parts: Vec<&str> = trimmed[1..].split_whitespace().collect();

        match parts.len() {
            0 => Some(format!("/{prefix}")),
            1 if !input.ends_with(' ') => Some(format!("/{prefix}")),
            _ if !input.ends_with(' ') => {
                let last_space = input.rfind(' ').unwrap_or(0);
                let input_prefix = &input[..=last_space];
                Some(format!("{input_prefix}{prefix}"))
            }
            _ => Some(format!("{input}{prefix}")),
        }
    }

    fn complete_command(&self, prefix: &str, registry: &CommandRegistry) -> Vec<Completion> {
        let lower = prefix.to_lowercase();
        let mut completions: Vec<Completion> = registry
            .command_names()
            .into_iter()
            .filter(|name| name.starts_with(&lower))
            .map(|name| {
                let desc = registry.get(name).map(|i| i.description).unwrap_or("");
                Completion {
                    text: name.to_string(),
                    display: format!("/{name}  {desc}"),
                    kind: CompletionKind::Command,
                }
            })
            .collect();
        completions.sort_by(|a, b| a.text.cmp(&b.text));
        completions
    }

    fn complete_subcommand(
        &self,
        command: &str,
        prefix: &str,
        registry: &CommandRegistry,
    ) -> Vec<Completion> {
        let lower_prefix = prefix.to_lowercase();
        let lower_cmd = command.to_lowercase();
        let info = match registry.get(&lower_cmd) {
            Some(info) => info,
            None => return Vec::new(),
        };

        let mut completions: Vec<Completion> = info
            .subcommands
            .iter()
            .filter(|(name, _)| name.to_lowercase().starts_with(&lower_prefix))
            .map(|(name, desc)| Completion {
                text: name.to_string(),
                display: format!("{name}  {desc}"),
                kind: CompletionKind::Subcommand,
            })
            .collect();
        completions.sort_by(|a, b| a.text.cmp(&b.text));
        completions
    }

    fn complete_flag(&self, command: &str, prefix: &str) -> Vec<Completion> {
        let flag_prefix = prefix.trim_start_matches('-');
        let flags = common_flags(command);

        let mut completions: Vec<Completion> = flags
            .into_iter()
            .filter(|(name, _)| name.starts_with(flag_prefix))
            .map(|(name, desc)| Completion {
                text: format!("--{name}"),
                display: format!("--{name}  {desc}"),
                kind: CompletionKind::Flag,
            })
            .collect();
        completions.sort_by(|a, b| a.text.cmp(&b.text));
        completions
    }

    fn complete_argument(&self, command: &str, prev_flag: &str, prefix: &str) -> Vec<Completion> {
        let lower = prefix.to_lowercase();

        // If the previous token was --to or similar, complete with known DIDs
        let source = match prev_flag.trim_start_matches('-') {
            "to" | "from" | "did" | "member" | "seller" | "buyer" => &self.known_dids,
            "channel" | "channel-id" => &self.known_channels,
            _ => {
                // Based on command context
                match command {
                    "message" | "peer" => &self.known_dids,
                    _ => return Vec::new(),
                }
            }
        };

        let mut completions: Vec<Completion> = source
            .iter()
            .filter(|v| v.to_lowercase().starts_with(&lower))
            .map(|v| Completion {
                text: v.clone(),
                display: v.clone(),
                kind: CompletionKind::Argument,
            })
            .collect();
        completions.sort_by(|a, b| a.text.cmp(&b.text));
        completions
    }
}

impl Default for Completer {
    fn default() -> Self {
        Self::new()
    }
}

/// Known flags for each command (used for flag completion).
fn common_flags(command: &str) -> Vec<(&'static str, &'static str)> {
    match command {
        "wallet" => vec![
            ("to", "Recipient DID"),
            ("amount", "Token amount"),
            ("token", "Token name (ETH, NOUS, USDC)"),
            ("memo", "Transaction memo"),
        ],
        "message" => vec![
            ("to", "Recipient DID"),
            ("channel", "Channel ID"),
            ("reply", "Reply to message ID"),
        ],
        "peer" => vec![("addr", "Multiaddress"), ("verbose", "Show detailed info")],
        "governance" => vec![
            ("dao", "DAO ID"),
            ("title", "Proposal title"),
            ("credits", "Vote credits"),
        ],
        "file" => vec![("name", "File name"), ("owner", "Owner DID")],
        "identity" => vec![("name", "Display name"), ("format", "Output format")],
        _ => vec![("json", "Output as JSON"), ("verbose", "Verbose output")],
    }
}

/// Find the longest common prefix among a set of strings.
fn longest_common_prefix(strings: &[&str]) -> String {
    if strings.is_empty() {
        return String::new();
    }
    if strings.len() == 1 {
        return strings[0].to_string();
    }

    let first = strings[0];
    let mut prefix_len = first.len();

    for s in &strings[1..] {
        prefix_len = prefix_len.min(s.len());
        for (i, (a, b)) in first.chars().zip(s.chars()).enumerate() {
            if a != b {
                prefix_len = prefix_len.min(i);
                break;
            }
        }
    }

    first[..prefix_len].to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn registry() -> CommandRegistry {
        CommandRegistry::new()
    }

    #[test]
    fn complete_empty_slash() {
        let c = Completer::new();
        let completions = c.complete("/", &registry());
        assert!(!completions.is_empty());
        assert!(
            completions
                .iter()
                .all(|c| c.kind == CompletionKind::Command)
        );
    }

    #[test]
    fn complete_command_prefix() {
        let c = Completer::new();
        let completions = c.complete("/wal", &registry());
        assert_eq!(completions.len(), 1);
        assert_eq!(completions[0].text, "wallet");
    }

    #[test]
    fn complete_command_multiple_matches() {
        let c = Completer::new();
        // "e" matches "exit"
        let completions = c.complete("/e", &registry());
        assert!(completions.iter().any(|c| c.text == "exit"));
    }

    #[test]
    fn complete_subcommand() {
        let c = Completer::new();
        let completions = c.complete("/wallet ", &registry());
        assert!(!completions.is_empty());
        assert!(
            completions
                .iter()
                .all(|c| c.kind == CompletionKind::Subcommand)
        );
        assert!(completions.iter().any(|c| c.text == "balance"));
        assert!(completions.iter().any(|c| c.text == "send"));
    }

    #[test]
    fn complete_subcommand_prefix() {
        let c = Completer::new();
        let completions = c.complete("/wallet bal", &registry());
        assert_eq!(completions.len(), 1);
        assert_eq!(completions[0].text, "balance");
    }

    #[test]
    fn complete_flag() {
        let c = Completer::new();
        let completions = c.complete("/wallet send --", &registry());
        assert!(!completions.is_empty());
        assert!(completions.iter().all(|c| c.kind == CompletionKind::Flag));
        assert!(completions.iter().any(|c| c.text == "--to"));
    }

    #[test]
    fn complete_flag_prefix() {
        let c = Completer::new();
        let completions = c.complete("/wallet send --to", &registry());
        assert_eq!(completions.len(), 2);
        assert!(completions.iter().any(|c| c.text == "--to"));
        assert!(completions.iter().any(|c| c.text == "--token"));
    }

    #[test]
    fn complete_flag_prefix_unique() {
        let c = Completer::new();
        let completions = c.complete("/wallet send --am", &registry());
        assert_eq!(completions.len(), 1);
        assert_eq!(completions[0].text, "--amount");
    }

    #[test]
    fn complete_argument_did() {
        let mut c = Completer::new();
        c.set_known_dids(vec!["did:key:z6MkAlice".into(), "did:key:z6MkBob".into()]);

        let completions = c.complete("/wallet send --to did:key:z6MkA", &registry());
        assert_eq!(completions.len(), 1);
        assert_eq!(completions[0].text, "did:key:z6MkAlice");
    }

    #[test]
    fn no_completions_for_non_command() {
        let c = Completer::new();
        let completions = c.complete("echo hello", &registry());
        assert!(completions.is_empty());
    }

    #[test]
    fn apply_first_command() {
        let c = Completer::new();
        let result = c.apply_first("/wal", &registry());
        assert_eq!(result, Some("/wallet ".to_string()));
    }

    #[test]
    fn apply_first_subcommand() {
        let c = Completer::new();
        let result = c.apply_first("/wallet bal", &registry());
        assert_eq!(result, Some("/wallet balance ".to_string()));
    }

    #[test]
    fn apply_first_no_match() {
        let c = Completer::new();
        let result = c.apply_first("/zzzzz", &registry());
        assert!(result.is_none());
    }

    #[test]
    fn unknown_command_no_subcommands() {
        let c = Completer::new();
        let completions = c.complete("/unknown ", &registry());
        assert!(completions.is_empty());
    }

    #[test]
    fn longest_common_prefix_basic() {
        assert_eq!(longest_common_prefix(&["abc", "abd", "abe"]), "ab");
        assert_eq!(longest_common_prefix(&["hello"]), "hello");
        assert_eq!(longest_common_prefix(&["abc", "xyz"]), "");
        assert_eq!(longest_common_prefix(&[]), "");
    }

    #[test]
    fn common_prefix_completion() {
        let c = Completer::new();
        // /f matches "file" — only one match so common_prefix returns full
        let result = c.common_prefix("/fi", &registry());
        assert_eq!(result, Some("/file ".to_string()));
    }

    #[test]
    fn completion_sorted() {
        let c = Completer::new();
        let completions = c.complete("/", &registry());
        let texts: Vec<&str> = completions.iter().map(|c| c.text.as_str()).collect();
        let mut sorted = texts.clone();
        sorted.sort();
        assert_eq!(texts, sorted);
    }

    #[test]
    fn set_known_channels() {
        let mut c = Completer::new();
        c.set_known_channels(vec!["ch-abc123".into(), "ch-def456".into()]);

        let completions = c.complete("/message read --channel ch-abc", &registry());
        assert_eq!(completions.len(), 1);
        assert_eq!(completions[0].text, "ch-abc123");
    }
}
