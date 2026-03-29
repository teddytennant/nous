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

    /// File management
    #[command(subcommand)]
    File(FileCommand),

    /// Messaging
    #[command(subcommand)]
    Message(MessageCommand),

    /// Marketplace
    #[command(subcommand)]
    Marketplace(MarketplaceCommand),

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

#[derive(Debug, Subcommand)]
pub enum FileCommand {
    /// Upload a file to the local file store
    Upload {
        /// Path to the file to upload
        path: String,
        /// Owner DID
        #[arg(long)]
        owner: String,
    },
    /// Download a file by content ID
    Download {
        /// Content ID of the file manifest
        content_id: String,
        /// Output file path
        #[arg(short, long)]
        output: String,
    },
    /// List files for an owner
    List {
        /// Owner DID
        #[arg(long)]
        owner: String,
    },
    /// Show version history for a file
    Versions {
        /// File name
        name: String,
        /// Owner DID
        #[arg(long)]
        owner: String,
    },
    /// Show file store statistics
    Stats,
    /// Share a file with another identity
    Share {
        /// File name
        name: String,
        /// Owner DID
        #[arg(long)]
        owner: String,
        /// DID to share with
        #[arg(long = "with")]
        with: String,
    },
    /// Encrypted vault operations
    #[command(subcommand)]
    Vault(VaultCommand),
}

#[derive(Debug, Subcommand)]
pub enum VaultCommand {
    /// Create an encrypted vault
    Create {
        /// Vault password
        #[arg(long)]
        password: String,
        /// Vault name
        #[arg(long, default_value = "default")]
        name: String,
    },
    /// Store a file in a vault
    Store {
        /// Path to the file to store
        path: String,
        /// Vault ID
        #[arg(long)]
        vault_id: String,
        /// Vault password
        #[arg(long)]
        password: String,
    },
}

#[derive(Debug, Subcommand)]
pub enum MessageCommand {
    /// Send a message to a channel
    Send {
        /// Channel ID
        channel_id: String,
        /// Message text
        text: String,
        /// Sender DID (uses active identity if omitted)
        #[arg(long)]
        sender: Option<String>,
    },
    /// List messages in a channel
    List {
        /// Channel ID
        channel_id: String,
        /// Maximum messages to show
        #[arg(short, long, default_value = "20")]
        limit: usize,
    },
    /// List channels a DID is member of
    Channels {
        /// Member DID (uses active identity if omitted)
        #[arg(long)]
        member: Option<String>,
    },
    /// Create a new channel
    CreateChannel {
        /// Channel kind: direct, group, or public
        #[arg(long)]
        kind: String,
        /// Channel name (required for group and public)
        #[arg(long)]
        name: Option<String>,
        /// Member DIDs (can be specified multiple times)
        #[arg(long = "member")]
        members: Vec<String>,
    },
}

#[derive(Debug, Subcommand)]
pub enum MarketplaceCommand {
    /// Create a new listing
    List {
        /// Title of the listing
        title: String,
        /// Description
        #[arg(short, long)]
        description: Option<String>,
        /// Category: physical, digital, service, nft, data, other
        #[arg(short, long, default_value = "digital")]
        category: String,
        /// Price token (e.g., ETH, USDC, NOUS)
        #[arg(long, default_value = "NOUS")]
        token: String,
        /// Price amount in minor units
        #[arg(short, long)]
        price: u128,
        /// Tags (comma-separated)
        #[arg(short, long)]
        tags: Option<String>,
    },
    /// Search listings
    Search {
        /// Search query text
        query: Option<String>,
        /// Filter by category
        #[arg(short, long)]
        category: Option<String>,
        /// Maximum results
        #[arg(short, long, default_value = "20")]
        limit: usize,
    },
    /// Show listing details
    Show {
        /// Listing ID
        id: String,
    },
    /// Create an order for a listing
    Order {
        /// Listing ID to purchase
        listing_id: String,
        /// Quantity
        #[arg(short, long, default_value = "1")]
        quantity: u32,
    },
    /// List your orders
    Orders {
        /// Filter: buying or selling
        #[arg(long)]
        role: Option<String>,
        /// Maximum results
        #[arg(short, long, default_value = "20")]
        limit: usize,
    },
    /// Make an offer on a listing
    Offer {
        /// Listing ID
        listing_id: String,
        /// Offer amount
        amount: u128,
        /// Token
        #[arg(long, default_value = "NOUS")]
        token: String,
        /// Optional message
        #[arg(short, long)]
        message: Option<String>,
    },
    /// List offers (sent or received)
    Offers {
        /// Filter by listing
        #[arg(long)]
        listing_id: Option<String>,
    },
    /// Open a dispute on an order
    Dispute {
        /// Order ID
        order_id: String,
        /// Reason: item_not_received, item_not_as_described, quality_issue, counterfeit, seller_unresponsive, other
        #[arg(short, long)]
        reason: String,
        /// Description of the dispute
        description: String,
    },
    /// Cancel a listing
    Cancel {
        /// Listing ID to cancel
        id: String,
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
    fn parse_file_upload() {
        let cli = Cli::parse_from([
            "nous",
            "file",
            "upload",
            "/tmp/test.txt",
            "--owner",
            "did:key:z123",
        ]);
        if let Command::File(FileCommand::Upload { path, owner }) = cli.command {
            assert_eq!(path, "/tmp/test.txt");
            assert_eq!(owner, "did:key:z123");
        } else {
            panic!("expected file upload");
        }
    }

    #[test]
    fn parse_file_download() {
        let cli = Cli::parse_from([
            "nous",
            "file",
            "download",
            "abc123",
            "--output",
            "/tmp/out.bin",
        ]);
        if let Command::File(FileCommand::Download { content_id, output }) = cli.command {
            assert_eq!(content_id, "abc123");
            assert_eq!(output, "/tmp/out.bin");
        } else {
            panic!("expected file download");
        }
    }

    #[test]
    fn parse_file_list() {
        let cli = Cli::parse_from(["nous", "file", "list", "--owner", "did:key:z123"]);
        if let Command::File(FileCommand::List { owner }) = cli.command {
            assert_eq!(owner, "did:key:z123");
        } else {
            panic!("expected file list");
        }
    }

    #[test]
    fn parse_file_versions() {
        let cli = Cli::parse_from([
            "nous",
            "file",
            "versions",
            "readme.md",
            "--owner",
            "did:key:z123",
        ]);
        if let Command::File(FileCommand::Versions { name, owner }) = cli.command {
            assert_eq!(name, "readme.md");
            assert_eq!(owner, "did:key:z123");
        } else {
            panic!("expected file versions");
        }
    }

    #[test]
    fn parse_file_stats() {
        let cli = Cli::parse_from(["nous", "file", "stats"]);
        assert!(matches!(cli.command, Command::File(FileCommand::Stats)));
    }

    #[test]
    fn parse_file_share() {
        let cli = Cli::parse_from([
            "nous",
            "file",
            "share",
            "doc.txt",
            "--owner",
            "did:key:zalice",
            "--with",
            "did:key:zbob",
        ]);
        if let Command::File(FileCommand::Share { name, owner, with }) = cli.command {
            assert_eq!(name, "doc.txt");
            assert_eq!(owner, "did:key:zalice");
            assert_eq!(with, "did:key:zbob");
        } else {
            panic!("expected file share");
        }
    }

    #[test]
    fn parse_file_vault_create() {
        let cli = Cli::parse_from(["nous", "file", "vault", "create", "--password", "s3cret"]);
        if let Command::File(FileCommand::Vault(VaultCommand::Create { password, name })) =
            cli.command
        {
            assert_eq!(password, "s3cret");
            assert_eq!(name, "default");
        } else {
            panic!("expected file vault create");
        }
    }

    #[test]
    fn parse_file_vault_store() {
        let cli = Cli::parse_from([
            "nous",
            "file",
            "vault",
            "store",
            "/tmp/secret.bin",
            "--vault-id",
            "vault-abc",
            "--password",
            "pass",
        ]);
        if let Command::File(FileCommand::Vault(VaultCommand::Store {
            path,
            vault_id,
            password,
        })) = cli.command
        {
            assert_eq!(path, "/tmp/secret.bin");
            assert_eq!(vault_id, "vault-abc");
            assert_eq!(password, "pass");
        } else {
            panic!("expected file vault store");
        }
    }

    #[test]
    fn parse_message_send() {
        let cli = Cli::parse_from([
            "nous",
            "message",
            "send",
            "ch-123",
            "hello world",
            "--sender",
            "did:key:zalice",
        ]);
        if let Command::Message(MessageCommand::Send {
            channel_id,
            text,
            sender,
        }) = cli.command
        {
            assert_eq!(channel_id, "ch-123");
            assert_eq!(text, "hello world");
            assert_eq!(sender, Some("did:key:zalice".to_string()));
        } else {
            panic!("expected message send");
        }
    }

    #[test]
    fn parse_message_send_no_sender() {
        let cli = Cli::parse_from(["nous", "message", "send", "ch-123", "hi"]);
        if let Command::Message(MessageCommand::Send {
            channel_id,
            text,
            sender,
        }) = cli.command
        {
            assert_eq!(channel_id, "ch-123");
            assert_eq!(text, "hi");
            assert!(sender.is_none());
        } else {
            panic!("expected message send");
        }
    }

    #[test]
    fn parse_message_list() {
        let cli = Cli::parse_from(["nous", "message", "list", "ch-123", "--limit", "50"]);
        if let Command::Message(MessageCommand::List { channel_id, limit }) = cli.command {
            assert_eq!(channel_id, "ch-123");
            assert_eq!(limit, 50);
        } else {
            panic!("expected message list");
        }
    }

    #[test]
    fn parse_message_list_default_limit() {
        let cli = Cli::parse_from(["nous", "message", "list", "ch-123"]);
        if let Command::Message(MessageCommand::List { channel_id, limit }) = cli.command {
            assert_eq!(channel_id, "ch-123");
            assert_eq!(limit, 20);
        } else {
            panic!("expected message list");
        }
    }

    #[test]
    fn parse_message_channels() {
        let cli = Cli::parse_from(["nous", "message", "channels", "--member", "did:key:zalice"]);
        if let Command::Message(MessageCommand::Channels { member }) = cli.command {
            assert_eq!(member, Some("did:key:zalice".to_string()));
        } else {
            panic!("expected message channels");
        }
    }

    #[test]
    fn parse_message_create_channel_group() {
        let cli = Cli::parse_from([
            "nous",
            "message",
            "create-channel",
            "--kind",
            "group",
            "--name",
            "engineering",
            "--member",
            "did:key:za",
            "--member",
            "did:key:zb",
        ]);
        if let Command::Message(MessageCommand::CreateChannel {
            kind,
            name,
            members,
        }) = cli.command
        {
            assert_eq!(kind, "group");
            assert_eq!(name, Some("engineering".to_string()));
            assert_eq!(members, vec!["did:key:za", "did:key:zb"]);
        } else {
            panic!("expected message create-channel");
        }
    }

    #[test]
    fn parse_message_create_channel_public() {
        let cli = Cli::parse_from([
            "nous",
            "message",
            "create-channel",
            "--kind",
            "public",
            "--name",
            "general",
        ]);
        if let Command::Message(MessageCommand::CreateChannel {
            kind,
            name,
            members,
        }) = cli.command
        {
            assert_eq!(kind, "public");
            assert_eq!(name, Some("general".to_string()));
            assert!(members.is_empty());
        } else {
            panic!("expected message create-channel");
        }
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

    #[test]
    fn parse_marketplace_list() {
        let cli = Cli::parse_from([
            "nous",
            "marketplace",
            "list",
            "Vintage Camera",
            "-p",
            "500",
            "-c",
            "physical",
            "-t",
            "vintage,camera",
        ]);
        if let Command::Marketplace(MarketplaceCommand::List {
            title,
            price,
            category,
            tags,
            ..
        }) = cli.command
        {
            assert_eq!(title, "Vintage Camera");
            assert_eq!(price, 500);
            assert_eq!(category, "physical");
            assert_eq!(tags, Some("vintage,camera".to_string()));
        } else {
            panic!("expected marketplace list");
        }
    }

    #[test]
    fn parse_marketplace_search() {
        let cli = Cli::parse_from(["nous", "marketplace", "search", "camera", "-l", "10"]);
        if let Command::Marketplace(MarketplaceCommand::Search {
            query,
            limit,
            category,
        }) = cli.command
        {
            assert_eq!(query, Some("camera".to_string()));
            assert_eq!(limit, 10);
            assert!(category.is_none());
        } else {
            panic!("expected marketplace search");
        }
    }

    #[test]
    fn parse_marketplace_search_empty() {
        let cli = Cli::parse_from(["nous", "marketplace", "search"]);
        if let Command::Marketplace(MarketplaceCommand::Search { query, limit, .. }) = cli.command {
            assert!(query.is_none());
            assert_eq!(limit, 20);
        } else {
            panic!("expected marketplace search");
        }
    }

    #[test]
    fn parse_marketplace_order() {
        let cli = Cli::parse_from(["nous", "marketplace", "order", "listing:abc", "-q", "3"]);
        if let Command::Marketplace(MarketplaceCommand::Order {
            listing_id,
            quantity,
        }) = cli.command
        {
            assert_eq!(listing_id, "listing:abc");
            assert_eq!(quantity, 3);
        } else {
            panic!("expected marketplace order");
        }
    }

    #[test]
    fn parse_marketplace_offer() {
        let cli = Cli::parse_from([
            "nous",
            "marketplace",
            "offer",
            "listing:abc",
            "400",
            "--token",
            "ETH",
            "-m",
            "Would you take 400?",
        ]);
        if let Command::Marketplace(MarketplaceCommand::Offer {
            listing_id,
            amount,
            token,
            message,
        }) = cli.command
        {
            assert_eq!(listing_id, "listing:abc");
            assert_eq!(amount, 400);
            assert_eq!(token, "ETH");
            assert_eq!(message, Some("Would you take 400?".to_string()));
        } else {
            panic!("expected marketplace offer");
        }
    }

    #[test]
    fn parse_marketplace_dispute() {
        let cli = Cli::parse_from([
            "nous",
            "marketplace",
            "dispute",
            "order:xyz",
            "-r",
            "item_not_received",
            "Never arrived",
        ]);
        if let Command::Marketplace(MarketplaceCommand::Dispute {
            order_id,
            reason,
            description,
        }) = cli.command
        {
            assert_eq!(order_id, "order:xyz");
            assert_eq!(reason, "item_not_received");
            assert_eq!(description, "Never arrived");
        } else {
            panic!("expected marketplace dispute");
        }
    }

    #[test]
    fn parse_marketplace_cancel() {
        let cli = Cli::parse_from(["nous", "marketplace", "cancel", "listing:abc"]);
        if let Command::Marketplace(MarketplaceCommand::Cancel { id }) = cli.command {
            assert_eq!(id, "listing:abc");
        } else {
            panic!("expected marketplace cancel");
        }
    }
}
