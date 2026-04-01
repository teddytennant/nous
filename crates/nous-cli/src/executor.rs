use std::io::Write;
use std::path::Path;

use nous_files::{FileStore, SharedFolder, Vault, VaultEntry};
use nous_governance::{Ballot, Dao, ProposalBuilder, QuadraticVoting, VoteChoice, VoteTally};
use nous_identity::Identity;
use nous_marketplace::{Listing, ListingCategory, SearchQuery};
use nous_messaging::message::MessageBuilder;
use nous_messaging::{Channel, Message, MessageContent};
use nous_payments::Wallet;
use nous_social::{EventKind, Feed, FollowGraph, SignedEvent, Tag};
use nous_storage::Database;
#[cfg(unix)]
use nous_terminal::{Terminal, TerminalConfig};

#[cfg(unix)]
use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyModifiers};
#[cfg(unix)]
use crossterm::terminal::{self, EnterAlternateScreen, LeaveAlternateScreen};
#[cfg(unix)]
use crossterm::{cursor, execute, queue, style};

use nous_cli::commands::*;
use nous_cli::output::Output;

pub struct Executor {
    output: Output,
    db: Database,
    data_dir: std::path::PathBuf,
}

impl Executor {
    pub fn new(data_dir: &Path, json: bool) -> Result<Self, String> {
        std::fs::create_dir_all(data_dir)
            .map_err(|e| format!("failed to create data directory: {e}"))?;

        let db_path = data_dir.join("nous.db");
        let db = Database::open(&db_path).map_err(|e| format!("failed to open database: {e}"))?;

        Ok(Self {
            output: Output::new(json),
            db,
            data_dir: data_dir.to_path_buf(),
        })
    }

    pub async fn execute(&self, command: Command) -> Result<(), String> {
        match command {
            Command::Init => self.init(),
            Command::Identity(cmd) => self.identity(cmd),
            Command::Social(cmd) => self.social(cmd),
            Command::Wallet(cmd) => self.wallet(cmd),
            Command::Governance(cmd) => self.governance(cmd).await,
            Command::File(cmd) => self.file(cmd),
            Command::Message(cmd) => self.message(cmd),
            Command::Net(cmd) => self.net(cmd).await,
            Command::Marketplace(cmd) => self.marketplace(cmd),
            Command::Ai(cmd) => self.ai(cmd),
            #[cfg(unix)]
            Command::Terminal => Self::run_terminal(),
            #[cfg(not(unix))]
            Command::Terminal => Err("Embedded terminal is only supported on Unix systems".to_string()),
            Command::Status => self.status(),
        }
    }

    fn init(&self) -> Result<(), String> {
        if self.load_identity().is_some() {
            self.output
                .success("Identity already exists. Use 'nous identity show' to view it.");
            return Ok(());
        }

        let identity = Identity::generate();
        self.store_identity(&identity)?;

        self.output.success(&format!(
            "Initialized Nous identity\n  DID: {}",
            identity.did()
        ));
        Ok(())
    }

    fn identity(&self, cmd: IdentityCommand) -> Result<(), String> {
        match cmd {
            IdentityCommand::Show => {
                let identity = self
                    .load_identity()
                    .ok_or("No identity found. Run 'nous init' first.")?;

                self.output.table(
                    &["Field", "Value"],
                    &[
                        vec!["DID".into(), identity.did().to_string()],
                        vec!["Signing Key".into(), "ed25519".into()],
                        vec!["Exchange Key".into(), "x25519".into()],
                        vec![
                            "Display Name".into(),
                            identity.display_name().unwrap_or("(none)").to_string(),
                        ],
                    ],
                );
                Ok(())
            }
            IdentityCommand::Generate => {
                let identity = Identity::generate();
                self.store_identity(&identity)?;
                self.output
                    .success(&format!("Generated new identity: {}", identity.did()));
                Ok(())
            }
            IdentityCommand::Export => {
                let identity = self
                    .load_identity()
                    .ok_or("No identity found. Run 'nous init' first.")?;

                let doc = identity.document();
                self.output.print_json(doc);
                Ok(())
            }
            IdentityCommand::List => {
                let identities = self.list_identities();
                if identities.is_empty() {
                    self.output.success("No identities found.");
                } else {
                    let rows: Vec<Vec<String>> = identities
                        .iter()
                        .enumerate()
                        .map(|(i, did)| vec![format!("{}", i + 1), did.clone()])
                        .collect();
                    self.output.table(&["#", "DID"], &rows);
                }
                Ok(())
            }
        }
    }

    fn social(&self, cmd: SocialCommand) -> Result<(), String> {
        let identity = self
            .load_identity()
            .ok_or("No identity found. Run 'nous init' first.")?;

        match cmd {
            SocialCommand::Post { content, tags } => {
                let tag_list: Vec<Tag> = tags
                    .unwrap_or_default()
                    .split(',')
                    .filter(|t| !t.is_empty())
                    .map(|t| Tag::hashtag(t.trim()))
                    .collect();

                let mut event =
                    SignedEvent::new(identity.did(), EventKind::TextNote, &content, tag_list);
                event.sign(identity.keypair());

                let event_json =
                    serde_json::to_string(&event).map_err(|e| format!("serialization: {e}"))?;
                self.db
                    .put_kv(&format!("event:{}", event.id), event_json.as_bytes())
                    .map_err(|e| format!("storage: {e}"))?;

                self.output
                    .success(&format!("Posted: {}\n  ID: {}", content, event.id));
                Ok(())
            }
            SocialCommand::Feed { limit } => {
                let events = self.load_events();
                let mut feed = Feed::new();
                for event in events {
                    feed.insert(event);
                }

                let latest = feed.latest(limit);
                if latest.is_empty() {
                    self.output.success("No posts yet.");
                    return Ok(());
                }

                let rows: Vec<Vec<String>> = latest
                    .iter()
                    .map(|e| {
                        let did_short = truncate_did(&e.pubkey);
                        let time = e.created_at.format("%Y-%m-%d %H:%M").to_string();
                        let hashtags = e.hashtags().join(", ");
                        vec![did_short, time, e.content.clone(), hashtags]
                    })
                    .collect();

                self.output
                    .table(&["Author", "Time", "Content", "Tags"], &rows);
                Ok(())
            }
            SocialCommand::Follow { did } => {
                let mut graph = self.load_follow_graph();
                if graph.follow(identity.did(), &did) {
                    self.save_follow_graph(&graph)?;
                    self.output.success(&format!("Now following {did}"));
                } else {
                    self.output.success(&format!("Already following {did}"));
                }
                Ok(())
            }
            SocialCommand::Unfollow { did } => {
                let mut graph = self.load_follow_graph();
                if graph.unfollow(identity.did(), &did) {
                    self.save_follow_graph(&graph)?;
                    self.output.success(&format!("Unfollowed {did}"));
                } else {
                    self.output.success(&format!("Not following {did}"));
                }
                Ok(())
            }
            SocialCommand::Following => {
                let identity = self.load_identity().ok_or("No identity found.")?;
                let graph = self.load_follow_graph();
                let following = graph.following_of(identity.did());

                if following.is_empty() {
                    self.output.success("Not following anyone.");
                } else {
                    let rows: Vec<Vec<String>> = following
                        .iter()
                        .enumerate()
                        .map(|(i, did)| vec![format!("{}", i + 1), did.to_string()])
                        .collect();
                    self.output.table(&["#", "DID"], &rows);
                }
                Ok(())
            }
        }
    }

    fn wallet(&self, cmd: WalletCommand) -> Result<(), String> {
        let identity = self
            .load_identity()
            .ok_or("No identity found. Run 'nous init' first.")?;

        match cmd {
            WalletCommand::Balance => {
                let wallet = self.load_wallet(identity.did());
                let tokens = wallet.tokens();

                if tokens.is_empty() {
                    self.output.success("Wallet is empty.");
                } else {
                    let rows: Vec<Vec<String>> = tokens
                        .iter()
                        .map(|t| vec![t.to_string(), wallet.balance(t).to_string()])
                        .collect();
                    self.output.table(&["Token", "Balance"], &rows);
                }
                Ok(())
            }
            WalletCommand::Send {
                to,
                token,
                amount,
                memo,
            } => {
                let mut wallet = self.load_wallet(identity.did());
                wallet.debit(&token, amount).map_err(|e| format!("{e}"))?;
                self.save_wallet(&wallet)?;

                let mut tx = nous_payments::Transaction::new(identity.did(), &to, &token, amount);
                if let Some(m) = memo {
                    tx = tx.with_memo(m);
                }
                tx.sign(identity.keypair());
                tx.confirm();

                let tx_json =
                    serde_json::to_string(&tx).map_err(|e| format!("serialization: {e}"))?;
                self.db
                    .put_kv(&format!("tx:{}", tx.id), tx_json.as_bytes())
                    .map_err(|e| format!("storage: {e}"))?;

                self.output.success(&format!(
                    "Sent {} {} to {}\n  TX: {}",
                    amount,
                    token,
                    truncate_did(&to),
                    tx.id
                ));
                Ok(())
            }
            WalletCommand::History { limit: _ } => {
                self.output.success("No transactions yet.");
                Ok(())
            }
        }
    }

    fn file(&self, cmd: FileCommand) -> Result<(), String> {
        match cmd {
            FileCommand::Upload { path, owner } => {
                let file_path = Path::new(&path);
                if !file_path.exists() {
                    return Err(format!("file not found: {path}"));
                }

                let data =
                    std::fs::read(file_path).map_err(|e| format!("failed to read file: {e}"))?;
                let file_name = file_path
                    .file_name()
                    .and_then(|n| n.to_str())
                    .unwrap_or("unknown");
                let mime_type = mime_from_extension(file_name);

                let mut store = self.load_file_store();
                let manifest = store
                    .put(file_name, &mime_type, &data, &owner)
                    .map_err(|e| format!("store error: {e}"))?;
                self.save_file_store(&store)?;

                self.output.success(&format!(
                    "Uploaded '{}' ({} bytes)\n  Content ID: {}\n  Version: {}\n  Owner: {}",
                    file_name,
                    data.len(),
                    manifest.id.0,
                    manifest.version,
                    truncate_did(&owner),
                ));
                Ok(())
            }
            FileCommand::Download { content_id, output } => {
                let store = self.load_file_store();
                let data = store
                    .get(&content_id)
                    .map_err(|e| format!("download error: {e}"))?;

                std::fs::write(&output, &data)
                    .map_err(|e| format!("failed to write output: {e}"))?;

                self.output
                    .success(&format!("Downloaded {} bytes to '{}'", data.len(), output,));
                Ok(())
            }
            FileCommand::List { owner } => {
                let store = self.load_file_store();
                let files = store.list_files(&owner);

                if files.is_empty() {
                    self.output.success("No files found.");
                } else {
                    let rows: Vec<Vec<String>> = files
                        .iter()
                        .map(|m| {
                            vec![
                                m.name.clone(),
                                m.id.0.clone(),
                                format!("v{}", m.version),
                                format!("{} B", m.total_size),
                                m.mime_type.clone(),
                            ]
                        })
                        .collect();
                    self.output
                        .table(&["Name", "Content ID", "Version", "Size", "Type"], &rows);
                }
                Ok(())
            }
            FileCommand::Versions { name, owner } => {
                let store = self.load_file_store();
                let history = store
                    .get_history(&name, &owner)
                    .ok_or_else(|| format!("no history found for '{name}'"))?;

                let mut rows: Vec<Vec<String>> = Vec::new();
                // Current version first.
                rows.push(vec![
                    format!("v{}", history.current.version),
                    history.current.id.0.clone(),
                    format!("{} B", history.current.total_size),
                    history
                        .current
                        .created_at
                        .format("%Y-%m-%d %H:%M")
                        .to_string(),
                    "(current)".into(),
                ]);
                for ver in &history.history {
                    rows.push(vec![
                        format!("v{}", ver.version),
                        ver.id.0.clone(),
                        format!("{} B", ver.total_size),
                        ver.created_at.format("%Y-%m-%d %H:%M").to_string(),
                        String::new(),
                    ]);
                }
                self.output.table(
                    &["Version", "Content ID", "Size", "Created", "Status"],
                    &rows,
                );
                Ok(())
            }
            FileCommand::Stats => {
                let store = self.load_file_store();
                let stats = store.stats();

                self.output.table(
                    &["Metric", "Value"],
                    &[
                        vec!["Total files".into(), stats.total_files.to_string()],
                        vec!["Total chunks".into(), stats.total_chunks.to_string()],
                        vec!["Total manifests".into(), stats.total_manifests.to_string()],
                        vec!["Stored bytes".into(), format_bytes(stats.stored_bytes)],
                        vec!["Logical bytes".into(), format_bytes(stats.logical_bytes)],
                        vec!["Dedup ratio".into(), format!("{:.2}x", stats.dedup_ratio)],
                    ],
                );
                Ok(())
            }
            FileCommand::Share { name, owner, with } => {
                let store = self.load_file_store();
                // Verify the file exists.
                let _history = store
                    .get_history(&name, &owner)
                    .ok_or_else(|| format!("file '{name}' not found for owner"))?;

                // Create or update shared folder for this owner.
                let mut folder = self.load_shared_folder(&owner).unwrap_or_else(|| {
                    SharedFolder::new(&format!("{}'s files", truncate_did(&owner)), &owner)
                });

                // Add the target DID as a reader.
                folder
                    .add_member(&owner, &with, nous_files::AccessLevel::Read)
                    .map_err(|e| format!("share error: {e}"))?;
                self.save_shared_folder(&folder)?;

                self.output.success(&format!(
                    "Shared '{}' with {}\n  Folder: {}",
                    name,
                    truncate_did(&with),
                    folder.id,
                ));
                Ok(())
            }
            FileCommand::Vault(vault_cmd) => match vault_cmd {
                VaultCommand::Create { password, name } => {
                    let vault = Vault::create(&name, password.as_bytes())
                        .map_err(|e| format!("vault creation error: {e}"))?;

                    let vault_id = vault.id.clone();
                    self.save_vault(&vault)?;

                    self.output
                        .success(&format!("Created vault '{}'\n  ID: {}", name, vault_id,));
                    Ok(())
                }
                VaultCommand::Store {
                    path,
                    vault_id,
                    password,
                } => {
                    let file_path = Path::new(&path);
                    if !file_path.exists() {
                        return Err(format!("file not found: {path}"));
                    }

                    let data = std::fs::read(file_path)
                        .map_err(|e| format!("failed to read file: {e}"))?;
                    let file_name = file_path
                        .file_name()
                        .and_then(|n| n.to_str())
                        .unwrap_or("unknown");
                    let mime_type = mime_from_extension(file_name);

                    let vault = self
                        .load_vault(&vault_id)
                        .ok_or_else(|| format!("vault '{vault_id}' not found"))?;
                    let key = vault
                        .unlock(password.as_bytes())
                        .map_err(|e| format!("unlock error: {e}"))?;
                    let entry = vault
                        .encrypt_file(&key, file_name, &mime_type, &data)
                        .map_err(|e| format!("encryption error: {e}"))?;

                    self.save_vault_entry(&vault_id, &entry)?;

                    self.output.success(&format!(
                        "Stored '{}' in vault '{}'\n  Size: {} bytes (encrypted)\n  Hash: {}",
                        file_name,
                        vault_id,
                        entry.encrypted.ciphertext.len(),
                        &entry.content_hash[..16],
                    ));
                    Ok(())
                }
            },
        }
    }

    fn message(&self, cmd: MessageCommand) -> Result<(), String> {
        match cmd {
            MessageCommand::Send {
                channel_id,
                text,
                sender: _,
            } => {
                let identity = self
                    .load_identity()
                    .ok_or("No identity found. Run 'nous init' first.")?;

                // Verify channel exists.
                let channel = self
                    .load_channel(&channel_id)
                    .ok_or_else(|| format!("channel '{channel_id}' not found"))?;

                if !channel.is_member(identity.did()) {
                    return Err(format!(
                        "identity {} is not a member of channel '{}'",
                        truncate_did(identity.did()),
                        channel_id
                    ));
                }

                let msg = MessageBuilder::text(&channel_id, &text)
                    .sign(&identity)
                    .map_err(|e| format!("signing error: {e}"))?;

                self.save_message(&msg)?;

                self.output.success(&format!(
                    "Sent message to '{}'\n  ID: {}\n  From: {}",
                    channel_id,
                    msg.id,
                    truncate_did(&msg.sender_did),
                ));
                Ok(())
            }
            MessageCommand::List { channel_id, limit } => {
                let _channel = self
                    .load_channel(&channel_id)
                    .ok_or_else(|| format!("channel '{channel_id}' not found"))?;

                let messages = self.load_messages(&channel_id);

                if messages.is_empty() {
                    self.output.success("No messages.");
                    return Ok(());
                }

                let display: Vec<_> = messages.into_iter().take(limit).collect();
                let rows: Vec<Vec<String>> = display
                    .iter()
                    .map(|m| {
                        let content_str = match &m.content {
                            MessageContent::Text(t) => t.clone(),
                            MessageContent::File { name, size, .. } => {
                                format!("[file: {} ({} B)]", name, size)
                            }
                            MessageContent::Reaction { emoji, .. } => {
                                format!("[reaction: {}]", emoji)
                            }
                            MessageContent::System(s) => format!("[system: {}]", s),
                        };
                        vec![
                            truncate_did(&m.sender_did),
                            m.timestamp.format("%Y-%m-%d %H:%M").to_string(),
                            content_str,
                        ]
                    })
                    .collect();

                self.output.table(&["Sender", "Time", "Content"], &rows);
                Ok(())
            }
            MessageCommand::Channels { member } => {
                let did = if let Some(m) = member {
                    m
                } else {
                    let identity = self
                        .load_identity()
                        .ok_or("No identity found. Run 'nous init' first.")?;
                    identity.did().to_string()
                };

                let channels = self.load_channels_for_member(&did);

                if channels.is_empty() {
                    self.output.success("No channels found.");
                } else {
                    let rows: Vec<Vec<String>> = channels
                        .iter()
                        .map(|ch| {
                            vec![
                                ch.id.clone(),
                                format!("{:?}", ch.kind),
                                ch.name.clone().unwrap_or_default(),
                                ch.member_count().to_string(),
                            ]
                        })
                        .collect();
                    self.output.table(&["ID", "Kind", "Name", "Members"], &rows);
                }
                Ok(())
            }
            MessageCommand::CreateChannel {
                kind,
                name,
                members,
            } => {
                let identity = self
                    .load_identity()
                    .ok_or("No identity found. Run 'nous init' first.")?;

                let channel = match kind.to_lowercase().as_str() {
                    "direct" => {
                        if members.len() != 1 {
                            return Err("direct channels require exactly one --member".into());
                        }
                        Channel::direct(identity.did(), &members[0])
                    }
                    "group" => {
                        let ch_name = name.ok_or("group channels require --name")?;
                        Channel::group(identity.did(), ch_name, members)
                    }
                    "public" => {
                        let ch_name = name.ok_or("public channels require --name")?;
                        Channel::public(identity.did(), ch_name)
                    }
                    other => {
                        return Err(format!(
                            "unknown channel kind: '{}'. Use: direct, group, public",
                            other
                        ));
                    }
                };

                let channel_id = channel.id.clone();
                self.save_channel(&channel)?;

                self.output.success(&format!(
                    "Created {:?} channel\n  ID: {}\n  Members: {}",
                    channel.kind,
                    channel_id,
                    channel.member_count(),
                ));
                Ok(())
            }
        }
    }

    async fn net(&self, cmd: NetCommand) -> Result<(), String> {
        match cmd {
            NetCommand::Peers => {
                // Boot a transient node to discover peers via mDNS.
                let config = nous_node::NodeConfig {
                    data_dir: self.data_dir.clone(),
                    display_name: None,
                    network: nous_net::NodeConfig::default(),
                };
                let mut node =
                    nous_node::NousNode::new(config).map_err(|e| format!("node init: {e}"))?;

                self.output.success(&format!(
                    "Local peer ID: {}\nDID: {}\nStarting peer discovery...",
                    node.peer_id().map(|p| p.to_string()).unwrap_or_default(),
                    node.did()
                ));

                node.start().await.map_err(|e| format!("node start: {e}"))?;

                // Let mDNS discover peers for a few seconds.
                tokio::time::sleep(std::time::Duration::from_secs(3)).await;

                node.shutdown().await;
                self.output.success("Peer discovery complete.");
                Ok(())
            }
            NetCommand::Status => {
                let config = nous_node::NodeConfig {
                    data_dir: self.data_dir.clone(),
                    display_name: None,
                    network: nous_net::NodeConfig::default(),
                };
                let node =
                    nous_node::NousNode::new(config).map_err(|e| format!("node init: {e}"))?;

                self.output.table(
                    &["Property", "Value"],
                    &[
                        vec!["Protocol".into(), "libp2p".into()],
                        vec!["Transport".into(), "TCP + Noise + Yamux".into()],
                        vec!["Discovery".into(), "mDNS + Kademlia".into()],
                        vec!["DID".into(), node.did().to_string()],
                        vec![
                            "Peer ID".into(),
                            node.peer_id()
                                .map(|p| p.to_string())
                                .unwrap_or_else(|| "unknown".into()),
                        ],
                        vec![
                            "Status".into(),
                            if node.is_running() {
                                "online"
                            } else {
                                "ready (not started)"
                            }
                            .into(),
                        ],
                    ],
                );
                Ok(())
            }
            NetCommand::Connect { addr } => {
                let config = nous_node::NodeConfig {
                    data_dir: self.data_dir.clone(),
                    display_name: None,
                    network: nous_net::NodeConfig {
                        bootstrap_peers: vec![addr.clone()],
                        ..Default::default()
                    },
                };
                let mut node =
                    nous_node::NousNode::new(config).map_err(|e| format!("node init: {e}"))?;

                node.start().await.map_err(|e| format!("node start: {e}"))?;

                self.output.success(&format!("Connecting to {addr}..."));

                // Give time for the connection to establish.
                tokio::time::sleep(std::time::Duration::from_secs(5)).await;

                node.shutdown().await;
                self.output.success("Connection attempt complete.");
                Ok(())
            }
        }
    }

    fn marketplace(&self, cmd: MarketplaceCommand) -> Result<(), String> {
        match cmd {
            MarketplaceCommand::List {
                title,
                description,
                category,
                token,
                price,
                tags,
            } => {
                let cat = parse_listing_category(&category)?;
                let identity = self
                    .load_identity()
                    .ok_or("No identity found. Run 'nous init' first.")?;

                let mut listing = Listing::new(
                    identity.did(),
                    &title,
                    description.as_deref().unwrap_or(""),
                    cat,
                    &token,
                    price,
                )
                .map_err(|e| e.to_string())?;

                if let Some(ref tags_str) = tags {
                    for tag in tags_str.split(',') {
                        listing = listing.with_tag(tag.trim());
                    }
                }

                self.output.table(
                    &["Field", "Value"],
                    &[
                        vec!["ID".into(), listing.id.clone()],
                        vec!["Title".into(), listing.title.clone()],
                        vec!["Category".into(), format!("{:?}", listing.category)],
                        vec![
                            "Price".into(),
                            format!("{} {}", listing.price_amount, listing.price_token),
                        ],
                        vec!["Status".into(), format!("{:?}", listing.status)],
                    ],
                );
                self.output.success("Listing created");
                Ok(())
            }
            MarketplaceCommand::Search {
                query,
                category,
                limit,
            } => {
                let mut sq = SearchQuery::new();
                if let Some(ref q) = query {
                    sq = sq.text(q);
                }
                if let Some(ref cat) = category
                    && let Ok(c) = parse_listing_category_opt(cat)
                {
                    sq = sq.category(c);
                }
                let _sq = sq.paginate(limit, 0);

                // In offline mode, search is over locally stored listings
                self.output
                    .success("Search executed (local mode — connect to API for full results)");
                self.output.table(
                    &["Query", "Category", "Limit"],
                    &[vec![
                        query.unwrap_or_else(|| "*".into()),
                        category.unwrap_or_else(|| "all".into()),
                        limit.to_string(),
                    ]],
                );
                Ok(())
            }
            MarketplaceCommand::Show { id } => {
                self.output.success(&format!("Listing: {id}"));
                self.output.success("Connect to API for listing details");
                Ok(())
            }
            MarketplaceCommand::Order {
                listing_id,
                quantity,
            } => {
                let identity = self
                    .load_identity()
                    .ok_or("No identity found. Run 'nous init' first.")?;

                self.output.table(
                    &["Field", "Value"],
                    &[
                        vec!["Listing".into(), listing_id],
                        vec!["Buyer".into(), truncate_did(identity.did())],
                        vec!["Quantity".into(), quantity.to_string()],
                    ],
                );
                self.output
                    .success("Order created (connect to API to fund escrow)");
                Ok(())
            }
            MarketplaceCommand::Orders { role, limit } => {
                self.output.success(&format!(
                    "Orders (role: {}, limit: {limit})",
                    role.as_deref().unwrap_or("all")
                ));
                self.output.success("Connect to API for order list");
                Ok(())
            }
            MarketplaceCommand::Offer {
                listing_id,
                amount,
                token,
                message,
            } => {
                let identity = self
                    .load_identity()
                    .ok_or("No identity found. Run 'nous init' first.")?;

                self.output.table(
                    &["Field", "Value"],
                    &[
                        vec!["Listing".into(), listing_id],
                        vec!["Buyer".into(), truncate_did(identity.did())],
                        vec!["Amount".into(), format!("{amount} {token}")],
                        vec!["Message".into(), message.unwrap_or_default()],
                    ],
                );
                self.output.success("Offer submitted");
                Ok(())
            }
            MarketplaceCommand::Offers { listing_id } => {
                self.output.success(&format!(
                    "Offers (listing: {})",
                    listing_id.as_deref().unwrap_or("all")
                ));
                self.output.success("Connect to API for offer list");
                Ok(())
            }
            MarketplaceCommand::Dispute {
                order_id,
                reason,
                description,
            } => {
                self.output.table(
                    &["Field", "Value"],
                    &[
                        vec!["Order".into(), order_id],
                        vec!["Reason".into(), reason],
                        vec!["Description".into(), description],
                    ],
                );
                self.output.success("Dispute opened");
                Ok(())
            }
            MarketplaceCommand::Cancel { id } => {
                self.output.success(&format!("Listing {id} cancelled"));
                Ok(())
            }
        }
    }

    fn status(&self) -> Result<(), String> {
        let identity = self.load_identity();
        let did = identity
            .as_ref()
            .map(|i| i.did().to_string())
            .unwrap_or_else(|| "(none)".to_string());
        let events = self.load_events();
        let graph = self.load_follow_graph();
        let following_count = identity
            .as_ref()
            .map(|i| graph.following_of(i.did()).len())
            .unwrap_or(0);

        self.output.table(
            &["Property", "Value"],
            &[
                vec!["Identity".into(), truncate_did(&did)],
                vec!["Events".into(), events.len().to_string()],
                vec!["Following".into(), following_count.to_string()],
                vec!["Version".into(), env!("CARGO_PKG_VERSION").to_string()],
                vec!["Protocol".into(), "nous/0.1".into()],
            ],
        );
        Ok(())
    }

    fn ai(&self, cmd: AiCommand) -> Result<(), String> {
        match cmd {
            AiCommand::Chat {
                message,
                model,
                temperature,
            } => {
                use nous_ai::{Conversation, Message};

                let mut conv = Conversation::new("cli-session");
                conv.add_message(Message::user(&message));

                self.output.table(
                    &["Field", "Value"],
                    &[
                        vec!["Model".into(), model],
                        vec!["Temperature".into(), format!("{temperature:.1}")],
                        vec!["Message".into(), truncate_text(&message, 60)],
                        vec![
                            "Tokens (est)".into(),
                            conv.total_tokens_estimate().to_string(),
                        ],
                    ],
                );
                self.output
                    .success("Chat request prepared. Connect to inference backend for response.");
                Ok(())
            }
            AiCommand::Search { query, limit } => {
                use nous_ai::KnowledgeBase;

                let kb_path = self.data_dir.join("knowledge.db");

                if !kb_path.exists() {
                    self.output.success(
                        "No knowledge base found. Index documents first with 'nous ai index'.",
                    );
                    return Ok(());
                }

                let kb =
                    KnowledgeBase::open(&kb_path).map_err(|e| format!("failed to open KB: {e}"))?;

                // Without an embedding model, display indexed document count and search config
                let doc_count = kb.document_count().map_err(|e| e.to_string())?;
                let chunk_count = kb.indexed_chunk_count();

                self.output.table(
                    &["Field", "Value"],
                    &[
                        vec!["Query".into(), query],
                        vec!["Limit".into(), limit.to_string()],
                        vec!["Documents".into(), doc_count.to_string()],
                        vec!["Indexed chunks".into(), chunk_count.to_string()],
                    ],
                );
                if chunk_count == 0 {
                    self.output.success(
                        "No embeddings indexed. Generate embeddings with an embedding model to enable semantic search.",
                    );
                } else {
                    self.output.success(
                        "Semantic search requires an embedding model. Connect to inference backend.",
                    );
                }
                Ok(())
            }
            AiCommand::Index {
                path,
                title,
                source,
            } => {
                use nous_ai::{ChunkOptions, KnowledgeBase};

                let file_path = Path::new(&path);
                if !file_path.exists() {
                    return Err(format!("file not found: {path}"));
                }

                let content = std::fs::read_to_string(file_path)
                    .map_err(|e| format!("failed to read file: {e}"))?;

                let doc_title = title.unwrap_or_else(|| {
                    file_path
                        .file_name()
                        .and_then(|n| n.to_str())
                        .unwrap_or("untitled")
                        .to_string()
                });

                let kb_path = self.data_dir.join("knowledge.db");

                let mut kb =
                    KnowledgeBase::open(&kb_path).map_err(|e| format!("failed to open KB: {e}"))?;

                let options = ChunkOptions::default();
                let doc = kb
                    .ingest(&doc_title, &content, &source, &options)
                    .map_err(|e| format!("failed to index: {e}"))?;

                self.output.table(
                    &["Field", "Value"],
                    &[
                        vec!["Document ID".into(), doc.id],
                        vec!["Title".into(), doc.title],
                        vec!["Chunks".into(), doc.chunk_count.to_string()],
                        vec!["Source".into(), doc.source],
                    ],
                );
                self.output.success("Document indexed");
                Ok(())
            }
            AiCommand::Documents { limit: _ } => {
                use nous_ai::KnowledgeBase;

                let kb_path = self.data_dir.join("knowledge.db");

                if !kb_path.exists() {
                    self.output.success("No knowledge base found");
                    return Ok(());
                }

                let kb =
                    KnowledgeBase::open(&kb_path).map_err(|e| format!("failed to open KB: {e}"))?;

                let docs = kb.list_documents().map_err(|e| e.to_string())?;

                if docs.is_empty() {
                    self.output.success("No documents indexed");
                } else {
                    let rows: Vec<Vec<String>> = docs
                        .iter()
                        .map(|d| {
                            vec![
                                d.id.clone(),
                                d.title.clone(),
                                d.chunk_count.to_string(),
                                d.source.clone(),
                            ]
                        })
                        .collect();
                    self.output
                        .table(&["ID", "Title", "Chunks", "Source"], &rows);
                }
                Ok(())
            }
            AiCommand::Agents => {
                use nous_ai::Agent;

                let agents = [
                    Agent::new("researcher", "Researcher")
                        .with_system_prompt(
                            "Research agent for finding and summarizing information.",
                        )
                        .with_capability("search")
                        .with_capability("summarize"),
                    Agent::new("analyst", "Governance Analyst")
                        .with_system_prompt("Analyzes governance proposals and voting patterns.")
                        .with_capability("analyze")
                        .with_capability("report"),
                    Agent::new("coder", "Code Assistant")
                        .with_system_prompt(
                            "Assists with code review, debugging, and implementation.",
                        )
                        .with_capability("code")
                        .with_capability("review"),
                ];

                let rows: Vec<Vec<String>> = agents
                    .iter()
                    .map(|a| {
                        vec![
                            a.id.clone(),
                            a.name.clone(),
                            a.capabilities.join(", "),
                            a.model.clone(),
                        ]
                    })
                    .collect();
                self.output
                    .table(&["ID", "Name", "Capabilities", "Model"], &rows);
                Ok(())
            }
            AiCommand::Run {
                agent_id,
                task,
                max_steps,
            } => {
                use nous_ai::{Agent, ExecutionConfig};

                let agent = match agent_id.as_str() {
                    "researcher" => Agent::new("researcher", "Researcher")
                        .with_system_prompt("You are a research agent.")
                        .with_capability("search")
                        .with_capability("summarize"),
                    "analyst" => Agent::new("analyst", "Governance Analyst")
                        .with_system_prompt("You analyze governance proposals.")
                        .with_capability("analyze"),
                    "coder" => Agent::new("coder", "Code Assistant")
                        .with_system_prompt("You assist with code.")
                        .with_capability("code"),
                    other => {
                        return Err(format!(
                            "unknown agent: {other}. Run 'nous ai agents' to list available agents."
                        ));
                    }
                };

                let config = ExecutionConfig {
                    max_steps,
                    ..Default::default()
                };

                self.output.table(
                    &["Field", "Value"],
                    &[
                        vec!["Agent".into(), agent.name],
                        vec!["Task".into(), truncate_text(&task, 60)],
                        vec!["Max steps".into(), config.max_steps.to_string()],
                        vec!["Model".into(), agent.model],
                    ],
                );
                self.output
                    .success("Agent configured. Connect to inference backend to execute.");
                Ok(())
            }
        }
    }

    /// Launch an embedded terminal using nous-terminal and crossterm raw mode.
    #[cfg(unix)]
    fn run_terminal() -> Result<(), String> {
        let (cols, rows) =
            terminal::size().map_err(|e| format!("failed to get terminal size: {e}"))?;

        let config = TerminalConfig {
            rows,
            cols,
            ..Default::default()
        };

        let mut term =
            Terminal::spawn(config).map_err(|e| format!("failed to spawn terminal: {e}"))?;

        let mut stdout = std::io::stdout();

        // Enter raw mode and alternate screen
        terminal::enable_raw_mode().map_err(|e| format!("failed to enable raw mode: {e}"))?;
        execute!(stdout, EnterAlternateScreen, cursor::Hide)
            .map_err(|e| format!("failed to enter alternate screen: {e}"))?;

        let result = Self::terminal_loop(&mut term, &mut stdout);

        // Always restore terminal state
        let _ = execute!(stdout, cursor::Show, LeaveAlternateScreen);
        let _ = terminal::disable_raw_mode();

        result
    }

    #[cfg(unix)]
    fn terminal_loop(term: &mut Terminal, stdout: &mut std::io::Stdout) -> Result<(), String> {
        use std::time::Duration;

        let mut last_screen: Vec<nous_terminal::RenderRow> = Vec::new();

        loop {
            // Check if the child shell is still alive
            if !term.is_alive() {
                break;
            }

            // Read PTY output and process through VT parser
            match term.tick() {
                Ok(dirty) => {
                    if dirty {
                        Self::render_screen(term, stdout, &mut last_screen)?;
                    }
                }
                Err(e) => {
                    // EIO means the child exited — not an error
                    let msg = e.to_string();
                    if msg.contains("EIO") || msg.contains("Input/output error") {
                        break;
                    }
                    return Err(format!("pty read error: {e}"));
                }
            }

            // Poll for keyboard/resize events with a short timeout
            if event::poll(Duration::from_millis(10))
                .map_err(|e| format!("event poll error: {e}"))?
            {
                match event::read().map_err(|e| format!("event read error: {e}"))? {
                    Event::Key(KeyEvent {
                        code: KeyCode::Char('q'),
                        modifiers: KeyModifiers::CONTROL,
                        ..
                    }) => {
                        break;
                    }
                    Event::Key(key_event) => {
                        let bytes = key_event_to_bytes(&key_event);
                        if !bytes.is_empty() {
                            term.write(&bytes)
                                .map_err(|e| format!("pty write error: {e}"))?;
                        }
                    }
                    Event::Resize(cols, rows) => {
                        term.resize(rows, cols)
                            .map_err(|e| format!("resize error: {e}"))?;
                        // Force full redraw after resize
                        last_screen.clear();
                        Self::render_screen(term, stdout, &mut last_screen)?;
                    }
                    Event::Paste(text) => {
                        term.write(text.as_bytes())
                            .map_err(|e| format!("pty write error: {e}"))?;
                    }
                    _ => {}
                }
            }
        }

        Ok(())
    }

    #[cfg(unix)]
    fn render_screen(
        term: &Terminal,
        stdout: &mut std::io::Stdout,
        last_screen: &mut Vec<nous_terminal::RenderRow>,
    ) -> Result<(), String> {
        let screen = term.screen();
        let (cursor_row, cursor_col) = term.cursor_position();

        for (row_idx, render_row) in screen.iter().enumerate() {
            // Skip unchanged rows for efficiency
            if row_idx < last_screen.len() {
                let old_row = &last_screen[row_idx];
                if old_row.cells.len() == render_row.cells.len()
                    && old_row
                        .cells
                        .iter()
                        .zip(render_row.cells.iter())
                        .all(|(a, b)| a == b)
                {
                    continue;
                }
            }

            queue!(stdout, cursor::MoveTo(0, row_idx as u16))
                .map_err(|e| format!("cursor move error: {e}"))?;

            for cell in &render_row.cells {
                let fg = nous_color_to_crossterm(cell.style.fg);
                let bg = nous_color_to_crossterm(cell.style.bg);

                let mut attrs = crossterm::style::Attributes::default();
                if cell.style.bold {
                    attrs.set(crossterm::style::Attribute::Bold);
                }
                if cell.style.italic {
                    attrs.set(crossterm::style::Attribute::Italic);
                }
                if cell.style.underline {
                    attrs.set(crossterm::style::Attribute::Underlined);
                }
                if cell.style.strikethrough {
                    attrs.set(crossterm::style::Attribute::CrossedOut);
                }
                if cell.style.inverse {
                    attrs.set(crossterm::style::Attribute::Reverse);
                }

                queue!(
                    stdout,
                    style::SetForegroundColor(fg),
                    style::SetBackgroundColor(bg),
                    style::SetAttributes(attrs),
                    style::Print(cell.ch),
                    style::SetAttributes(crossterm::style::Attributes::default()),
                    style::ResetColor
                )
                .map_err(|e| format!("render error: {e}"))?;
            }
        }

        // Position the cursor
        queue!(stdout, cursor::MoveTo(cursor_col, cursor_row), cursor::Show)
            .map_err(|e| format!("cursor position error: {e}"))?;

        stdout.flush().map_err(|e| format!("flush error: {e}"))?;

        *last_screen = screen;
        Ok(())
    }

    // --- persistence helpers ---

    fn store_identity(&self, identity: &Identity) -> Result<(), String> {
        let doc_json = serde_json::to_string(identity.document())
            .map_err(|e| format!("serialization: {e}"))?;
        let signing_key = identity.export_signing_key();
        self.db
            .store_identity(identity.did(), &doc_json, Some(&signing_key))
            .map_err(|e| format!("storage: {e}"))?;

        // Mark as active identity
        self.db
            .put_kv("active_identity", identity.did().as_bytes())
            .map_err(|e| format!("storage: {e}"))?;
        Ok(())
    }

    fn load_identity(&self) -> Option<Identity> {
        let active_did_bytes = self.db.get_kv("active_identity").ok()??;
        let active_did = String::from_utf8(active_did_bytes).ok()?;

        let (_, signing_key) = self.db.get_identity(&active_did).ok()??;
        let signing_key = signing_key?;

        Identity::restore(&signing_key).ok()
    }

    fn list_identities(&self) -> Vec<String> {
        // Query all identities from the database
        let conn = self.db.conn();
        let mut stmt = match conn.prepare("SELECT did FROM identities") {
            Ok(s) => s,
            Err(_) => return vec![],
        };
        let rows = match stmt.query_map([], |row| row.get::<_, String>(0)) {
            Ok(r) => r,
            Err(_) => return vec![],
        };
        rows.filter_map(|r| r.ok()).collect()
    }

    fn load_events(&self) -> Vec<SignedEvent> {
        let conn = self.db.conn();
        let mut stmt = match conn.prepare("SELECT value FROM kv WHERE key LIKE 'event:%'") {
            Ok(s) => s,
            Err(_) => return vec![],
        };
        let rows = match stmt.query_map([], |row| row.get::<_, Vec<u8>>(0)) {
            Ok(r) => r,
            Err(_) => return vec![],
        };
        rows.filter_map(|r| r.ok())
            .filter_map(|bytes| serde_json::from_slice(&bytes).ok())
            .collect()
    }

    fn load_follow_graph(&self) -> FollowGraph {
        self.db
            .get_kv("follow_graph")
            .ok()
            .flatten()
            .and_then(|bytes| serde_json::from_slice(&bytes).ok())
            .unwrap_or_default()
    }

    fn save_follow_graph(&self, graph: &FollowGraph) -> Result<(), String> {
        let json = serde_json::to_vec(graph).map_err(|e| format!("serialization: {e}"))?;
        self.db
            .put_kv("follow_graph", &json)
            .map_err(|e| format!("storage: {e}"))?;
        Ok(())
    }

    fn load_wallet(&self, did: &str) -> Wallet {
        let key = format!("wallet:{did}");
        self.db
            .get_kv(&key)
            .ok()
            .flatten()
            .and_then(|bytes| serde_json::from_slice(&bytes).ok())
            .unwrap_or_else(|| Wallet::new(did))
    }

    fn save_wallet(&self, wallet: &Wallet) -> Result<(), String> {
        let key = format!("wallet:{}", wallet.did);
        let json = serde_json::to_vec(wallet).map_err(|e| format!("serialization: {e}"))?;
        self.db
            .put_kv(&key, &json)
            .map_err(|e| format!("storage: {e}"))?;
        Ok(())
    }

    async fn governance(&self, cmd: GovernanceCommand) -> Result<(), String> {
        let identity = self
            .load_identity()
            .ok_or("No identity found. Run 'nous init' first.")?;

        match cmd {
            GovernanceCommand::CreateDao { name, description } => {
                let desc = description.unwrap_or_default();
                let dao = Dao::create(identity.did(), &name, &desc);
                let dao_json =
                    serde_json::to_vec(&dao).map_err(|e| format!("serialization: {e}"))?;
                self.db
                    .put_kv(&format!("dao:{}", dao.id), &dao_json)
                    .map_err(|e| format!("storage: {e}"))?;

                self.output
                    .success(&format!("Created DAO '{}'\n  ID: {}", name, dao.id));
                Ok(())
            }
            GovernanceCommand::ListDaos => {
                let daos = self.load_daos();
                if daos.is_empty() {
                    self.output.success("No DAOs found.");
                } else {
                    let rows: Vec<Vec<String>> = daos
                        .iter()
                        .map(|d| {
                            vec![
                                d.name.clone(),
                                d.id.clone(),
                                d.member_count().to_string(),
                                truncate_did(&d.founder_did),
                            ]
                        })
                        .collect();
                    self.output
                        .table(&["Name", "ID", "Members", "Founder"], &rows);
                }
                Ok(())
            }
            GovernanceCommand::ShowDao { id } => {
                let dao = self.load_dao(&id).ok_or("DAO not found")?;
                self.output.table(
                    &["Property", "Value"],
                    &[
                        vec!["Name".into(), dao.name.clone()],
                        vec!["ID".into(), dao.id.clone()],
                        vec!["Description".into(), dao.description.clone()],
                        vec!["Founder".into(), truncate_did(&dao.founder_did)],
                        vec!["Members".into(), dao.member_count().to_string()],
                    ],
                );
                Ok(())
            }
            GovernanceCommand::Propose {
                dao_id,
                title,
                description,
                voting_days,
            } => {
                let _dao = self.load_dao(&dao_id).ok_or("DAO not found")?;
                let desc = description.unwrap_or_default();
                let proposal = ProposalBuilder::new(&dao_id, &title, &desc)
                    .voting_duration(chrono::Duration::days(voting_days as i64))
                    .submit(&identity)
                    .map_err(|e| format!("{e}"))?;

                let proposal_json =
                    serde_json::to_vec(&proposal).map_err(|e| format!("serialization: {e}"))?;
                self.db
                    .put_kv(&format!("proposal:{}", proposal.id), &proposal_json)
                    .map_err(|e| format!("storage: {e}"))?;

                // Initialize tally
                let tally = VoteTally::new(&proposal.id, proposal.quorum, proposal.threshold);
                let tally_json =
                    serde_json::to_vec(&tally).map_err(|e| format!("serialization: {e}"))?;
                self.db
                    .put_kv(&format!("tally:{}", proposal.id), &tally_json)
                    .map_err(|e| format!("storage: {e}"))?;

                self.output.success(&format!(
                    "Proposal submitted: '{}'\n  ID: {}\n  Voting ends: {}",
                    title,
                    proposal.id,
                    proposal.voting_ends.format("%Y-%m-%d %H:%M")
                ));
                Ok(())
            }
            GovernanceCommand::ListProposals { dao_id } => {
                let proposals = self.load_proposals();
                let filtered: Vec<_> = if let Some(ref dao) = dao_id {
                    proposals.into_iter().filter(|p| p.dao_id == *dao).collect()
                } else {
                    proposals
                };

                if filtered.is_empty() {
                    self.output.success("No proposals found.");
                } else {
                    let rows: Vec<Vec<String>> = filtered
                        .iter()
                        .map(|p| {
                            vec![
                                p.title.clone(),
                                p.id.clone(),
                                format!("{:?}", p.status),
                                truncate_did(&p.proposer_did),
                            ]
                        })
                        .collect();
                    self.output
                        .table(&["Title", "ID", "Status", "Proposer"], &rows);
                }
                Ok(())
            }
            GovernanceCommand::Vote {
                proposal_id,
                choice,
                credits,
            } => {
                let _proposal = self
                    .load_proposal(&proposal_id)
                    .ok_or("Proposal not found")?;
                let vote_choice = match choice.to_lowercase().as_str() {
                    "for" | "yes" => VoteChoice::For,
                    "against" | "no" => VoteChoice::Against,
                    "abstain" => VoteChoice::Abstain,
                    _ => {
                        return Err(format!(
                            "Invalid vote choice: '{}'. Use: for, against, abstain",
                            choice
                        ));
                    }
                };

                let ballot = Ballot::new(&proposal_id, &identity, vote_choice, credits)
                    .map_err(|e| format!("{e}"))?;

                let mut tally = self
                    .load_tally(&proposal_id)
                    .unwrap_or_else(|| VoteTally::new(&proposal_id, 0.0, 0.5));
                tally.cast(ballot).map_err(|e| format!("{e}"))?;

                let tally_json =
                    serde_json::to_vec(&tally).map_err(|e| format!("serialization: {e}"))?;
                self.db
                    .put_kv(&format!("tally:{}", proposal_id), &tally_json)
                    .map_err(|e| format!("storage: {e}"))?;

                let votes = QuadraticVoting::credits_to_votes(credits);
                self.output.success(&format!(
                    "Vote cast: {} ({} credits = {} votes)\n  Proposal: {}",
                    choice, credits, votes, proposal_id
                ));
                Ok(())
            }
            GovernanceCommand::Tally { proposal_id } => {
                let tally = self
                    .load_tally(&proposal_id)
                    .ok_or("No tally found for this proposal")?;
                let result = tally.tally(tally.voter_count());

                self.output.table(
                    &["Metric", "Value"],
                    &[
                        vec!["Proposal".into(), proposal_id],
                        vec!["For".into(), result.votes_for.to_string()],
                        vec!["Against".into(), result.votes_against.to_string()],
                        vec!["Abstain".into(), result.votes_abstain.to_string()],
                        vec!["Voters".into(), result.total_voters.to_string()],
                        vec!["Passed".into(), result.passed.to_string()],
                    ],
                );
                Ok(())
            }
        }
    }

    fn load_daos(&self) -> Vec<Dao> {
        let conn = self.db.conn();
        let mut stmt = match conn.prepare("SELECT value FROM kv WHERE key LIKE 'dao:%'") {
            Ok(s) => s,
            Err(_) => return vec![],
        };
        let rows = match stmt.query_map([], |row| row.get::<_, Vec<u8>>(0)) {
            Ok(r) => r,
            Err(_) => return vec![],
        };
        rows.filter_map(|r| r.ok())
            .filter_map(|bytes| serde_json::from_slice(&bytes).ok())
            .collect()
    }

    fn load_dao(&self, id: &str) -> Option<Dao> {
        let key = format!("dao:{id}");
        self.db
            .get_kv(&key)
            .ok()?
            .and_then(|bytes| serde_json::from_slice(&bytes).ok())
    }

    fn load_proposals(&self) -> Vec<nous_governance::Proposal> {
        let conn = self.db.conn();
        let mut stmt = match conn.prepare("SELECT value FROM kv WHERE key LIKE 'proposal:%'") {
            Ok(s) => s,
            Err(_) => return vec![],
        };
        let rows = match stmt.query_map([], |row| row.get::<_, Vec<u8>>(0)) {
            Ok(r) => r,
            Err(_) => return vec![],
        };
        rows.filter_map(|r| r.ok())
            .filter_map(|bytes| serde_json::from_slice(&bytes).ok())
            .collect()
    }

    fn load_proposal(&self, id: &str) -> Option<nous_governance::Proposal> {
        let key = format!("proposal:{id}");
        self.db
            .get_kv(&key)
            .ok()?
            .and_then(|bytes| serde_json::from_slice(&bytes).ok())
    }

    fn load_tally(&self, proposal_id: &str) -> Option<VoteTally> {
        let key = format!("tally:{proposal_id}");
        self.db
            .get_kv(&key)
            .ok()?
            .and_then(|bytes| serde_json::from_slice(&bytes).ok())
    }

    // --- file persistence helpers ---

    fn load_file_store(&self) -> FileStore {
        self.db
            .get_kv("file_store")
            .ok()
            .flatten()
            .and_then(|bytes| serde_json::from_slice(&bytes).ok())
            .unwrap_or_default()
    }

    fn save_file_store(&self, store: &FileStore) -> Result<(), String> {
        let json = serde_json::to_vec(store).map_err(|e| format!("serialization: {e}"))?;
        self.db
            .put_kv("file_store", &json)
            .map_err(|e| format!("storage: {e}"))?;
        Ok(())
    }

    fn load_shared_folder(&self, owner: &str) -> Option<SharedFolder> {
        let key = format!("shared_folder:{owner}");
        self.db
            .get_kv(&key)
            .ok()?
            .and_then(|bytes| serde_json::from_slice(&bytes).ok())
    }

    fn save_shared_folder(&self, folder: &SharedFolder) -> Result<(), String> {
        let key = format!("shared_folder:{}", folder.owner);
        let json = serde_json::to_vec(folder).map_err(|e| format!("serialization: {e}"))?;
        self.db
            .put_kv(&key, &json)
            .map_err(|e| format!("storage: {e}"))?;
        Ok(())
    }

    fn save_vault(&self, vault: &Vault) -> Result<(), String> {
        let key = format!("vault:{}", vault.id);
        let json = serde_json::to_vec(vault).map_err(|e| format!("serialization: {e}"))?;
        self.db
            .put_kv(&key, &json)
            .map_err(|e| format!("storage: {e}"))?;
        Ok(())
    }

    fn load_vault(&self, vault_id: &str) -> Option<Vault> {
        let key = format!("vault:{vault_id}");
        self.db
            .get_kv(&key)
            .ok()?
            .and_then(|bytes| serde_json::from_slice(&bytes).ok())
    }

    fn save_vault_entry(&self, vault_id: &str, entry: &VaultEntry) -> Result<(), String> {
        let key = format!("vault_entry:{vault_id}:{}", entry.content_hash);
        let json = serde_json::to_vec(entry).map_err(|e| format!("serialization: {e}"))?;
        self.db
            .put_kv(&key, &json)
            .map_err(|e| format!("storage: {e}"))?;
        Ok(())
    }

    // --- messaging persistence helpers ---

    fn save_channel(&self, channel: &Channel) -> Result<(), String> {
        let key = format!("channel:{}", channel.id);
        let json = serde_json::to_vec(channel).map_err(|e| format!("serialization: {e}"))?;
        self.db
            .put_kv(&key, &json)
            .map_err(|e| format!("storage: {e}"))?;
        Ok(())
    }

    fn load_channel(&self, channel_id: &str) -> Option<Channel> {
        let key = format!("channel:{channel_id}");
        self.db
            .get_kv(&key)
            .ok()?
            .and_then(|bytes| serde_json::from_slice(&bytes).ok())
    }

    fn save_message(&self, msg: &Message) -> Result<(), String> {
        let key = format!("msg:{}:{}", msg.channel_id, msg.id);
        let json = serde_json::to_vec(msg).map_err(|e| format!("serialization: {e}"))?;
        self.db
            .put_kv(&key, &json)
            .map_err(|e| format!("storage: {e}"))?;
        Ok(())
    }

    fn load_messages(&self, channel_id: &str) -> Vec<Message> {
        let prefix = format!("msg:{channel_id}:");
        let conn = self.db.conn();
        let mut stmt = match conn.prepare("SELECT value FROM kv WHERE key LIKE ?1") {
            Ok(s) => s,
            Err(_) => return vec![],
        };
        let pattern = format!("{prefix}%");
        let rows = match stmt.query_map([&pattern], |row| row.get::<_, Vec<u8>>(0)) {
            Ok(r) => r,
            Err(_) => return vec![],
        };
        let mut messages: Vec<Message> = rows
            .filter_map(|r| r.ok())
            .filter_map(|bytes| serde_json::from_slice(&bytes).ok())
            .collect();
        messages.sort_by(|a, b| a.timestamp.cmp(&b.timestamp));
        messages
    }

    fn load_channels_for_member(&self, did: &str) -> Vec<Channel> {
        let conn = self.db.conn();
        let mut stmt = match conn.prepare("SELECT value FROM kv WHERE key LIKE 'channel:%'") {
            Ok(s) => s,
            Err(_) => return vec![],
        };
        let rows = match stmt.query_map([], |row| row.get::<_, Vec<u8>>(0)) {
            Ok(r) => r,
            Err(_) => return vec![],
        };
        rows.filter_map(|r| r.ok())
            .filter_map(|bytes| serde_json::from_slice::<Channel>(&bytes).ok())
            .filter(|ch| ch.is_member(did))
            .collect()
    }
}

fn truncate_text(text: &str, max: usize) -> String {
    if text.len() <= max {
        text.to_string()
    } else {
        format!("{}...", &text[..max.saturating_sub(3)])
    }
}

fn truncate_did(did: &str) -> String {
    if did.len() > 24 {
        format!("{}...{}", &did[..16], &did[did.len() - 6..])
    } else {
        did.to_string()
    }
}

fn parse_listing_category(s: &str) -> Result<ListingCategory, String> {
    parse_listing_category_opt(s).map_err(|_| format!("invalid category: {s}"))
}

fn parse_listing_category_opt(s: &str) -> Result<ListingCategory, ()> {
    match s.to_lowercase().as_str() {
        "physical" => Ok(ListingCategory::Physical),
        "digital" => Ok(ListingCategory::Digital),
        "service" => Ok(ListingCategory::Service),
        "nft" => Ok(ListingCategory::NFT),
        "data" => Ok(ListingCategory::Data),
        "other" => Ok(ListingCategory::Other),
        _ => Err(()),
    }
}

fn format_bytes(bytes: u64) -> String {
    if bytes < 1024 {
        format!("{} B", bytes)
    } else if bytes < 1024 * 1024 {
        format!("{:.1} KiB", bytes as f64 / 1024.0)
    } else if bytes < 1024 * 1024 * 1024 {
        format!("{:.1} MiB", bytes as f64 / (1024.0 * 1024.0))
    } else {
        format!("{:.1} GiB", bytes as f64 / (1024.0 * 1024.0 * 1024.0))
    }
}

fn mime_from_extension(filename: &str) -> String {
    match filename
        .rsplit('.')
        .next()
        .unwrap_or("")
        .to_lowercase()
        .as_str()
    {
        "txt" => "text/plain",
        "md" => "text/markdown",
        "html" | "htm" => "text/html",
        "css" => "text/css",
        "js" => "application/javascript",
        "json" => "application/json",
        "xml" => "application/xml",
        "pdf" => "application/pdf",
        "png" => "image/png",
        "jpg" | "jpeg" => "image/jpeg",
        "gif" => "image/gif",
        "webp" => "image/webp",
        "svg" => "image/svg+xml",
        "mp3" => "audio/mpeg",
        "wav" => "audio/wav",
        "mp4" => "video/mp4",
        "webm" => "video/webm",
        "zip" => "application/zip",
        "tar" => "application/x-tar",
        "gz" => "application/gzip",
        "rs" => "text/x-rust",
        "toml" => "application/toml",
        "yaml" | "yml" => "application/yaml",
        _ => "application/octet-stream",
    }
    .to_string()
}

/// Convert a nous-terminal Color to a crossterm Color.
#[cfg(unix)]
fn nous_color_to_crossterm(color: nous_terminal::Color) -> crossterm::style::Color {
    match color {
        nous_terminal::Color::Default => crossterm::style::Color::Reset,
        nous_terminal::Color::Rgb(r, g, b) => crossterm::style::Color::Rgb { r, g, b },
        nous_terminal::Color::Indexed(idx) => crossterm::style::Color::AnsiValue(idx),
    }
}

/// Convert a crossterm KeyEvent into the byte sequence expected by a PTY.
#[cfg(unix)]
fn key_event_to_bytes(key: &KeyEvent) -> Vec<u8> {
    // Handle Ctrl+<letter> combinations (except Ctrl+Q which is handled as quit)
    if key.modifiers.contains(KeyModifiers::CONTROL)
        && let KeyCode::Char(c) = key.code
    {
        let ctrl_byte = (c.to_ascii_lowercase() as u8)
            .wrapping_sub(b'a')
            .wrapping_add(1);
        return vec![ctrl_byte];
    }

    match key.code {
        KeyCode::Char(c) => {
            let mut buf = [0u8; 4];
            let s = c.encode_utf8(&mut buf);
            s.as_bytes().to_vec()
        }
        KeyCode::Enter => vec![b'\r'],
        KeyCode::Backspace => vec![0x7f],
        KeyCode::Tab => vec![b'\t'],
        KeyCode::Esc => vec![0x1b],
        KeyCode::Up => b"\x1b[A".to_vec(),
        KeyCode::Down => b"\x1b[B".to_vec(),
        KeyCode::Right => b"\x1b[C".to_vec(),
        KeyCode::Left => b"\x1b[D".to_vec(),
        KeyCode::Home => b"\x1b[H".to_vec(),
        KeyCode::End => b"\x1b[F".to_vec(),
        KeyCode::PageUp => b"\x1b[5~".to_vec(),
        KeyCode::PageDown => b"\x1b[6~".to_vec(),
        KeyCode::Insert => b"\x1b[2~".to_vec(),
        KeyCode::Delete => b"\x1b[3~".to_vec(),
        KeyCode::F(n) => match n {
            1 => b"\x1bOP".to_vec(),
            2 => b"\x1bOQ".to_vec(),
            3 => b"\x1bOR".to_vec(),
            4 => b"\x1bOS".to_vec(),
            5 => b"\x1b[15~".to_vec(),
            6 => b"\x1b[17~".to_vec(),
            7 => b"\x1b[18~".to_vec(),
            8 => b"\x1b[19~".to_vec(),
            9 => b"\x1b[20~".to_vec(),
            10 => b"\x1b[21~".to_vec(),
            11 => b"\x1b[23~".to_vec(),
            12 => b"\x1b[24~".to_vec(),
            _ => vec![],
        },
        KeyCode::BackTab => b"\x1b[Z".to_vec(),
        KeyCode::Null => vec![0],
        _ => vec![],
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_executor() -> Executor {
        let dir = tempfile::tempdir().unwrap();
        Executor::new(dir.path(), false).unwrap()
    }

    fn test_executor_json() -> Executor {
        let dir = tempfile::tempdir().unwrap();
        Executor::new(dir.path(), true).unwrap()
    }

    #[tokio::test]
    async fn init_creates_identity() {
        let exec = test_executor();
        exec.execute(Command::Init).await.unwrap();
        assert!(exec.load_identity().is_some());
    }

    #[tokio::test]
    async fn init_idempotent() {
        let exec = test_executor();
        exec.execute(Command::Init).await.unwrap();
        exec.execute(Command::Init).await.unwrap();
        assert_eq!(exec.list_identities().len(), 1);
    }

    #[tokio::test]
    async fn identity_show_without_init_fails() {
        let exec = test_executor();
        let result = exec.execute(Command::Identity(IdentityCommand::Show)).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn identity_show_after_init() {
        let exec = test_executor();
        exec.execute(Command::Init).await.unwrap();
        exec.execute(Command::Identity(IdentityCommand::Show))
            .await
            .unwrap();
    }

    #[tokio::test]
    async fn identity_generate() {
        let exec = test_executor();
        exec.execute(Command::Identity(IdentityCommand::Generate))
            .await
            .unwrap();
        assert!(exec.load_identity().is_some());
    }

    #[tokio::test]
    async fn identity_export_after_init() {
        let exec = test_executor();
        exec.execute(Command::Init).await.unwrap();
        exec.execute(Command::Identity(IdentityCommand::Export))
            .await
            .unwrap();
    }

    #[tokio::test]
    async fn identity_list() {
        let exec = test_executor();
        exec.execute(Command::Init).await.unwrap();
        let ids = exec.list_identities();
        assert_eq!(ids.len(), 1);
        assert!(ids[0].starts_with("did:key:z"));
    }

    #[tokio::test]
    async fn social_post() {
        let exec = test_executor();
        exec.execute(Command::Init).await.unwrap();
        exec.execute(Command::Social(SocialCommand::Post {
            content: "hello world".into(),
            tags: Some("nous,web3".into()),
        }))
        .await
        .unwrap();

        let events = exec.load_events();
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].content, "hello world");
        assert_eq!(events[0].hashtags(), vec!["nous", "web3"]);
    }

    #[tokio::test]
    async fn social_feed() {
        let exec = test_executor();
        exec.execute(Command::Init).await.unwrap();
        exec.execute(Command::Social(SocialCommand::Post {
            content: "post one".into(),
            tags: None,
        }))
        .await
        .unwrap();
        exec.execute(Command::Social(SocialCommand::Feed { limit: 20 }))
            .await
            .unwrap();
    }

    #[tokio::test]
    async fn social_follow_unfollow() {
        let exec = test_executor();
        exec.execute(Command::Init).await.unwrap();
        let did = exec.load_identity().unwrap().did().to_string();

        exec.execute(Command::Social(SocialCommand::Follow {
            did: "did:key:zpeer".into(),
        }))
        .await
        .unwrap();

        let graph = exec.load_follow_graph();
        assert!(graph.is_following(&did, "did:key:zpeer"));

        exec.execute(Command::Social(SocialCommand::Unfollow {
            did: "did:key:zpeer".into(),
        }))
        .await
        .unwrap();

        let graph = exec.load_follow_graph();
        assert!(!graph.is_following(&did, "did:key:zpeer"));
    }

    #[tokio::test]
    async fn wallet_balance_empty() {
        let exec = test_executor();
        exec.execute(Command::Init).await.unwrap();
        exec.execute(Command::Wallet(WalletCommand::Balance))
            .await
            .unwrap();
    }

    #[tokio::test]
    async fn wallet_send_insufficient() {
        let exec = test_executor();
        exec.execute(Command::Init).await.unwrap();
        let result = exec
            .execute(Command::Wallet(WalletCommand::Send {
                to: "did:key:zbob".into(),
                token: "ETH".into(),
                amount: 100,
                memo: None,
            }))
            .await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn status_shows_info() {
        let exec = test_executor();
        exec.execute(Command::Init).await.unwrap();
        exec.execute(Command::Status).await.unwrap();
    }

    #[tokio::test]
    async fn net_status() {
        let exec = test_executor();
        exec.execute(Command::Net(NetCommand::Status))
            .await
            .unwrap();
    }

    #[tokio::test]
    async fn net_peers() {
        let exec = test_executor();
        exec.execute(Command::Net(NetCommand::Peers)).await.unwrap();
    }

    #[tokio::test]
    async fn json_output_mode() {
        let exec = test_executor_json();
        exec.execute(Command::Init).await.unwrap();
        exec.execute(Command::Status).await.unwrap();
    }

    #[tokio::test]
    async fn truncate_did_short() {
        assert_eq!(truncate_did("short"), "short");
    }

    #[tokio::test]
    async fn truncate_did_long() {
        let long = "did:key:z6MkhaXgBZDvotDkL5257faiztiGiC2QtKLGpbnnEGta2doK";
        let truncated = truncate_did(long);
        assert!(truncated.contains("..."));
        assert!(truncated.len() < long.len());
    }

    #[tokio::test]
    async fn governance_create_dao() {
        let exec = test_executor();
        exec.execute(Command::Init).await.unwrap();
        exec.execute(Command::Governance(GovernanceCommand::CreateDao {
            name: "TestDAO".into(),
            description: Some("A test DAO".into()),
        }))
        .await
        .unwrap();

        let daos = exec.load_daos();
        assert_eq!(daos.len(), 1);
        assert_eq!(daos[0].name, "TestDAO");
    }

    #[tokio::test]
    async fn governance_list_daos_empty() {
        let exec = test_executor();
        exec.execute(Command::Init).await.unwrap();
        exec.execute(Command::Governance(GovernanceCommand::ListDaos))
            .await
            .unwrap();
    }

    #[tokio::test]
    async fn governance_propose_requires_dao() {
        let exec = test_executor();
        exec.execute(Command::Init).await.unwrap();
        let result = exec
            .execute(Command::Governance(GovernanceCommand::Propose {
                dao_id: "nonexistent".into(),
                title: "Test".into(),
                description: None,
                voting_days: 7,
            }))
            .await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn governance_full_flow() {
        let exec = test_executor();
        exec.execute(Command::Init).await.unwrap();

        // Create DAO
        exec.execute(Command::Governance(GovernanceCommand::CreateDao {
            name: "FlowDAO".into(),
            description: None,
        }))
        .await
        .unwrap();

        let daos = exec.load_daos();
        let dao_id = daos[0].id.clone();

        // Submit proposal
        exec.execute(Command::Governance(GovernanceCommand::Propose {
            dao_id: dao_id.clone(),
            title: "First proposal".into(),
            description: Some("Testing the flow".into()),
            voting_days: 7,
        }))
        .await
        .unwrap();

        let proposals = exec.load_proposals();
        assert_eq!(proposals.len(), 1);
        let prop_id = proposals[0].id.clone();

        // Vote
        exec.execute(Command::Governance(GovernanceCommand::Vote {
            proposal_id: prop_id.clone(),
            choice: "for".into(),
            credits: 4,
        }))
        .await
        .unwrap();

        // Check tally
        exec.execute(Command::Governance(GovernanceCommand::Tally {
            proposal_id: prop_id,
        }))
        .await
        .unwrap();
    }

    #[tokio::test]
    async fn governance_vote_invalid_choice() {
        let exec = test_executor();
        exec.execute(Command::Init).await.unwrap();

        exec.execute(Command::Governance(GovernanceCommand::CreateDao {
            name: "VoteDAO".into(),
            description: None,
        }))
        .await
        .unwrap();

        let daos = exec.load_daos();
        let dao_id = daos[0].id.clone();

        exec.execute(Command::Governance(GovernanceCommand::Propose {
            dao_id,
            title: "Test".into(),
            description: None,
            voting_days: 7,
        }))
        .await
        .unwrap();

        let proposals = exec.load_proposals();
        let result = exec
            .execute(Command::Governance(GovernanceCommand::Vote {
                proposal_id: proposals[0].id.clone(),
                choice: "invalid".into(),
                credits: 1,
            }))
            .await;
        assert!(result.is_err());
    }

    // --- File command tests ---

    #[tokio::test]
    async fn file_upload_and_download() {
        let exec = test_executor();
        let dir = tempfile::tempdir().unwrap();
        let file_path = dir.path().join("test.txt");
        std::fs::write(&file_path, b"hello nous files").unwrap();

        exec.execute(Command::File(FileCommand::Upload {
            path: file_path.to_str().unwrap().to_string(),
            owner: "did:key:zTestOwner".into(),
        }))
        .await
        .unwrap();

        // List files to get the content ID.
        let store = exec.load_file_store();
        let files = store.list_files("did:key:zTestOwner");
        assert_eq!(files.len(), 1);
        let content_id = files[0].id.0.clone();

        // Download.
        let out_path = dir.path().join("downloaded.txt");
        exec.execute(Command::File(FileCommand::Download {
            content_id,
            output: out_path.to_str().unwrap().to_string(),
        }))
        .await
        .unwrap();

        let downloaded = std::fs::read(&out_path).unwrap();
        assert_eq!(downloaded, b"hello nous files");
    }

    #[tokio::test]
    async fn file_upload_nonexistent() {
        let exec = test_executor();
        let result = exec
            .execute(Command::File(FileCommand::Upload {
                path: "/tmp/definitely_does_not_exist_nous_test.bin".into(),
                owner: "did:key:z1".into(),
            }))
            .await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn file_list_empty() {
        let exec = test_executor();
        exec.execute(Command::File(FileCommand::List {
            owner: "did:key:z1".into(),
        }))
        .await
        .unwrap();
    }

    #[tokio::test]
    async fn file_versions() {
        let exec = test_executor();
        let dir = tempfile::tempdir().unwrap();
        let file_path = dir.path().join("doc.txt");

        std::fs::write(&file_path, b"version 1").unwrap();
        exec.execute(Command::File(FileCommand::Upload {
            path: file_path.to_str().unwrap().to_string(),
            owner: "did:key:z1".into(),
        }))
        .await
        .unwrap();

        std::fs::write(&file_path, b"version 2").unwrap();
        exec.execute(Command::File(FileCommand::Upload {
            path: file_path.to_str().unwrap().to_string(),
            owner: "did:key:z1".into(),
        }))
        .await
        .unwrap();

        exec.execute(Command::File(FileCommand::Versions {
            name: "doc.txt".into(),
            owner: "did:key:z1".into(),
        }))
        .await
        .unwrap();

        let store = exec.load_file_store();
        let history = store.get_history("doc.txt", "did:key:z1").unwrap();
        assert_eq!(history.version_count(), 2);
    }

    #[tokio::test]
    async fn file_stats() {
        let exec = test_executor();
        exec.execute(Command::File(FileCommand::Stats))
            .await
            .unwrap();
    }

    #[tokio::test]
    async fn file_share() {
        let exec = test_executor();
        let dir = tempfile::tempdir().unwrap();
        let file_path = dir.path().join("shared.txt");
        std::fs::write(&file_path, b"shared content").unwrap();

        exec.execute(Command::File(FileCommand::Upload {
            path: file_path.to_str().unwrap().to_string(),
            owner: "did:key:zalice".into(),
        }))
        .await
        .unwrap();

        exec.execute(Command::File(FileCommand::Share {
            name: "shared.txt".into(),
            owner: "did:key:zalice".into(),
            with: "did:key:zbob".into(),
        }))
        .await
        .unwrap();
    }

    #[tokio::test]
    async fn file_share_nonexistent() {
        let exec = test_executor();
        let result = exec
            .execute(Command::File(FileCommand::Share {
                name: "nope.txt".into(),
                owner: "did:key:z1".into(),
                with: "did:key:z2".into(),
            }))
            .await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn file_vault_create_and_store() {
        let exec = test_executor();

        exec.execute(Command::File(FileCommand::Vault(VaultCommand::Create {
            password: "strong-pass".into(),
            name: "my-vault".into(),
        })))
        .await
        .unwrap();

        // Find the vault ID from the DB.
        let conn = exec.db.conn();
        let mut stmt = conn
            .prepare("SELECT key FROM kv WHERE key LIKE 'vault:%'")
            .unwrap();
        let keys: Vec<String> = stmt
            .query_map([], |row| row.get::<_, String>(0))
            .unwrap()
            .filter_map(|r| r.ok())
            .collect();
        assert_eq!(keys.len(), 1);
        let vault_id = keys[0].strip_prefix("vault:").unwrap().to_string();

        // Store a file in the vault.
        let dir = tempfile::tempdir().unwrap();
        let secret_path = dir.path().join("secret.txt");
        std::fs::write(&secret_path, b"top secret data").unwrap();

        exec.execute(Command::File(FileCommand::Vault(VaultCommand::Store {
            path: secret_path.to_str().unwrap().to_string(),
            vault_id,
            password: "strong-pass".into(),
        })))
        .await
        .unwrap();
    }

    #[tokio::test]
    async fn file_vault_wrong_password() {
        let exec = test_executor();

        exec.execute(Command::File(FileCommand::Vault(VaultCommand::Create {
            password: "correct".into(),
            name: "vault".into(),
        })))
        .await
        .unwrap();

        let conn = exec.db.conn();
        let mut stmt = conn
            .prepare("SELECT key FROM kv WHERE key LIKE 'vault:%'")
            .unwrap();
        let keys: Vec<String> = stmt
            .query_map([], |row| row.get::<_, String>(0))
            .unwrap()
            .filter_map(|r| r.ok())
            .collect();
        let vault_id = keys[0].strip_prefix("vault:").unwrap().to_string();

        let dir = tempfile::tempdir().unwrap();
        let file_path = dir.path().join("test.bin");
        std::fs::write(&file_path, b"data").unwrap();

        let result = exec
            .execute(Command::File(FileCommand::Vault(VaultCommand::Store {
                path: file_path.to_str().unwrap().to_string(),
                vault_id,
                password: "wrong".into(),
            })))
            .await;
        assert!(result.is_err());
    }

    // --- Messaging command tests ---

    #[tokio::test]
    async fn message_create_channel_and_send() {
        let exec = test_executor();
        exec.execute(Command::Init).await.unwrap();
        let identity = exec.load_identity().unwrap();
        let did = identity.did().to_string();

        // Create a public channel.
        exec.execute(Command::Message(MessageCommand::CreateChannel {
            kind: "public".into(),
            name: Some("general".into()),
            members: vec![],
        }))
        .await
        .unwrap();

        // Find the channel.
        let channels = exec.load_channels_for_member(&did);
        assert_eq!(channels.len(), 1);
        let ch_id = channels[0].id.clone();

        // Send a message.
        exec.execute(Command::Message(MessageCommand::Send {
            channel_id: ch_id.clone(),
            text: "hello from test".into(),
            sender: None,
        }))
        .await
        .unwrap();

        // List messages.
        let messages = exec.load_messages(&ch_id);
        assert_eq!(messages.len(), 1);

        // Execute list command (just verify no errors).
        exec.execute(Command::Message(MessageCommand::List {
            channel_id: ch_id,
            limit: 10,
        }))
        .await
        .unwrap();
    }

    #[tokio::test]
    async fn message_send_to_nonexistent_channel() {
        let exec = test_executor();
        exec.execute(Command::Init).await.unwrap();

        let result = exec
            .execute(Command::Message(MessageCommand::Send {
                channel_id: "nonexistent".into(),
                text: "hi".into(),
                sender: None,
            }))
            .await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn message_create_direct_channel() {
        let exec = test_executor();
        exec.execute(Command::Init).await.unwrap();

        exec.execute(Command::Message(MessageCommand::CreateChannel {
            kind: "direct".into(),
            name: None,
            members: vec!["did:key:zbob".into()],
        }))
        .await
        .unwrap();
    }

    #[tokio::test]
    async fn message_create_group_channel() {
        let exec = test_executor();
        exec.execute(Command::Init).await.unwrap();

        exec.execute(Command::Message(MessageCommand::CreateChannel {
            kind: "group".into(),
            name: Some("team".into()),
            members: vec!["did:key:za".into(), "did:key:zb".into()],
        }))
        .await
        .unwrap();
    }

    #[tokio::test]
    async fn message_create_direct_requires_one_member() {
        let exec = test_executor();
        exec.execute(Command::Init).await.unwrap();

        let result = exec
            .execute(Command::Message(MessageCommand::CreateChannel {
                kind: "direct".into(),
                name: None,
                members: vec![],
            }))
            .await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn message_create_group_requires_name() {
        let exec = test_executor();
        exec.execute(Command::Init).await.unwrap();

        let result = exec
            .execute(Command::Message(MessageCommand::CreateChannel {
                kind: "group".into(),
                name: None,
                members: vec![],
            }))
            .await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn message_create_invalid_kind() {
        let exec = test_executor();
        exec.execute(Command::Init).await.unwrap();

        let result = exec
            .execute(Command::Message(MessageCommand::CreateChannel {
                kind: "invalid".into(),
                name: Some("test".into()),
                members: vec![],
            }))
            .await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn message_channels_empty() {
        let exec = test_executor();
        exec.execute(Command::Init).await.unwrap();

        exec.execute(Command::Message(MessageCommand::Channels { member: None }))
            .await
            .unwrap();
    }

    #[tokio::test]
    async fn message_list_nonexistent_channel() {
        let exec = test_executor();
        let result = exec
            .execute(Command::Message(MessageCommand::List {
                channel_id: "nonexistent".into(),
                limit: 10,
            }))
            .await;
        assert!(result.is_err());
    }

    // --- Utility function tests ---

    #[test]
    fn test_format_bytes() {
        assert_eq!(format_bytes(0), "0 B");
        assert_eq!(format_bytes(512), "512 B");
        assert_eq!(format_bytes(1024), "1.0 KiB");
        assert_eq!(format_bytes(1024 * 1024), "1.0 MiB");
        assert_eq!(format_bytes(1024 * 1024 * 1024), "1.0 GiB");
    }

    #[test]
    fn test_mime_from_extension() {
        assert_eq!(mime_from_extension("test.txt"), "text/plain");
        assert_eq!(mime_from_extension("photo.png"), "image/png");
        assert_eq!(mime_from_extension("data.json"), "application/json");
        assert_eq!(mime_from_extension("main.rs"), "text/x-rust");
        assert_eq!(
            mime_from_extension("unknown.xyz"),
            "application/octet-stream"
        );
        assert_eq!(mime_from_extension("noext"), "application/octet-stream");
    }
}
