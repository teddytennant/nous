use std::io::Write;
use std::path::Path;

use nous_governance::{Ballot, Dao, ProposalBuilder, QuadraticVoting, VoteChoice, VoteTally};
use nous_identity::Identity;
use nous_payments::Wallet;
use nous_social::{EventKind, Feed, FollowGraph, SignedEvent, Tag};
use nous_storage::Database;
use nous_terminal::{Terminal, TerminalConfig};

use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyModifiers};
use crossterm::terminal::{self, EnterAlternateScreen, LeaveAlternateScreen};
use crossterm::{cursor, execute, queue, style};

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
        let db = Database::open(&db_path).map_err(|e| format!("failed to open database: {e}"))?;

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
            Command::Governance(cmd) => self.governance(cmd).await,
            Command::Net(cmd) => self.net(cmd).await,
            Command::Terminal => Self::run_terminal(),
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
                self.output.success(&format!(
                    "Connect to {addr}: not yet implemented in offline mode"
                ));
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

    /// Launch an embedded terminal using nous-terminal and crossterm raw mode.
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
}

fn truncate_did(did: &str) -> String {
    if did.len() > 24 {
        format!("{}...{}", &did[..16], &did[did.len() - 6..])
    } else {
        did.to_string()
    }
}

/// Convert a nous-terminal Color to a crossterm Color.
fn nous_color_to_crossterm(color: nous_terminal::Color) -> crossterm::style::Color {
    match color {
        nous_terminal::Color::Default => crossterm::style::Color::Reset,
        nous_terminal::Color::Rgb(r, g, b) => crossterm::style::Color::Rgb { r, g, b },
        nous_terminal::Color::Indexed(idx) => crossterm::style::Color::AnsiValue(idx),
    }
}

/// Convert a crossterm KeyEvent into the byte sequence expected by a PTY.
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
}
