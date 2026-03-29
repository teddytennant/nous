use crate::event::{Event, Kind};
use crate::message::{ClientMessage, RelayMessage};
use crate::store::EventStore;
use crate::subscription::{Subscription, SubscriptionManager};
use tokio::sync::broadcast;
use tracing::{debug, warn};

/// Configuration for the relay.
#[derive(Debug, Clone)]
pub struct RelayConfig {
    pub max_events: usize,
    pub max_subscriptions_per_client: usize,
    pub max_event_size_bytes: usize,
    pub require_valid_signatures: bool,
}

impl Default for RelayConfig {
    fn default() -> Self {
        Self {
            max_events: 100_000,
            max_subscriptions_per_client: 20,
            max_event_size_bytes: 65_536,
            require_valid_signatures: true,
        }
    }
}

/// The core relay engine. Handles event storage, subscriptions, and broadcasting.
#[derive(Clone)]
pub struct Relay {
    config: RelayConfig,
    store: EventStore,
    broadcast_tx: broadcast::Sender<Event>,
}

impl Relay {
    pub fn new(config: RelayConfig) -> Self {
        let store = EventStore::new(config.max_events);
        let (broadcast_tx, _) = broadcast::channel(1024);
        Self {
            config,
            store,
            broadcast_tx,
        }
    }

    pub fn with_defaults() -> Self {
        Self::new(RelayConfig::default())
    }

    /// Subscribe to the broadcast channel for new events.
    pub fn subscribe_broadcast(&self) -> broadcast::Receiver<Event> {
        self.broadcast_tx.subscribe()
    }

    /// Get a reference to the event store.
    pub fn store(&self) -> &EventStore {
        &self.store
    }

    /// Handle an incoming client message and produce relay responses.
    pub fn handle_message(
        &self,
        msg: &ClientMessage,
        sub_mgr: &SubscriptionManager,
    ) -> Vec<RelayMessage> {
        match msg {
            ClientMessage::Event(event) => self.handle_event(event),
            ClientMessage::Req {
                subscription_id,
                filters,
            } => self.handle_req(subscription_id, filters, sub_mgr),
            ClientMessage::Close(sub_id) => {
                self.handle_close(sub_id, sub_mgr);
                Vec::new()
            }
        }
    }

    fn handle_event(&self, event: &Event) -> Vec<RelayMessage> {
        // Validate event size
        let event_json = serde_json::to_string(event).unwrap_or_default();
        if event_json.len() > self.config.max_event_size_bytes {
            return vec![RelayMessage::Ok {
                event_id: event.id.clone(),
                accepted: false,
                message: format!(
                    "error: event too large ({} > {} bytes)",
                    event_json.len(),
                    self.config.max_event_size_bytes
                ),
            }];
        }

        // Validate ID
        if !event.verify_id() {
            return vec![RelayMessage::Ok {
                event_id: event.id.clone(),
                accepted: false,
                message: "invalid: event id does not match".into(),
            }];
        }

        // Validate signature
        if self.config.require_valid_signatures && !event.verify_signature() {
            return vec![RelayMessage::Ok {
                event_id: event.id.clone(),
                accepted: false,
                message: "invalid: bad signature".into(),
            }];
        }

        // Store
        let is_new = self.store.insert(event.clone());
        if !is_new {
            return vec![RelayMessage::Ok {
                event_id: event.id.clone(),
                accepted: true,
                message: "duplicate: already have this event".into(),
            }];
        }

        // NIP-09: Process deletion events
        if event.kind == Kind::DELETE {
            let mut deleted = 0usize;
            for tag in &event.tags {
                if tag.tag_name() == Some("e")
                    && let Some(target_id) = tag.value()
                {
                    // Only delete if the target event's pubkey matches the requester
                    if let Some(target_event) = self.store.get(target_id)
                        && target_event.pubkey == event.pubkey
                    {
                        self.store.delete(target_id);
                        deleted += 1;
                    }
                }
            }
            debug!(event_id = %event.id, deleted, "NIP-09 deletion processed");
        }

        // Broadcast to subscribers
        let _ = self.broadcast_tx.send(event.clone());
        debug!(event_id = %event.id, "event accepted and broadcast");

        vec![RelayMessage::Ok {
            event_id: event.id.clone(),
            accepted: true,
            message: String::new(),
        }]
    }

    fn handle_req(
        &self,
        subscription_id: &str,
        filters: &[crate::filter::Filter],
        sub_mgr: &SubscriptionManager,
    ) -> Vec<RelayMessage> {
        let sub = Subscription::new(subscription_id.to_string(), filters.to_vec());

        if let Err(e) = sub_mgr.add(sub) {
            warn!(error = %e, "failed to add subscription");
            return vec![RelayMessage::Notice(format!("error: {e}"))];
        }

        // Send stored events matching filters
        let mut responses: Vec<RelayMessage> = self
            .store
            .query_any(filters)
            .into_iter()
            .map(|event| RelayMessage::Event {
                subscription_id: subscription_id.to_string(),
                event,
            })
            .collect();

        // Send EOSE
        responses.push(RelayMessage::EndOfStoredEvents(subscription_id.to_string()));

        responses
    }

    fn handle_close(&self, sub_id: &str, sub_mgr: &SubscriptionManager) {
        if sub_mgr.remove(sub_id) {
            debug!(subscription_id = %sub_id, "subscription closed");
        } else {
            warn!(subscription_id = %sub_id, "close for unknown subscription");
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::event::{EventBuilder, Kind, Tag};
    use crate::filter::Filter;
    use ed25519_dalek::SigningKey;
    use rand::rngs::OsRng;

    fn key() -> SigningKey {
        SigningKey::generate(&mut OsRng)
    }

    fn relay() -> Relay {
        Relay::new(RelayConfig {
            max_events: 1000,
            max_subscriptions_per_client: 10,
            max_event_size_bytes: 65536,
            require_valid_signatures: true,
        })
    }

    fn sub_mgr() -> SubscriptionManager {
        SubscriptionManager::new(10)
    }

    #[test]
    fn accept_valid_event() {
        let r = relay();
        let k = key();
        let event = EventBuilder::text_note("hello").created_at(1000).sign(&k);
        let msg = ClientMessage::Event(event.clone());

        let responses = r.handle_message(&msg, &sub_mgr());
        assert_eq!(responses.len(), 1);
        if let RelayMessage::Ok {
            event_id,
            accepted,
            message,
        } = &responses[0]
        {
            assert_eq!(event_id, &event.id);
            assert!(accepted);
            assert!(message.is_empty());
        } else {
            panic!("expected OK");
        }
    }

    #[test]
    fn reject_invalid_id() {
        let r = relay();
        let k = key();
        let mut event = EventBuilder::text_note("hello").created_at(1000).sign(&k);
        event.id = "0000000000000000000000000000000000000000000000000000000000000000".into();

        let responses = r.handle_message(&ClientMessage::Event(event), &sub_mgr());
        if let RelayMessage::Ok {
            accepted, message, ..
        } = &responses[0]
        {
            assert!(!accepted);
            assert!(message.contains("id does not match"));
        }
    }

    #[test]
    fn reject_invalid_signature() {
        let r = relay();
        let k = key();
        let mut event = EventBuilder::text_note("hello").created_at(1000).sign(&k);
        let mut sig_bytes = hex::decode(&event.sig).unwrap();
        sig_bytes[0] ^= 0xff;
        event.sig = hex::encode(&sig_bytes);
        // Recompute ID to pass ID check
        event.id = Event::compute_id(
            &event.pubkey,
            event.created_at,
            event.kind,
            &event.tags,
            &event.content,
        );

        let responses = r.handle_message(&ClientMessage::Event(event), &sub_mgr());
        if let RelayMessage::Ok {
            accepted, message, ..
        } = &responses[0]
        {
            assert!(!accepted);
            assert!(message.contains("bad signature"));
        }
    }

    #[test]
    fn duplicate_event_accepted_with_note() {
        let r = relay();
        let k = key();
        let event = EventBuilder::text_note("hello").created_at(1000).sign(&k);

        r.handle_message(&ClientMessage::Event(event.clone()), &sub_mgr());
        let responses = r.handle_message(&ClientMessage::Event(event), &sub_mgr());
        if let RelayMessage::Ok {
            accepted, message, ..
        } = &responses[0]
        {
            assert!(accepted);
            assert!(message.contains("duplicate"));
        }
    }

    #[test]
    fn req_returns_stored_events_then_eose() {
        let r = relay();
        let k = key();
        let mgr = sub_mgr();

        // Store some events
        let e1 = EventBuilder::text_note("first").created_at(1000).sign(&k);
        let e2 = EventBuilder::text_note("second").created_at(2000).sign(&k);
        r.handle_message(&ClientMessage::Event(e1), &mgr);
        r.handle_message(&ClientMessage::Event(e2), &mgr);

        // Subscribe
        let req = ClientMessage::Req {
            subscription_id: "sub1".into(),
            filters: vec![Filter::new().kinds(vec![Kind::TEXT_NOTE])],
        };
        let responses = r.handle_message(&req, &mgr);

        // Should have 2 events + 1 EOSE
        assert_eq!(responses.len(), 3);
        assert!(matches!(&responses[0], RelayMessage::Event { .. }));
        assert!(matches!(&responses[1], RelayMessage::Event { .. }));
        assert!(matches!(
            &responses[2],
            RelayMessage::EndOfStoredEvents(id) if id == "sub1"
        ));
    }

    #[test]
    fn req_filters_correctly() {
        let r = relay();
        let k = key();
        let mgr = sub_mgr();

        r.handle_message(
            &ClientMessage::Event(EventBuilder::text_note("note").created_at(1000).sign(&k)),
            &mgr,
        );
        r.handle_message(
            &ClientMessage::Event(EventBuilder::metadata("{}").created_at(1001).sign(&k)),
            &mgr,
        );

        let req = ClientMessage::Req {
            subscription_id: "notes_only".into(),
            filters: vec![Filter::new().kinds(vec![Kind::METADATA])],
        };
        let responses = r.handle_message(&req, &mgr);

        // 1 metadata event + EOSE
        assert_eq!(responses.len(), 2);
    }

    #[test]
    fn close_removes_subscription() {
        let r = relay();
        let mgr = sub_mgr();

        let req = ClientMessage::Req {
            subscription_id: "s1".into(),
            filters: vec![Filter::new()],
        };
        r.handle_message(&req, &mgr);
        assert_eq!(mgr.len(), 1);

        r.handle_message(&ClientMessage::Close("s1".into()), &mgr);
        assert_eq!(mgr.len(), 0);
    }

    #[tokio::test]
    async fn broadcast_reaches_subscribers() {
        let r = relay();
        let mut rx = r.subscribe_broadcast();
        let k = key();

        let event = EventBuilder::text_note("broadcast me")
            .created_at(1000)
            .sign(&k);
        r.handle_message(&ClientMessage::Event(event.clone()), &sub_mgr());

        let received = rx.recv().await.unwrap();
        assert_eq!(received.id, event.id);
    }

    #[test]
    fn reject_oversized_event() {
        let r = Relay::new(RelayConfig {
            max_event_size_bytes: 100,
            ..Default::default()
        });
        let k = key();
        let big_content = "x".repeat(200);
        let event = EventBuilder::text_note(big_content)
            .created_at(1000)
            .sign(&k);

        let responses = r.handle_message(&ClientMessage::Event(event), &sub_mgr());
        if let RelayMessage::Ok {
            accepted, message, ..
        } = &responses[0]
        {
            assert!(!accepted);
            assert!(message.contains("too large"));
        }
    }

    #[test]
    fn event_stored_after_acceptance() {
        let r = relay();
        let k = key();
        let event = EventBuilder::text_note("stored").created_at(1000).sign(&k);
        let id = event.id.clone();

        r.handle_message(&ClientMessage::Event(event), &sub_mgr());
        assert!(r.store().get(&id).is_some());
    }

    // ── NIP-09: Event Deletion ─────────────────────────────────

    #[test]
    fn nip09_delete_own_event() {
        let r = relay();
        let k = key();
        let mgr = sub_mgr();

        // Publish a text note
        let note = EventBuilder::text_note("delete me")
            .created_at(1000)
            .sign(&k);
        let note_id = note.id.clone();
        r.handle_message(&ClientMessage::Event(note), &mgr);
        assert!(r.store().get(&note_id).is_some());

        // Send deletion event
        let deletion = EventBuilder::deletion(&[&note_id], "spam").sign(&k);
        let responses = r.handle_message(&ClientMessage::Event(deletion), &mgr);
        assert!(matches!(
            &responses[0],
            RelayMessage::Ok { accepted: true, .. }
        ));

        // Original event should be gone
        assert!(r.store().get(&note_id).is_none());
    }

    #[test]
    fn nip09_cannot_delete_other_users_event() {
        let r = relay();
        let alice = key();
        let bob = key();
        let mgr = sub_mgr();

        // Alice publishes
        let note = EventBuilder::text_note("alice's post")
            .created_at(1000)
            .sign(&alice);
        let note_id = note.id.clone();
        r.handle_message(&ClientMessage::Event(note), &mgr);

        // Bob tries to delete Alice's event
        let deletion = EventBuilder::deletion(&[&note_id], "").sign(&bob);
        r.handle_message(&ClientMessage::Event(deletion), &mgr);

        // Alice's event should still exist
        assert!(r.store().get(&note_id).is_some());
    }

    #[test]
    fn nip09_delete_multiple_events() {
        let r = relay();
        let k = key();
        let mgr = sub_mgr();

        let e1 = EventBuilder::text_note("one").created_at(1000).sign(&k);
        let e2 = EventBuilder::text_note("two").created_at(2000).sign(&k);
        let e3 = EventBuilder::text_note("three").created_at(3000).sign(&k);
        let id1 = e1.id.clone();
        let id2 = e2.id.clone();
        let id3 = e3.id.clone();

        r.handle_message(&ClientMessage::Event(e1), &mgr);
        r.handle_message(&ClientMessage::Event(e2), &mgr);
        r.handle_message(&ClientMessage::Event(e3), &mgr);
        assert_eq!(r.store().len(), 3);

        // Delete first two
        let deletion = EventBuilder::deletion(&[&id1, &id2], "cleanup").sign(&k);
        r.handle_message(&ClientMessage::Event(deletion), &mgr);

        assert!(r.store().get(&id1).is_none());
        assert!(r.store().get(&id2).is_none());
        assert!(r.store().get(&id3).is_some());
        // deletion event itself + e3 remain
        assert_eq!(r.store().len(), 2);
    }

    #[test]
    fn nip09_deletion_event_is_stored() {
        let r = relay();
        let k = key();
        let mgr = sub_mgr();

        let note = EventBuilder::text_note("ephemeral")
            .created_at(1000)
            .sign(&k);
        let note_id = note.id.clone();
        r.handle_message(&ClientMessage::Event(note), &mgr);

        let deletion = EventBuilder::deletion(&[&note_id], "reason").sign(&k);
        let del_id = deletion.id.clone();
        r.handle_message(&ClientMessage::Event(deletion), &mgr);

        // Deletion event itself should be stored
        assert!(r.store().get(&del_id).is_some());
    }

    #[test]
    fn nip09_delete_nonexistent_event_ok() {
        let r = relay();
        let k = key();
        let mgr = sub_mgr();

        let deletion = EventBuilder::deletion(&["nonexistent_id"], "").sign(&k);
        let responses = r.handle_message(&ClientMessage::Event(deletion), &mgr);
        assert!(matches!(
            &responses[0],
            RelayMessage::Ok { accepted: true, .. }
        ));
    }

    #[test]
    fn relay_with_defaults_works() {
        let r = Relay::with_defaults();
        let k = key();
        let event = EventBuilder::text_note("default").created_at(1000).sign(&k);
        let responses = r.handle_message(&ClientMessage::Event(event), &sub_mgr());
        assert!(!responses.is_empty());
    }
}
