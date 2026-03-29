use crate::event::Event;
use crate::filter::Filter;
use std::collections::HashMap;
use std::sync::{Arc, RwLock};

/// Thread-safe in-memory event store for the relay.
#[derive(Debug, Clone)]
pub struct EventStore {
    events: Arc<RwLock<HashMap<String, Event>>>,
    max_events: usize,
}

impl EventStore {
    pub fn new(max_events: usize) -> Self {
        Self {
            events: Arc::new(RwLock::new(HashMap::new())),
            max_events,
        }
    }

    /// Insert an event. Returns `true` if the event was new, `false` if duplicate.
    pub fn insert(&self, event: Event) -> bool {
        let mut events = self.events.write().unwrap();

        if events.contains_key(&event.id) {
            return false;
        }

        // Evict oldest if at capacity
        if events.len() >= self.max_events {
            if let Some(oldest_id) = events
                .values()
                .min_by_key(|e| e.created_at)
                .map(|e| e.id.clone())
            {
                events.remove(&oldest_id);
            }
        }

        events.insert(event.id.clone(), event);
        true
    }

    /// Query events matching a filter.
    pub fn query(&self, filter: &Filter) -> Vec<Event> {
        let events = self.events.read().unwrap();
        let all: Vec<Event> = events.values().cloned().collect();
        filter
            .apply(&all)
            .into_iter()
            .cloned()
            .collect()
    }

    /// Query events matching any of the given filters.
    pub fn query_any(&self, filters: &[Filter]) -> Vec<Event> {
        let events = self.events.read().unwrap();
        let all: Vec<Event> = events.values().cloned().collect();

        let mut matched: Vec<Event> = all
            .iter()
            .filter(|e| filters.iter().any(|f| f.matches(e)))
            .cloned()
            .collect();

        matched.sort_by(|a, b| b.created_at.cmp(&a.created_at));
        matched
    }

    /// Get an event by ID.
    pub fn get(&self, id: &str) -> Option<Event> {
        self.events.read().unwrap().get(id).cloned()
    }

    /// Delete an event by ID.
    pub fn delete(&self, id: &str) -> bool {
        self.events.write().unwrap().remove(id).is_some()
    }

    /// Total number of stored events.
    pub fn len(&self) -> usize {
        self.events.read().unwrap().len()
    }

    /// Whether the store is empty.
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Delete events matching a filter (for NIP-09 deletion events).
    pub fn delete_matching(&self, filter: &Filter) -> usize {
        let to_delete: Vec<String> = {
            let events = self.events.read().unwrap();
            let all: Vec<Event> = events.values().cloned().collect();
            all.iter()
                .filter(|e| filter.matches(e))
                .map(|e| e.id.clone())
                .collect()
        };
        let count = to_delete.len();
        let mut events = self.events.write().unwrap();
        for id in &to_delete {
            events.remove(id);
        }
        count
    }
}

impl Default for EventStore {
    fn default() -> Self {
        Self::new(100_000)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::event::{EventBuilder, Kind, Tag};
    use ed25519_dalek::SigningKey;
    use rand::rngs::OsRng;

    fn key() -> SigningKey {
        SigningKey::generate(&mut OsRng)
    }

    fn note(content: &str, ts: u64) -> Event {
        EventBuilder::text_note(content)
            .created_at(ts)
            .sign(&key())
    }

    #[test]
    fn insert_and_get() {
        let store = EventStore::new(100);
        let e = note("hello", 1000);
        let id = e.id.clone();

        assert!(store.insert(e));
        assert!(store.get(&id).is_some());
        assert_eq!(store.len(), 1);
    }

    #[test]
    fn reject_duplicates() {
        let store = EventStore::new(100);
        let e = note("hello", 1000);

        assert!(store.insert(e.clone()));
        assert!(!store.insert(e));
        assert_eq!(store.len(), 1);
    }

    #[test]
    fn evict_oldest_at_capacity() {
        let store = EventStore::new(3);
        let e1 = note("first", 1000);
        let e2 = note("second", 2000);
        let e3 = note("third", 3000);
        let e4 = note("fourth", 4000);
        let e1_id = e1.id.clone();

        store.insert(e1);
        store.insert(e2);
        store.insert(e3);
        assert_eq!(store.len(), 3);

        store.insert(e4);
        assert_eq!(store.len(), 3);
        // Oldest (e1) should be evicted
        assert!(store.get(&e1_id).is_none());
    }

    #[test]
    fn query_by_kind() {
        let store = EventStore::new(100);
        let k = key();
        store.insert(EventBuilder::text_note("note").created_at(1000).sign(&k));
        store.insert(EventBuilder::metadata("{}").created_at(1001).sign(&k));

        let results = store.query(&Filter::new().kinds(vec![Kind::TEXT_NOTE]));
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].kind, Kind::TEXT_NOTE);
    }

    #[test]
    fn query_with_limit() {
        let store = EventStore::new(100);
        for i in 0..10 {
            store.insert(note(&format!("msg {i}"), 1000 + i));
        }

        let results = store.query(&Filter::new().limit(3));
        assert_eq!(results.len(), 3);
    }

    #[test]
    fn query_any_multiple_filters() {
        let store = EventStore::new(100);
        let k = key();
        store.insert(EventBuilder::text_note("note").created_at(1000).sign(&k));
        store.insert(EventBuilder::metadata("{}").created_at(1001).sign(&k));
        store.insert(
            EventBuilder::new(Kind::CONTACTS, "contacts")
                .created_at(1002)
                .sign(&k),
        );

        let results = store.query_any(&[
            Filter::new().kinds(vec![Kind::TEXT_NOTE]),
            Filter::new().kinds(vec![Kind::METADATA]),
        ]);
        assert_eq!(results.len(), 2);
    }

    #[test]
    fn delete_event() {
        let store = EventStore::new(100);
        let e = note("del", 1000);
        let id = e.id.clone();

        store.insert(e);
        assert!(store.delete(&id));
        assert!(store.get(&id).is_none());
        assert!(!store.delete(&id));
    }

    #[test]
    fn delete_matching() {
        let store = EventStore::new(100);
        let k = key();
        store.insert(EventBuilder::text_note("a").created_at(1000).sign(&k));
        store.insert(EventBuilder::text_note("b").created_at(1001).sign(&k));
        store.insert(EventBuilder::metadata("{}").created_at(1002).sign(&k));

        let deleted = store.delete_matching(&Filter::new().kinds(vec![Kind::TEXT_NOTE]));
        assert_eq!(deleted, 2);
        assert_eq!(store.len(), 1);
    }

    #[test]
    fn empty_store() {
        let store = EventStore::new(100);
        assert!(store.is_empty());
        assert_eq!(store.len(), 0);
        assert!(store.get("nonexistent").is_none());
    }

    #[test]
    fn thread_safe_concurrent_access() {
        use std::thread;

        let store = EventStore::new(1000);
        let store_clone = store.clone();

        let handle = thread::spawn(move || {
            for i in 0..50 {
                store_clone.insert(note(&format!("thread1_{i}"), 1000 + i));
            }
        });

        for i in 0..50 {
            store.insert(note(&format!("thread2_{i}"), 2000 + i));
        }

        handle.join().unwrap();
        assert_eq!(store.len(), 100);
    }
}
