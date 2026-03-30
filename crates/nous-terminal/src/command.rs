//! Command parser and registry for the embedded terminal.
//!
//! Parses user input into structured commands that map to Nous operations.
//! Commands are prefixed with `/` to distinguish them from shell input.

use std::collections::HashMap;

/// A parsed command with its arguments.
#[derive(Debug, Clone, PartialEq)]
pub struct ParsedCommand {
    pub name: String,
    pub subcommand: Option<String>,
    pub args: Vec<String>,
    pub flags: HashMap<String, Option<String>>,
}

/// Result of executing a command.
#[derive(Debug, Clone)]
pub struct CommandOutput {
    pub text: String,
    pub status: CommandStatus,
}

/// Command execution status.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CommandStatus {
    Success,
    Error,
    NotFound,
}

impl CommandOutput {
    pub fn success(text: impl Into<String>) -> Self {
        Self {
            text: text.into(),
            status: CommandStatus::Success,
        }
    }

    pub fn error(text: impl Into<String>) -> Self {
        Self {
            text: text.into(),
            status: CommandStatus::Error,
        }
    }

    pub fn not_found(name: &str) -> Self {
        Self {
            text: format!("unknown command: /{name}. Type /help for available commands."),
            status: CommandStatus::NotFound,
        }
    }
}

/// Metadata about a registered command.
#[derive(Debug, Clone)]
pub struct CommandInfo {
    pub name: &'static str,
    pub description: &'static str,
    pub usage: &'static str,
    pub subcommands: &'static [(&'static str, &'static str)],
}

/// Registry of all available commands.
#[derive(Debug)]
pub struct CommandRegistry {
    commands: HashMap<&'static str, CommandInfo>,
}

impl CommandRegistry {
    /// Create a new registry with all built-in Nous commands.
    pub fn new() -> Self {
        let mut commands = HashMap::new();

        let builtins: Vec<CommandInfo> = vec![
            CommandInfo {
                name: "help",
                description: "Show available commands",
                usage: "/help [command]",
                subcommands: &[],
            },
            CommandInfo {
                name: "identity",
                description: "Identity management",
                usage: "/identity <subcommand>",
                subcommands: &[
                    ("show", "Display current identity"),
                    ("create", "Generate a new DID"),
                    ("list", "List known identities"),
                    ("switch", "Switch active identity"),
                    ("export", "Export DID document"),
                ],
            },
            CommandInfo {
                name: "wallet",
                description: "Wallet and payments",
                usage: "/wallet <subcommand>",
                subcommands: &[
                    ("balance", "Show token balances"),
                    ("send", "Send tokens"),
                    ("receive", "Show receive address"),
                    ("history", "Transaction history"),
                ],
            },
            CommandInfo {
                name: "message",
                description: "Encrypted messaging",
                usage: "/message <subcommand>",
                subcommands: &[
                    ("list", "List channels"),
                    ("send", "Send a message"),
                    ("read", "Read messages in a channel"),
                    ("create", "Create a new channel"),
                ],
            },
            CommandInfo {
                name: "peer",
                description: "Peer and network management",
                usage: "/peer <subcommand>",
                subcommands: &[
                    ("list", "List connected peers"),
                    ("connect", "Connect to a peer"),
                    ("disconnect", "Disconnect from a peer"),
                    ("info", "Show peer details"),
                ],
            },
            CommandInfo {
                name: "governance",
                description: "DAO governance",
                usage: "/governance <subcommand>",
                subcommands: &[
                    ("list", "List DAOs"),
                    ("create", "Create a new DAO"),
                    ("propose", "Create a proposal"),
                    ("vote", "Cast a vote"),
                ],
            },
            CommandInfo {
                name: "file",
                description: "Decentralized file storage",
                usage: "/file <subcommand>",
                subcommands: &[
                    ("list", "List stored files"),
                    ("upload", "Upload a file"),
                    ("download", "Download a file"),
                    ("delete", "Delete a file"),
                ],
            },
            CommandInfo {
                name: "node",
                description: "Node status and management",
                usage: "/node <subcommand>",
                subcommands: &[
                    ("status", "Show node status"),
                    ("health", "Health check"),
                    ("config", "Show configuration"),
                ],
            },
            CommandInfo {
                name: "clear",
                description: "Clear the terminal screen",
                usage: "/clear",
                subcommands: &[],
            },
            CommandInfo {
                name: "exit",
                description: "Exit the terminal",
                usage: "/exit",
                subcommands: &[],
            },
        ];

        for info in builtins {
            commands.insert(info.name, info);
        }

        Self { commands }
    }

    /// Look up a command by name.
    pub fn get(&self, name: &str) -> Option<&CommandInfo> {
        self.commands.get(name)
    }

    /// Get all registered command names, sorted.
    pub fn command_names(&self) -> Vec<&'static str> {
        let mut names: Vec<_> = self.commands.keys().copied().collect();
        names.sort();
        names
    }

    /// Get subcommand names for a given command.
    pub fn subcommand_names(&self, command: &str) -> Vec<&'static str> {
        self.commands
            .get(command)
            .map(|info| info.subcommands.iter().map(|(name, _)| *name).collect())
            .unwrap_or_default()
    }

    /// Format help text for all commands.
    pub fn help_all(&self) -> String {
        let mut lines = vec!["Available commands:".to_string(), String::new()];
        let mut names = self.command_names();
        names.sort();

        let max_len = names.iter().map(|n| n.len()).max().unwrap_or(0);

        for name in &names {
            if let Some(info) = self.commands.get(name) {
                lines.push(format!(
                    "  /{:<width$}  {}",
                    name,
                    info.description,
                    width = max_len
                ));
            }
        }

        lines.push(String::new());
        lines.push("Type /help <command> for detailed usage.".to_string());
        lines.join("\n")
    }

    /// Format help text for a specific command.
    pub fn help_for(&self, name: &str) -> Option<String> {
        let info = self.commands.get(name)?;
        let mut lines = vec![
            format!("/{} — {}", info.name, info.description),
            String::new(),
            format!("Usage: {}", info.usage),
        ];

        if !info.subcommands.is_empty() {
            lines.push(String::new());
            lines.push("Subcommands:".to_string());

            let max_len = info
                .subcommands
                .iter()
                .map(|(n, _)| n.len())
                .max()
                .unwrap_or(0);

            for (sub, desc) in info.subcommands {
                lines.push(format!("  {:<width$}  {}", sub, desc, width = max_len));
            }
        }

        Some(lines.join("\n"))
    }
}

impl Default for CommandRegistry {
    fn default() -> Self {
        Self::new()
    }
}

/// Parse a line of input into a command, if it starts with `/`.
///
/// Returns `None` if the input is not a command (i.e., regular shell input).
pub fn parse(input: &str) -> Option<ParsedCommand> {
    let trimmed = input.trim();
    if !trimmed.starts_with('/') {
        return None;
    }

    let without_slash = &trimmed[1..];
    if without_slash.is_empty() {
        return None;
    }

    let tokens = tokenize(without_slash);
    if tokens.is_empty() {
        return None;
    }

    let name = tokens[0].to_lowercase();
    let mut subcommand = None;
    let mut args = Vec::new();
    let mut flags = HashMap::new();
    let mut i = 1;

    // First non-flag token after name is the subcommand
    if i < tokens.len() && !tokens[i].starts_with('-') {
        subcommand = Some(tokens[i].to_string());
        i += 1;
    }

    while i < tokens.len() {
        let token = &tokens[i];
        if let Some(flag) = token.strip_prefix("--") {
            // --key=value or --key value or --flag
            if let Some((key, value)) = flag.split_once('=') {
                flags.insert(key.to_string(), Some(value.to_string()));
            } else if i + 1 < tokens.len() && !tokens[i + 1].starts_with('-') {
                flags.insert(flag.to_string(), Some(tokens[i + 1].clone()));
                i += 1;
            } else {
                flags.insert(flag.to_string(), None);
            }
        } else if let Some(flag) = token.strip_prefix('-') {
            flags.insert(flag.to_string(), None);
        } else {
            args.push(token.clone());
        }
        i += 1;
    }

    Some(ParsedCommand {
        name,
        subcommand,
        args,
        flags,
    })
}

/// Tokenize input respecting quoted strings.
fn tokenize(input: &str) -> Vec<String> {
    let mut tokens = Vec::new();
    let mut current = String::new();
    let mut in_quote = false;
    let mut quote_char = '"';

    for ch in input.chars() {
        if in_quote {
            if ch == quote_char {
                in_quote = false;
            } else {
                current.push(ch);
            }
        } else if ch == '"' || ch == '\'' {
            in_quote = true;
            quote_char = ch;
        } else if ch.is_whitespace() {
            if !current.is_empty() {
                tokens.push(current.clone());
                current.clear();
            }
        } else {
            current.push(ch);
        }
    }

    if !current.is_empty() {
        tokens.push(current);
    }

    tokens
}

/// Execute a built-in command (help, clear, exit).
/// Returns `None` if the command needs to be dispatched to the API.
pub fn execute_builtin(cmd: &ParsedCommand, registry: &CommandRegistry) -> Option<CommandOutput> {
    match cmd.name.as_str() {
        "help" => {
            let text = if let Some(ref sub) = cmd.subcommand {
                registry
                    .help_for(sub)
                    .unwrap_or_else(|| format!("Unknown command: /{sub}"))
            } else {
                registry.help_all()
            };
            Some(CommandOutput::success(text))
        }
        "clear" => Some(CommandOutput::success("")),
        "exit" => Some(CommandOutput::success("Goodbye.")),
        _ => {
            if registry.get(&cmd.name).is_none() {
                Some(CommandOutput::not_found(&cmd.name))
            } else {
                None // dispatch to API
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_simple_command() {
        let cmd = parse("/help").unwrap();
        assert_eq!(cmd.name, "help");
        assert!(cmd.subcommand.is_none());
        assert!(cmd.args.is_empty());
    }

    #[test]
    fn parse_with_subcommand() {
        let cmd = parse("/wallet balance").unwrap();
        assert_eq!(cmd.name, "wallet");
        assert_eq!(cmd.subcommand.as_deref(), Some("balance"));
    }

    #[test]
    fn parse_with_args() {
        let cmd = parse("/message send hello world").unwrap();
        assert_eq!(cmd.name, "message");
        assert_eq!(cmd.subcommand.as_deref(), Some("send"));
        assert_eq!(cmd.args, vec!["hello", "world"]);
    }

    #[test]
    fn parse_with_flags() {
        let cmd = parse("/wallet send --to did:key:z6Mk --amount 100").unwrap();
        assert_eq!(cmd.name, "wallet");
        assert_eq!(cmd.subcommand.as_deref(), Some("send"));
        assert_eq!(cmd.flags.get("to"), Some(&Some("did:key:z6Mk".to_string())));
        assert_eq!(cmd.flags.get("amount"), Some(&Some("100".to_string())));
    }

    #[test]
    fn parse_with_equals_flag() {
        let cmd = parse("/node config --key=value").unwrap();
        assert_eq!(cmd.name, "node");
        assert_eq!(cmd.flags.get("key"), Some(&Some("value".to_string())));
    }

    #[test]
    fn parse_boolean_flag() {
        let cmd = parse("/peer list --verbose").unwrap();
        assert_eq!(cmd.flags.get("verbose"), Some(&None));
    }

    #[test]
    fn parse_short_flag() {
        let cmd = parse("/peer list -v").unwrap();
        assert_eq!(cmd.flags.get("v"), Some(&None));
    }

    #[test]
    fn parse_quoted_args() {
        let cmd = parse(r#"/message send "hello world""#).unwrap();
        assert_eq!(cmd.name, "message");
        assert_eq!(cmd.subcommand.as_deref(), Some("send"));
        assert_eq!(cmd.args, vec!["hello world"]);
    }

    #[test]
    fn parse_single_quoted_args() {
        let cmd = parse("/message send 'hello world'").unwrap();
        assert_eq!(cmd.args, vec!["hello world"]);
    }

    #[test]
    fn non_command_returns_none() {
        assert!(parse("echo hello").is_none());
        assert!(parse("").is_none());
        assert!(parse("/").is_none());
    }

    #[test]
    fn case_insensitive_command_name() {
        let cmd = parse("/HELP").unwrap();
        assert_eq!(cmd.name, "help");

        let cmd = parse("/Wallet Balance").unwrap();
        assert_eq!(cmd.name, "wallet");
    }

    #[test]
    fn whitespace_handling() {
        let cmd = parse("  /help  ").unwrap();
        assert_eq!(cmd.name, "help");

        let cmd = parse("/wallet   balance   --verbose").unwrap();
        assert_eq!(cmd.name, "wallet");
        assert_eq!(cmd.subcommand.as_deref(), Some("balance"));
    }

    #[test]
    fn registry_has_all_commands() {
        let reg = CommandRegistry::new();
        let names = reg.command_names();
        assert!(names.contains(&"help"));
        assert!(names.contains(&"identity"));
        assert!(names.contains(&"wallet"));
        assert!(names.contains(&"message"));
        assert!(names.contains(&"peer"));
        assert!(names.contains(&"governance"));
        assert!(names.contains(&"file"));
        assert!(names.contains(&"node"));
        assert!(names.contains(&"clear"));
        assert!(names.contains(&"exit"));
    }

    #[test]
    fn registry_subcommands() {
        let reg = CommandRegistry::new();
        let subs = reg.subcommand_names("wallet");
        assert!(subs.contains(&"balance"));
        assert!(subs.contains(&"send"));
        assert!(subs.contains(&"receive"));
        assert!(subs.contains(&"history"));
    }

    #[test]
    fn registry_no_subcommands_for_simple() {
        let reg = CommandRegistry::new();
        let subs = reg.subcommand_names("clear");
        assert!(subs.is_empty());
    }

    #[test]
    fn help_all_format() {
        let reg = CommandRegistry::new();
        let help = reg.help_all();
        assert!(help.contains("Available commands:"));
        assert!(help.contains("/help"));
        assert!(help.contains("/wallet"));
        assert!(help.contains("Type /help <command>"));
    }

    #[test]
    fn help_for_specific() {
        let reg = CommandRegistry::new();
        let help = reg.help_for("identity").unwrap();
        assert!(help.contains("/identity"));
        assert!(help.contains("Identity management"));
        assert!(help.contains("Subcommands:"));
        assert!(help.contains("show"));
        assert!(help.contains("create"));
    }

    #[test]
    fn help_for_unknown() {
        let reg = CommandRegistry::new();
        assert!(reg.help_for("nonexistent").is_none());
    }

    #[test]
    fn execute_help() {
        let reg = CommandRegistry::new();
        let cmd = parse("/help").unwrap();
        let output = execute_builtin(&cmd, &reg).unwrap();
        assert_eq!(output.status, CommandStatus::Success);
        assert!(output.text.contains("Available commands:"));
    }

    #[test]
    fn execute_help_specific() {
        let reg = CommandRegistry::new();
        let cmd = parse("/help wallet").unwrap();
        let output = execute_builtin(&cmd, &reg).unwrap();
        assert_eq!(output.status, CommandStatus::Success);
        assert!(output.text.contains("balance"));
    }

    #[test]
    fn execute_unknown_command() {
        let reg = CommandRegistry::new();
        let cmd = parse("/foobar").unwrap();
        let output = execute_builtin(&cmd, &reg).unwrap();
        assert_eq!(output.status, CommandStatus::NotFound);
    }

    #[test]
    fn execute_api_command_returns_none() {
        let reg = CommandRegistry::new();
        let cmd = parse("/wallet balance").unwrap();
        assert!(execute_builtin(&cmd, &reg).is_none());
    }

    #[test]
    fn tokenize_empty() {
        assert!(tokenize("").is_empty());
    }

    #[test]
    fn tokenize_preserves_quotes() {
        let tokens = tokenize(r#"send "hello world" --to peer"#);
        assert_eq!(tokens, vec!["send", "hello world", "--to", "peer"]);
    }
}
