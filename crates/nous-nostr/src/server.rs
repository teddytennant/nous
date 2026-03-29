use crate::event::Event;
use crate::message::{ClientMessage, RelayMessage};
use crate::relay::Relay;
use crate::subscription::SubscriptionManager;
use futures::stream::SplitSink;
use futures::{SinkExt, StreamExt};
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::broadcast;
use tokio_tungstenite::WebSocketStream;
use tokio_tungstenite::tungstenite::Message;
use tracing::{debug, error, info, warn};

/// A WebSocket-based NIP-01 relay server.
pub struct RelayServer {
    relay: Arc<Relay>,
    max_subscriptions_per_client: usize,
}

impl RelayServer {
    pub fn new(relay: Relay) -> Self {
        let max_subs = 20;
        Self {
            relay: Arc::new(relay),
            max_subscriptions_per_client: max_subs,
        }
    }

    /// Start listening on the given address. Returns when the server shuts down.
    pub async fn listen(self, addr: SocketAddr) -> Result<(), std::io::Error> {
        let listener = TcpListener::bind(addr).await?;
        info!(%addr, "nostr relay listening");

        let server = Arc::new(self);

        loop {
            let (stream, peer) = listener.accept().await?;
            let server = server.clone();
            tokio::spawn(async move {
                if let Err(e) = server.handle_connection(stream, peer).await {
                    warn!(%peer, error = %e, "connection error");
                }
            });
        }
    }

    /// Start listening and return the bound address. Useful for tests.
    pub async fn listen_with_addr(
        self,
        addr: SocketAddr,
    ) -> Result<(SocketAddr, tokio::task::JoinHandle<()>), std::io::Error> {
        let listener = TcpListener::bind(addr).await?;
        let bound_addr = listener.local_addr()?;
        info!(%bound_addr, "nostr relay listening");

        let server = Arc::new(self);

        let handle = tokio::spawn(async move {
            loop {
                match listener.accept().await {
                    Ok((stream, peer)) => {
                        let server = server.clone();
                        tokio::spawn(async move {
                            if let Err(e) = server.handle_connection(stream, peer).await {
                                warn!(%peer, error = %e, "connection error");
                            }
                        });
                    }
                    Err(e) => {
                        error!(error = %e, "accept error");
                        break;
                    }
                }
            }
        });

        Ok((bound_addr, handle))
    }

    async fn handle_connection(
        &self,
        stream: TcpStream,
        peer: SocketAddr,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        debug!(%peer, "new connection");

        let ws_stream = tokio_tungstenite::accept_async(stream).await?;
        let (mut write, mut read) = ws_stream.split();

        let sub_mgr = SubscriptionManager::new(self.max_subscriptions_per_client);
        let mut broadcast_rx = self.relay.subscribe_broadcast();

        loop {
            tokio::select! {
                msg = read.next() => {
                    match msg {
                        Some(Ok(Message::Text(text))) => {
                            self.handle_text_message(
                                &text,
                                &sub_mgr,
                                &mut write,
                            ).await?;
                        }
                        Some(Ok(Message::Close(_))) | None => {
                            debug!(%peer, "connection closed");
                            break;
                        }
                        Some(Ok(Message::Ping(data))) => {
                            write.send(Message::Pong(data)).await?;
                        }
                        Some(Err(e)) => {
                            warn!(%peer, error = %e, "websocket error");
                            break;
                        }
                        _ => {} // Ignore binary, pong, etc.
                    }
                }
                event = broadcast_rx.recv() => {
                    match event {
                        Ok(event) => {
                            self.forward_event_to_subscriptions(
                                &event,
                                &sub_mgr,
                                &mut write,
                            ).await?;
                        }
                        Err(broadcast::error::RecvError::Lagged(n)) => {
                            warn!(%peer, skipped = n, "client lagging, skipped events");
                        }
                        Err(broadcast::error::RecvError::Closed) => break,
                    }
                }
            }
        }

        sub_mgr.clear();
        Ok(())
    }

    async fn handle_text_message(
        &self,
        text: &str,
        sub_mgr: &SubscriptionManager,
        write: &mut SplitSink<WebSocketStream<TcpStream>, Message>,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let client_msg = match ClientMessage::from_json(text) {
            Ok(msg) => msg,
            Err(e) => {
                let notice = RelayMessage::Notice(format!("error: {e}"));
                write.send(Message::Text(notice.to_json())).await?;
                return Ok(());
            }
        };

        let responses = self.relay.handle_message(&client_msg, sub_mgr);
        for response in responses {
            write.send(Message::Text(response.to_json())).await?;
        }
        Ok(())
    }

    async fn forward_event_to_subscriptions(
        &self,
        event: &Event,
        sub_mgr: &SubscriptionManager,
        write: &mut SplitSink<WebSocketStream<TcpStream>, Message>,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let matching = sub_mgr.matching_subscriptions(event);
        for sub_id in matching {
            let msg = RelayMessage::Event {
                subscription_id: sub_id,
                event: event.clone(),
            };
            write.send(Message::Text(msg.to_json())).await?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::event::EventBuilder;
    use crate::filter::Filter;
    use crate::relay::RelayConfig;
    use ed25519_dalek::SigningKey;
    use futures::{SinkExt, StreamExt};
    use rand::rngs::OsRng;
    use tokio_tungstenite::connect_async;

    fn key() -> SigningKey {
        SigningKey::generate(&mut OsRng)
    }

    async fn start_test_relay() -> SocketAddr {
        let relay = Relay::new(RelayConfig {
            require_valid_signatures: true,
            ..Default::default()
        });
        let server = RelayServer::new(relay);
        let (addr, _handle) = server
            .listen_with_addr("127.0.0.1:0".parse().unwrap())
            .await
            .unwrap();
        addr
    }

    async fn connect(
        addr: SocketAddr,
    ) -> WebSocketStream<tokio_tungstenite::MaybeTlsStream<TcpStream>> {
        let url = format!("ws://{addr}");
        let (ws, _) = connect_async(&url).await.unwrap();
        ws
    }

    #[tokio::test]
    async fn client_sends_event_gets_ok() {
        let addr = start_test_relay().await;
        let mut ws = connect(addr).await;

        let k = key();
        let event = EventBuilder::text_note("hello relay")
            .created_at(1000)
            .sign(&k);
        let msg = ClientMessage::Event(event.clone());

        ws.send(Message::Text(msg.to_json().into())).await.unwrap();

        let response = ws.next().await.unwrap().unwrap();
        let text = response.into_text().unwrap();
        let relay_msg = RelayMessage::from_json(&text).unwrap();

        match relay_msg {
            RelayMessage::Ok {
                event_id, accepted, ..
            } => {
                assert_eq!(event_id, event.id);
                assert!(accepted);
            }
            other => panic!("expected OK, got {other:?}"),
        }
    }

    #[tokio::test]
    async fn client_subscribes_gets_stored_events() {
        let addr = start_test_relay().await;
        let mut ws = connect(addr).await;

        let k = key();

        // Send event first
        let event = EventBuilder::text_note("stored event")
            .created_at(1000)
            .sign(&k);
        ws.send(Message::Text(
            ClientMessage::Event(event.clone()).to_json().into(),
        ))
        .await
        .unwrap();
        // Read OK response
        ws.next().await.unwrap().unwrap();

        // Subscribe
        let req = ClientMessage::Req {
            subscription_id: "sub1".into(),
            filters: vec![Filter::new()],
        };
        ws.send(Message::Text(req.to_json().into())).await.unwrap();

        // Should get the stored event
        let resp = ws.next().await.unwrap().unwrap();
        let text = resp.into_text().unwrap();
        let relay_msg = RelayMessage::from_json(&text).unwrap();
        assert!(matches!(relay_msg, RelayMessage::Event { .. }));

        // Then EOSE
        let resp = ws.next().await.unwrap().unwrap();
        let text = resp.into_text().unwrap();
        let relay_msg = RelayMessage::from_json(&text).unwrap();
        assert!(matches!(relay_msg, RelayMessage::EndOfStoredEvents(_)));
    }

    #[tokio::test]
    async fn subscriber_receives_new_events() {
        let addr = start_test_relay().await;
        let mut ws1 = connect(addr).await;
        let mut ws2 = connect(addr).await;

        // ws1 subscribes to all
        let req = ClientMessage::Req {
            subscription_id: "all".into(),
            filters: vec![Filter::new()],
        };
        ws1.send(Message::Text(req.to_json().into())).await.unwrap();
        // Read EOSE
        ws1.next().await.unwrap().unwrap();

        // ws2 sends an event
        let k = key();
        let event = EventBuilder::text_note("live event")
            .created_at(2000)
            .sign(&k);
        ws2.send(Message::Text(
            ClientMessage::Event(event.clone()).to_json().into(),
        ))
        .await
        .unwrap();

        // ws1 should receive the event
        let resp = ws1.next().await.unwrap().unwrap();
        let text = resp.into_text().unwrap();
        let relay_msg = RelayMessage::from_json(&text).unwrap();
        match relay_msg {
            RelayMessage::Event {
                subscription_id,
                event: received,
            } => {
                assert_eq!(subscription_id, "all");
                assert_eq!(received.id, event.id);
            }
            other => panic!("expected Event, got {other:?}"),
        }
    }

    #[tokio::test]
    async fn invalid_json_returns_notice() {
        let addr = start_test_relay().await;
        let mut ws = connect(addr).await;

        ws.send(Message::Text("not json".into())).await.unwrap();

        let resp = ws.next().await.unwrap().unwrap();
        let text = resp.into_text().unwrap();
        let relay_msg = RelayMessage::from_json(&text).unwrap();
        assert!(matches!(relay_msg, RelayMessage::Notice(_)));
    }

    #[tokio::test]
    async fn close_subscription() {
        let addr = start_test_relay().await;
        let mut ws = connect(addr).await;

        // Subscribe
        let req = ClientMessage::Req {
            subscription_id: "temp".into(),
            filters: vec![Filter::new()],
        };
        ws.send(Message::Text(req.to_json().into())).await.unwrap();
        ws.next().await.unwrap().unwrap(); // EOSE

        // Close
        let close = ClientMessage::Close("temp".into());
        ws.send(Message::Text(close.to_json().into()))
            .await
            .unwrap();

        // Should not crash — no response expected for CLOSE
    }
}
