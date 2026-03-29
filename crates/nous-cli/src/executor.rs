use std::path::Path;

use nous_identity::Identity;
use nous_payments::Wallet;
use nous_social::{EventKind, Feed, FollowGraph, SignedEvent, Tag};
use nous_storage::Database;

use nous_cli::commands::*;
use nous_cli::output::Output;

pub struct Executor {
    output: Output,
    db: Database,
}

impl Executor {
    pub fn new(data_dir: &Path, json: bool) -> Result<Self, String> {
        std::fs::create_dir_all(data_dir)
            .map_err(|e| format!("failed to create data directory: {e}"))?;

        let db_path = data_dir.join("nous.db");
        let db =
            Database::open(&db_path).map_err(|e| format!("failed to open database: {e}"))?;

        Ok(Self {
            output: Output::new(json),
            db,
        })
    }

    pub async fn execute(&self, command: Command) -> Result<(), String> {
        match command {
            Command::Init => self.init(),
            Command::Identity(cmd) => self.identity(cmd),
            Command::Social(cmd) => self.social(cmd),
            Command::Wallet(cmd) => self.wallet(cmd),
            Command::Net(cmd) => self.net(cmd).await,
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
                            identity
                                .display_name()
                                .unwrap_or("(none)")
                                .to_string(),
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
                    self.output
                        .success(&format!("Not following {did}"));
                }
                Ok(())
            }
            SocialCommand::Following => {
                let identity = self
                    .load_identity()
                    .ok_or("No identity found.")?;
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
                        .map(|t| {
                            vec![
                                t.to_string(),
                                wallet.balance(t).to_string(),
                            ]
                        })
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
                wallet
                    .debit(&token, amount)
                    .map_err(|e| format!("{e}"))?;
                self.save_wallet(&wallet)?;

                let mut tx =
                    nous_payments::Transaction::new(identity.did(), &to, &token, amount);
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

    async fn net(&self, cmd: NetCommand) -> Result<(), String> {
        match cmd {
            NetCommand::Peers => {
                self.output.success("No peers connected.");
                Ok(())
            }
            NetCommand::Status => {
                self.output.table(
                    &["Property", "Value"],
                    &[
                        vec!["Protocol".into(), "libp2p".into()],
                        vec!["Transport".into(), "TCP + Noise + Yamux".into()],
                        vec!["Discovery".into(), "mDNS + Kademlia".into()],
                        vec!["Status".into(), "offline".into()],
                    ],
                );
                Ok(())
            }
            NetCommand::Connect { addr } => {
                self.output
                    .success(&format!("Connect to {addr}: not yet implemented in offline mode"));
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
            .unwrap_or_else(FollowGraph::new)
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
}

fn truncate_did(did: &str) -> String {
    if did.len() > 24 {
        format!("{}...{}", &did[..16], &did[did.len() - 6..])
    } else {
        did.to_string()
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
        let result = exec
            .execute(Command::Identity(IdentityCommand::Show))
            .await;
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
        exec.execute(Command::Net(NetCommand::Peers))
            .await
            .unwrap();
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
}
