use std::convert::Infallible;
use std::sync::Arc;

use axum::extract::State;
use axum::extract::ws::{Message, WebSocket, WebSocketUpgrade};
use axum::response::IntoResponse;
use axum::response::sse::{Event, KeepAlive, Sse};
use futures::stream::Stream;
use tokio_stream::StreamExt;
use tokio_stream::wrappers::BroadcastStream;

use crate::state::{AppState, RealtimeEvent};

/// WebSocket upgrade handler.
///
/// Clients connect to `/api/v1/ws` and receive JSON-encoded `RealtimeEvent`
/// messages for all mutations happening on the server.
pub async fn ws_handler(
    ws: WebSocketUpgrade,
    State(state): State<Arc<AppState>>,
) -> impl IntoResponse {
    ws.on_upgrade(move |socket| handle_ws(socket, state))
}

async fn handle_ws(mut socket: WebSocket, state: Arc<AppState>) {
    let mut rx = state.events.subscribe();

    loop {
        tokio::select! {
            // Forward server events to the WebSocket client
            result = rx.recv() => {
                match result {
                    Ok(event) => {
                        let json = match serde_json::to_string(&event) {
                            Ok(j) => j,
                            Err(_) => continue,
                        };
                        if socket.send(Message::Text(json.into())).await.is_err() {
                            break;
                        }
                    }
                    Err(tokio::sync::broadcast::error::RecvError::Lagged(n)) => {
                        tracing::warn!(skipped = n, "WebSocket client lagged");
                    }
                    Err(tokio::sync::broadcast::error::RecvError::Closed) => break,
                }
            }
            // Handle incoming client messages (ping/pong, close)
            msg = socket.recv() => {
                match msg {
                    Some(Ok(Message::Close(_))) | None => break,
                    Some(Ok(Message::Ping(data))) => {
                        if socket.send(Message::Pong(data)).await.is_err() {
                            break;
                        }
                    }
                    _ => {}
                }
            }
        }
    }
}

/// Server-Sent Events stream handler.
///
/// Clients connect to `/api/v1/events` and receive a stream of JSON events.
/// This is the preferred method for browser clients that don't need
/// bidirectional communication.
pub async fn sse_handler(
    State(state): State<Arc<AppState>>,
) -> Sse<impl Stream<Item = Result<Event, Infallible>>> {
    let rx = state.events.subscribe();
    let stream = BroadcastStream::new(rx).filter_map(|result| match result {
        Ok(event) => {
            let json = serde_json::to_string(&event).ok()?;
            let event_type = match &event {
                RealtimeEvent::NewPost { .. } => "new_post",
                RealtimeEvent::NewMessage { .. } => "new_message",
                RealtimeEvent::VoteCast { .. } => "vote_cast",
                RealtimeEvent::DaoCreated { .. } => "dao_created",
                RealtimeEvent::ProposalCreated { .. } => "proposal_created",
                RealtimeEvent::Transfer { .. } => "transfer",
                RealtimeEvent::ListingUpdate { .. } => "listing_update",
            };
            Some(Ok(Event::default().event(event_type).data(json)))
        }
        Err(_) => None,
    });

    Sse::new(stream).keep_alive(KeepAlive::default())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::ApiConfig;

    #[test]
    fn realtime_event_serializes() {
        let event = RealtimeEvent::NewPost {
            id: "post1".into(),
            author: "did:key:z123".into(),
            content: "hello".into(),
        };
        let json = serde_json::to_string(&event).unwrap();
        assert!(json.contains("\"type\":\"NewPost\""));
        assert!(json.contains("\"author\":\"did:key:z123\""));
    }

    #[test]
    fn realtime_event_message_serializes() {
        let event = RealtimeEvent::NewMessage {
            channel_id: "ch1".into(),
            sender: "did:key:z456".into(),
            content: "hey there".into(),
        };
        let json = serde_json::to_string(&event).unwrap();
        assert!(json.contains("\"type\":\"NewMessage\""));
        assert!(json.contains("\"channel_id\":\"ch1\""));
    }

    #[test]
    fn realtime_event_vote_serializes() {
        let event = RealtimeEvent::VoteCast {
            proposal_id: "p1".into(),
            voter: "did:key:z789".into(),
        };
        let json = serde_json::to_string(&event).unwrap();
        assert!(json.contains("\"type\":\"VoteCast\""));
    }

    #[test]
    fn realtime_event_transfer_serializes() {
        let event = RealtimeEvent::Transfer {
            from: "did:key:z1".into(),
            to: "did:key:z2".into(),
            amount: "100".into(),
            token: "NOUS".into(),
        };
        let json = serde_json::to_string(&event).unwrap();
        assert!(json.contains("\"token\":\"NOUS\""));
    }

    #[test]
    fn realtime_event_dao_serializes() {
        let event = RealtimeEvent::DaoCreated {
            id: "dao1".into(),
            name: "Test DAO".into(),
        };
        let json = serde_json::to_string(&event).unwrap();
        assert!(json.contains("\"type\":\"DaoCreated\""));
    }

    #[test]
    fn realtime_event_proposal_serializes() {
        let event = RealtimeEvent::ProposalCreated {
            id: "p1".into(),
            title: "Fund dev".into(),
            dao_id: "dao1".into(),
        };
        let json = serde_json::to_string(&event).unwrap();
        assert!(json.contains("\"type\":\"ProposalCreated\""));
    }

    #[test]
    fn realtime_event_listing_serializes() {
        let event = RealtimeEvent::ListingUpdate {
            id: "l1".into(),
            title: "Widget".into(),
        };
        let json = serde_json::to_string(&event).unwrap();
        assert!(json.contains("\"type\":\"ListingUpdate\""));
    }

    #[tokio::test]
    async fn broadcast_channel_delivers() {
        let state = AppState::new(ApiConfig::default());
        let mut rx = state.events.subscribe();

        state.emit(RealtimeEvent::NewPost {
            id: "p1".into(),
            author: "alice".into(),
            content: "hi".into(),
        });

        let event = rx.recv().await.unwrap();
        match event {
            RealtimeEvent::NewPost { id, .. } => assert_eq!(id, "p1"),
            _ => panic!("unexpected event type"),
        }
    }

    #[tokio::test]
    async fn broadcast_multiple_subscribers() {
        let state = AppState::new(ApiConfig::default());
        let mut rx1 = state.events.subscribe();
        let mut rx2 = state.events.subscribe();

        state.emit(RealtimeEvent::DaoCreated {
            id: "d1".into(),
            name: "DAO".into(),
        });

        let e1 = rx1.recv().await.unwrap();
        let e2 = rx2.recv().await.unwrap();

        match (e1, e2) {
            (
                RealtimeEvent::DaoCreated { id: id1, .. },
                RealtimeEvent::DaoCreated { id: id2, .. },
            ) => {
                assert_eq!(id1, "d1");
                assert_eq!(id2, "d1");
            }
            _ => panic!("unexpected event types"),
        }
    }

    #[tokio::test]
    async fn emit_on_no_subscribers_does_not_panic() {
        let state = AppState::new(ApiConfig::default());
        // No subscribers — emit should silently drop
        state.emit(RealtimeEvent::Transfer {
            from: "a".into(),
            to: "b".into(),
            amount: "1".into(),
            token: "ETH".into(),
        });
    }
}
