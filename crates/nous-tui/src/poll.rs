use std::sync::Arc;
use std::time::Duration;

use tokio::sync::mpsc;

use crate::app::{DisplayMessage, FeedItem};
use crate::client::{ApiClient, BalanceEntry, ChannelListItem, DaoItem, ProposalItem};

/// Events produced by the background poller and consumed by the main loop.
#[derive(Debug)]
pub enum PollEvent {
    Health {
        status: String,
        version: String,
        uptime_ms: u64,
    },
    Feed(Vec<FeedItem>),
    Channels(Vec<ChannelListItem>),
    Messages(Vec<DisplayMessage>),
    Balances(Vec<BalanceEntry>),
    Daos(Vec<DaoItem>),
    Proposals(Vec<ProposalItem>),
    Disconnected,
}

/// Configuration for polling intervals.
#[derive(Debug, Clone)]
pub struct PollConfig {
    pub health_interval: Duration,
    pub feed_interval: Duration,
    pub messages_interval: Duration,
    pub governance_interval: Duration,
    pub wallet_interval: Duration,
}

impl Default for PollConfig {
    fn default() -> Self {
        Self {
            health_interval: Duration::from_secs(5),
            feed_interval: Duration::from_secs(10),
            messages_interval: Duration::from_secs(3),
            governance_interval: Duration::from_secs(15),
            wallet_interval: Duration::from_secs(15),
        }
    }
}

/// Spawn a background poller that fetches data from the API at regular intervals.
///
/// Returns the sender handle (for shutdown) and receiver for poll events.
/// Drop the returned `JoinHandle` or abort it to stop polling.
pub fn spawn_poller(
    client: ApiClient,
    did: String,
    config: PollConfig,
) -> (mpsc::Receiver<PollEvent>, tokio::task::JoinHandle<()>) {
    let (tx, rx) = mpsc::channel(64);
    let handle = tokio::spawn(poll_loop(client, did, config, tx));
    (rx, handle)
}

async fn poll_loop(
    client: ApiClient,
    did: String,
    config: PollConfig,
    tx: mpsc::Sender<PollEvent>,
) {
    let mut health_tick = tokio::time::interval(config.health_interval);
    let mut feed_tick = tokio::time::interval(config.feed_interval);
    let mut msg_tick = tokio::time::interval(config.messages_interval);
    let mut gov_tick = tokio::time::interval(config.governance_interval);
    let mut wallet_tick = tokio::time::interval(config.wallet_interval);

    loop {
        tokio::select! {
            _ = health_tick.tick() => {
                match client.health().await {
                    Ok(h) => {
                        let _ = tx.send(PollEvent::Health {
                            status: h.status,
                            version: h.version,
                            uptime_ms: h.uptime_ms,
                        }).await;
                    }
                    Err(_) => {
                        let _ = tx.send(PollEvent::Disconnected).await;
                    }
                }
            }
            _ = feed_tick.tick() => {
                if let Ok(resp) = client.feed(50).await {
                    let items: Vec<FeedItem> = resp.events.into_iter().map(|e| {
                        FeedItem {
                            author: e.pubkey,
                            content: e.content,
                            timestamp: e.created_at,
                            reactions: 0,
                            replies: 0,
                        }
                    }).collect();
                    let _ = tx.send(PollEvent::Feed(items)).await;
                }
            }
            _ = msg_tick.tick() => {
                if let Ok(channels) = client.channels(&did).await {
                    let _ = tx.send(PollEvent::Channels(channels)).await;
                }
            }
            _ = gov_tick.tick() => {
                if let Ok(resp) = client.daos().await {
                    let _ = tx.send(PollEvent::Daos(resp.daos)).await;
                }
                if let Ok(resp) = client.proposals().await {
                    let _ = tx.send(PollEvent::Proposals(resp.proposals)).await;
                }
            }
            _ = wallet_tick.tick() => {
                if !did.is_empty() {
                    if let Ok(resp) = client.wallet(&did).await {
                        let _ = tx.send(PollEvent::Balances(resp.balances)).await;
                    }
                }
            }
        }

        if tx.is_closed() {
            break;
        }
    }
}

/// Apply a poll event to the app state.
pub fn apply_event(app: &mut crate::app::App, event: PollEvent) {
    match event {
        PollEvent::Health {
            status,
            version,
            uptime_ms,
        } => {
            app.node_status = Some(status);
            app.node_version = Some(version);
            app.node_uptime = Some(uptime_ms);
            app.connected = true;
        }
        PollEvent::Feed(items) => {
            app.feed_items = items;
        }
        PollEvent::Channels(channels) => {
            app.channels = channels;
        }
        PollEvent::Messages(messages) => {
            for msg in messages {
                app.add_message(msg);
            }
        }
        PollEvent::Balances(balances) => {
            app.balances = balances;
        }
        PollEvent::Daos(daos) => {
            app.daos = daos;
        }
        PollEvent::Proposals(proposals) => {
            app.proposals = proposals;
        }
        PollEvent::Disconnected => {
            app.connected = false;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::TuiConfig;

    fn test_app() -> crate::app::App {
        crate::app::App::new(TuiConfig::default())
    }

    #[test]
    fn poll_config_defaults() {
        let cfg = PollConfig::default();
        assert_eq!(cfg.health_interval, Duration::from_secs(5));
        assert_eq!(cfg.feed_interval, Duration::from_secs(10));
        assert_eq!(cfg.messages_interval, Duration::from_secs(3));
    }

    #[test]
    fn apply_health_event() {
        let mut app = test_app();
        assert!(!app.connected);

        apply_event(
            &mut app,
            PollEvent::Health {
                status: "running".into(),
                version: "0.1.0".into(),
                uptime_ms: 60000,
            },
        );

        assert!(app.connected);
        assert_eq!(app.node_status, Some("running".into()));
        assert_eq!(app.node_version, Some("0.1.0".into()));
        assert_eq!(app.node_uptime, Some(60000));
    }

    #[test]
    fn apply_disconnected_event() {
        let mut app = test_app();
        app.connected = true;
        apply_event(&mut app, PollEvent::Disconnected);
        assert!(!app.connected);
    }

    #[test]
    fn apply_feed_event() {
        let mut app = test_app();
        let items = vec![
            FeedItem {
                author: "alice".into(),
                content: "hello".into(),
                timestamp: "12:00".into(),
                reactions: 1,
                replies: 0,
            },
            FeedItem {
                author: "bob".into(),
                content: "world".into(),
                timestamp: "12:01".into(),
                reactions: 0,
                replies: 0,
            },
        ];

        apply_event(&mut app, PollEvent::Feed(items));
        assert_eq!(app.feed_items.len(), 2);
        assert_eq!(app.feed_items[0].author, "alice");
    }

    #[test]
    fn apply_balances_event() {
        let mut app = test_app();
        let balances = vec![BalanceEntry {
            token: "ETH".into(),
            amount: "2.5".into(),
        }];

        apply_event(&mut app, PollEvent::Balances(balances));
        assert_eq!(app.balances.len(), 1);
        assert_eq!(app.balances[0].token, "ETH");
    }

    #[test]
    fn apply_daos_event() {
        let mut app = test_app();
        let daos = vec![DaoItem {
            id: "d1".into(),
            name: "Test DAO".into(),
            description: "desc".into(),
            founder_did: "did:key:z123".into(),
            member_count: 5,
            created_at: "2026-03-29".into(),
        }];

        apply_event(&mut app, PollEvent::Daos(daos));
        assert_eq!(app.daos.len(), 1);
        assert_eq!(app.daos[0].name, "Test DAO");
    }

    #[test]
    fn apply_proposals_event() {
        let mut app = test_app();
        let proposals = vec![ProposalItem {
            id: "p1".into(),
            dao_id: "d1".into(),
            title: "Fund".into(),
            description: "desc".into(),
            proposer_did: "did:key:z123".into(),
            status: "Active".into(),
            created_at: "2026-03-29".into(),
        }];

        apply_event(&mut app, PollEvent::Proposals(proposals));
        assert_eq!(app.proposals.len(), 1);
    }

    #[test]
    fn apply_channels_event() {
        let mut app = test_app();
        let channels = vec![ChannelListItem {
            id: "ch1".into(),
            kind: "group".into(),
            name: Some("General".into()),
            members: vec!["a".into(), "b".into()],
            created_at: "2026-03-29".into(),
        }];

        apply_event(&mut app, PollEvent::Channels(channels));
        assert_eq!(app.channels.len(), 1);
    }

    #[test]
    fn apply_messages_event() {
        let mut app = test_app();
        let messages = vec![
            DisplayMessage {
                sender: "alice".into(),
                content: "hey".into(),
                timestamp: "12:00".into(),
            },
            DisplayMessage {
                sender: "bob".into(),
                content: "hi".into(),
                timestamp: "12:01".into(),
            },
        ];

        apply_event(&mut app, PollEvent::Messages(messages));
        assert_eq!(app.messages.len(), 2);
    }

    #[tokio::test]
    async fn spawn_poller_returns_handle() {
        let client = ApiClient::new("http://localhost:1/api/v1");
        let (mut rx, handle) = spawn_poller(
            client,
            "did:key:z123".into(),
            PollConfig {
                health_interval: Duration::from_millis(50),
                feed_interval: Duration::from_secs(100),
                messages_interval: Duration::from_secs(100),
                governance_interval: Duration::from_secs(100),
                wallet_interval: Duration::from_secs(100),
            },
        );

        // The poller should emit a Disconnected event quickly since the server is down
        let event = tokio::time::timeout(Duration::from_secs(2), rx.recv()).await;
        assert!(event.is_ok());
        handle.abort();
    }
}
