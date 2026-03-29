use clap::{Parser, Subcommand};

#[derive(Debug, Parser)]
#[command(
    name = "nous",
    version,
    about = "Nous — sovereign decentralized everything-app",
    long_about = None,
)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Command,

    /// Output as JSON
    #[arg(long, global = true)]
    pub json: bool,

    /// Verbose logging
    #[arg(short, long, global = true)]
    pub verbose: bool,
}

#[derive(Debug, Subcommand)]
pub enum Command {
    /// Initialize a new Nous identity
    Init,

    /// Identity management
    #[command(subcommand)]
    Identity(IdentityCommand),

    /// Social features
    #[command(subcommand)]
    Social(SocialCommand),

    /// Wallet and payments
    #[command(subcommand)]
    Wallet(WalletCommand),

    /// Network and peers
    #[command(subcommand)]
    Net(NetCommand),

    /// Governance and DAOs
    #[command(subcommand)]
    Governance(GovernanceCommand),

    /// Node information
    Status,

    /// Launch an embedded terminal
    Terminal,
}

#[derive(Debug, Subcommand)]
pub enum IdentityCommand {
    /// Show current identity
    Show,
    /// Generate a new keypair
    Generate,
    /// Export public key
    Export,
    /// List all identities
    List,
}

#[derive(Debug, Subcommand)]
pub enum SocialCommand {
    /// Post a text note
    Post {
        /// The content to post
        content: String,
        /// Hashtags (comma-separated)
        #[arg(short, long)]
        tags: Option<String>,
    },
    /// Show your feed
    Feed {
        /// Maximum posts to show
        #[arg(short, long, default_value = "20")]
        limit: usize,
    },
    /// Follow a DID
    Follow {
        /// The DID to follow
        did: String,
    },
    /// Unfollow a DID
    Unfollow {
        /// The DID to unfollow
        did: String,
    },
    /// Show who you follow
    Following,
}

#[derive(Debug, Subcommand)]
pub enum WalletCommand {
    /// Show balances
    Balance,
    /// Send tokens
    Send {
        /// Recipient DID
        to: String,
        /// Token name
        token: String,
        /// Amount to send
        amount: u128,
        /// Optional memo
        #[arg(short, long)]
        memo: Option<String>,
    },
    /// Show transaction history
    History {
        #[arg(short, long, default_value = "20")]
        limit: usize,
    },
}

#[derive(Debug, Subcommand)]
pub enum NetCommand {
    /// Show connected peers
    Peers,
    /// Show network status
    Status,
    /// Connect to a peer
    Connect {
        /// Multiaddr of the peer
        addr: String,
    },
}

#[derive(Debug, Subcommand)]
pub enum GovernanceCommand {
    /// Create a DAO
    CreateDao {
        /// DAO name
        name: String,
        /// Description
        #[arg(short, long)]
        description: Option<String>,
    },
    /// List DAOs
    ListDaos,
    /// Show DAO details
    ShowDao {
        /// DAO ID
        id: String,
    },
    /// Submit a proposal
    Propose {
        /// DAO ID
        dao_id: String,
        /// Proposal title
        title: String,
        /// Description
        #[arg(short, long)]
        description: Option<String>,
        /// Voting duration in days
        #[arg(long, default_value = "7")]
        voting_days: u64,
    },
    /// List proposals
    ListProposals {
        /// Filter by DAO ID
        #[arg(long)]
        dao_id: Option<String>,
    },
    /// Vote on a proposal
    Vote {
        /// Proposal ID
        proposal_id: String,
        /// Vote choice: for, against, abstain
        choice: String,
        /// Quadratic voting credits
        #[arg(short, long, default_value = "1")]
        credits: u64,
    },
    /// Show vote tally
    Tally {
        /// Proposal ID
        proposal_id: String,
    },
}

#[cfg(test)]
mod tests {
    use super::*;
    use clap::CommandFactory;

    #[test]
    fn cli_parses() {
        // Verify CLI definition is valid
        Cli::command().debug_assert();
    }

    #[test]
    fn parse_init() {
        let cli = Cli::parse_from(["nous", "init"]);
        assert!(matches!(cli.command, Command::Init));
    }

    #[test]
    fn parse_status() {
        let cli = Cli::parse_from(["nous", "status"]);
        assert!(matches!(cli.command, Command::Status));
    }

    #[test]
    fn parse_identity_show() {
        let cli = Cli::parse_from(["nous", "identity", "show"]);
        assert!(matches!(
            cli.command,
            Command::Identity(IdentityCommand::Show)
        ));
    }

    #[test]
    fn parse_social_post() {
        let cli = Cli::parse_from(["nous", "social", "post", "hello world"]);
        if let Command::Social(SocialCommand::Post { content, tags }) = cli.command {
            assert_eq!(content, "hello world");
            assert!(tags.is_none());
        } else {
            panic!("expected social post");
        }
    }

    #[test]
    fn parse_social_post_with_tags() {
        let cli = Cli::parse_from(["nous", "social", "post", "hello", "-t", "nous,web3"]);
        if let Command::Social(SocialCommand::Post { tags, .. }) = cli.command {
            assert_eq!(tags, Some("nous,web3".to_string()));
        } else {
            panic!("expected social post");
        }
    }

    #[test]
    fn parse_wallet_send() {
        let cli = Cli::parse_from([
            "nous",
            "wallet",
            "send",
            "did:key:bob",
            "ETH",
            "100",
            "-m",
            "for coffee",
        ]);
        if let Command::Wallet(WalletCommand::Send {
            to,
            token,
            amount,
            memo,
        }) = cli.command
        {
            assert_eq!(to, "did:key:bob");
            assert_eq!(token, "ETH");
            assert_eq!(amount, 100);
            assert_eq!(memo, Some("for coffee".to_string()));
        } else {
            panic!("expected wallet send");
        }
    }

    #[test]
    fn parse_net_connect() {
        let cli = Cli::parse_from(["nous", "net", "connect", "/ip4/127.0.0.1/tcp/4001"]);
        if let Command::Net(NetCommand::Connect { addr }) = cli.command {
            assert_eq!(addr, "/ip4/127.0.0.1/tcp/4001");
        } else {
            panic!("expected net connect");
        }
    }

    #[test]
    fn json_flag() {
        let cli = Cli::parse_from(["nous", "--json", "status"]);
        assert!(cli.json);
    }

    #[test]
    fn verbose_flag() {
        let cli = Cli::parse_from(["nous", "-v", "status"]);
        assert!(cli.verbose);
    }

    #[test]
    fn parse_governance_create_dao() {
        let cli = Cli::parse_from(["nous", "governance", "create-dao", "TestDAO"]);
        if let Command::Governance(GovernanceCommand::CreateDao { name, description }) = cli.command
        {
            assert_eq!(name, "TestDAO");
            assert!(description.is_none());
        } else {
            panic!("expected governance create-dao");
        }
    }

    #[test]
    fn parse_governance_propose() {
        let cli = Cli::parse_from([
            "nous",
            "governance",
            "propose",
            "dao123",
            "Fund treasury",
            "-d",
            "Allocate 1000 NOUS to treasury",
        ]);
        if let Command::Governance(GovernanceCommand::Propose {
            dao_id,
            title,
            description,
            voting_days,
        }) = cli.command
        {
            assert_eq!(dao_id, "dao123");
            assert_eq!(title, "Fund treasury");
            assert_eq!(
                description,
                Some("Allocate 1000 NOUS to treasury".to_string())
            );
            assert_eq!(voting_days, 7);
        } else {
            panic!("expected governance propose");
        }
    }

    #[test]
    fn parse_governance_vote() {
        let cli = Cli::parse_from(["nous", "governance", "vote", "prop123", "for", "-c", "4"]);
        if let Command::Governance(GovernanceCommand::Vote {
            proposal_id,
            choice,
            credits,
        }) = cli.command
        {
            assert_eq!(proposal_id, "prop123");
            assert_eq!(choice, "for");
            assert_eq!(credits, 4);
        } else {
            panic!("expected governance vote");
        }
    }

    #[test]
    fn parse_terminal() {
        let cli = Cli::parse_from(["nous", "terminal"]);
        assert!(matches!(cli.command, Command::Terminal));
    }

    #[test]
    fn parse_governance_list_daos() {
        let cli = Cli::parse_from(["nous", "governance", "list-daos"]);
        assert!(matches!(
            cli.command,
            Command::Governance(GovernanceCommand::ListDaos)
        ));
    }

    #[test]
    fn parse_governance_tally() {
        let cli = Cli::parse_from(["nous", "governance", "tally", "prop123"]);
        if let Command::Governance(GovernanceCommand::Tally { proposal_id }) = cli.command {
            assert_eq!(proposal_id, "prop123");
        } else {
            panic!("expected governance tally");
        }
    }
}
