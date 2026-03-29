use crate::event::Event;
use crate::filter::Filter;
use std::collections::HashMap;
use std::sync::{Arc, RwLock};

/// A subscription is an ID with one or more filters.
#[derive(Debug, Clone)]
pub struct Subscription {
    pub id: String,
    pub filters: Vec<Filter>,
}

impl Subscription {
    pub fn new(id: impl Into<String>, filters: Vec<Filter>) -> Self {
        Self {
            id: id.into(),
            filters,
        }
    }

    /// Check if an event matches any of this subscription's filters.
    pub fn matches(&self, event: &Event) -> bool {
        self.filters.iter().any(|f| f.matches(event))
    }
}

/// Manages subscriptions for a single client connection.
#[derive(Debug, Clone)]
pub struct SubscriptionManager {
    subscriptions: Arc<RwLock<HashMap<String, Subscription>>>,
    max_subscriptions: usize,
}

impl SubscriptionManager {
    pub fn new(max_subscriptions: usize) -> Self {
        Self {
            subscriptions: Arc::new(RwLock::new(HashMap::new())),
            max_subscriptions,
        }
    }

    /// Add or replace a subscription. Returns error if at capacity and this is a new subscription.
    pub fn add(&self, sub: Subscription) -> Result<(), SubscriptionError> {
        let mut subs = self.subscriptions.write().unwrap();
        if !subs.contains_key(&sub.id) && subs.len() >= self.max_subscriptions {
            return Err(SubscriptionError::TooManySubscriptions {
                max: self.max_subscriptions,
            });
        }
        subs.insert(sub.id.clone(), sub);
        Ok(())
    }

    /// Remove a subscription by ID. Returns whether it existed.
    pub fn remove(&self, id: &str) -> bool {
        self.subscriptions.write().unwrap().remove(id).is_some()
    }

    /// Get all subscription IDs that match an event.
    pub fn matching_subscriptions(&self, event: &Event) -> Vec<String> {
        let subs = self.subscriptions.read().unwrap();
        subs.values()
            .filter(|s| s.matches(event))
            .map(|s| s.id.clone())
            .collect()
    }

    /// Get a subscription by ID.
    pub fn get(&self, id: &str) -> Option<Subscription> {
        self.subscriptions.read().unwrap().get(id).cloned()
    }

    /// Number of active subscriptions.
    pub fn len(&self) -> usize {
        self.subscriptions.read().unwrap().len()
    }

    /// Whether there are no active subscriptions.
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Get all subscription IDs.
    pub fn subscription_ids(&self) -> Vec<String> {
        self.subscriptions.read().unwrap().keys().cloned().collect()
    }

    /// Clear all subscriptions.
    pub fn clear(&self) {
        self.subscriptions.write().unwrap().clear();
    }
}

#[derive(Debug, thiserror::Error)]
pub enum SubscriptionError {
    #[error("too many subscriptions (max {max})")]
    TooManySubscriptions { max: usize },
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::event::{EventBuilder, Kind};
    use ed25519_dalek::SigningKey;
    use rand::rngs::OsRng;

    fn key() -> SigningKey {
        SigningKey::generate(&mut OsRng)
    }

    #[test]
    fn subscription_matches_event() {
        let sub = Subscription::new("test", vec![Filter::new().kinds(vec![Kind::TEXT_NOTE])]);
        let k = key();
        let note = EventBuilder::text_note("hi").created_at(1000).sign(&k);
        let meta = EventBuilder::metadata("{}").created_at(1000).sign(&k);

        assert!(sub.matches(&note));
        assert!(!sub.matches(&meta));
    }

    #[test]
    fn subscription_multiple_filters_or_logic() {
        let sub = Subscription::new(
            "multi",
            vec![
                Filter::new().kinds(vec![Kind::TEXT_NOTE]),
                Filter::new().kinds(vec![Kind::METADATA]),
            ],
        );
        let k = key();
        let note = EventBuilder::text_note("hi").created_at(1000).sign(&k);
        let meta = EventBuilder::metadata("{}").created_at(1000).sign(&k);
        let contacts = EventBuilder::new(Kind::CONTACTS, "[]")
            .created_at(1000)
            .sign(&k);

        assert!(sub.matches(&note));
        assert!(sub.matches(&meta));
        assert!(!sub.matches(&contacts));
    }

    #[test]
    fn manager_add_and_get() {
        let mgr = SubscriptionManager::new(10);
        let sub = Subscription::new("s1", vec![Filter::new()]);
        mgr.add(sub).unwrap();

        assert_eq!(mgr.len(), 1);
        assert!(mgr.get("s1").is_some());
        assert!(mgr.get("s2").is_none());
    }

    #[test]
    fn manager_replace_subscription() {
        let mgr = SubscriptionManager::new(10);
        mgr.add(Subscription::new(
            "s1",
            vec![Filter::new().kinds(vec![Kind::TEXT_NOTE])],
        ))
        .unwrap();
        mgr.add(Subscription::new(
            "s1",
            vec![Filter::new().kinds(vec![Kind::METADATA])],
        ))
        .unwrap();

        assert_eq!(mgr.len(), 1);
        let sub = mgr.get("s1").unwrap();
        assert_eq!(sub.filters[0].kinds.as_ref().unwrap(), &[Kind::METADATA]);
    }

    #[test]
    fn manager_max_subscriptions() {
        let mgr = SubscriptionManager::new(2);
        mgr.add(Subscription::new("s1", vec![Filter::new()]))
            .unwrap();
        mgr.add(Subscription::new("s2", vec![Filter::new()]))
            .unwrap();

        let result = mgr.add(Subscription::new("s3", vec![Filter::new()]));
        assert!(result.is_err());
        assert_eq!(mgr.len(), 2);
    }

    #[test]
    fn manager_remove() {
        let mgr = SubscriptionManager::new(10);
        mgr.add(Subscription::new("s1", vec![Filter::new()]))
            .unwrap();

        assert!(mgr.remove("s1"));
        assert!(!mgr.remove("s1"));
        assert!(mgr.is_empty());
    }

    #[test]
    fn manager_matching_subscriptions() {
        let mgr = SubscriptionManager::new(10);
        let k = key();

        mgr.add(Subscription::new(
            "notes",
            vec![Filter::new().kinds(vec![Kind::TEXT_NOTE])],
        ))
        .unwrap();
        mgr.add(Subscription::new(
            "meta",
            vec![Filter::new().kinds(vec![Kind::METADATA])],
        ))
        .unwrap();
        mgr.add(Subscription::new("all", vec![Filter::new()]))
            .unwrap();

        let note = EventBuilder::text_note("hi").created_at(1000).sign(&k);
        let matching = mgr.matching_subscriptions(&note);

        assert!(matching.contains(&"notes".to_string()));
        assert!(matching.contains(&"all".to_string()));
        assert!(!matching.contains(&"meta".to_string()));
    }

    #[test]
    fn manager_subscription_ids() {
        let mgr = SubscriptionManager::new(10);
        mgr.add(Subscription::new("a", vec![Filter::new()]))
            .unwrap();
        mgr.add(Subscription::new("b", vec![Filter::new()]))
            .unwrap();

        let mut ids = mgr.subscription_ids();
        ids.sort();
        assert_eq!(ids, vec!["a", "b"]);
    }

    #[test]
    fn manager_clear() {
        let mgr = SubscriptionManager::new(10);
        mgr.add(Subscription::new("a", vec![Filter::new()]))
            .unwrap();
        mgr.add(Subscription::new("b", vec![Filter::new()]))
            .unwrap();
        mgr.clear();
        assert!(mgr.is_empty());
    }
}
