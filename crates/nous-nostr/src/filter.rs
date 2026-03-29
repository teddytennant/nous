use crate::event::{Event, Kind};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;

/// NIP-01 subscription filter.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct Filter {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ids: Option<Vec<String>>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub authors: Option<Vec<String>>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub kinds: Option<Vec<Kind>>,

    /// Event references (#e tag).
    #[serde(rename = "#e", skip_serializing_if = "Option::is_none")]
    pub event_refs: Option<Vec<String>>,

    /// Pubkey references (#p tag).
    #[serde(rename = "#p", skip_serializing_if = "Option::is_none")]
    pub pubkey_refs: Option<Vec<String>>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub since: Option<u64>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub until: Option<u64>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub limit: Option<usize>,
}

impl Filter {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn ids(mut self, ids: Vec<String>) -> Self {
        self.ids = Some(ids);
        self
    }

    pub fn authors(mut self, authors: Vec<String>) -> Self {
        self.authors = Some(authors);
        self
    }

    pub fn kinds(mut self, kinds: Vec<Kind>) -> Self {
        self.kinds = Some(kinds);
        self
    }

    pub fn event_refs(mut self, refs: Vec<String>) -> Self {
        self.event_refs = Some(refs);
        self
    }

    pub fn pubkey_refs(mut self, refs: Vec<String>) -> Self {
        self.pubkey_refs = Some(refs);
        self
    }

    pub fn since(mut self, ts: u64) -> Self {
        self.since = Some(ts);
        self
    }

    pub fn until(mut self, ts: u64) -> Self {
        self.until = Some(ts);
        self
    }

    pub fn limit(mut self, n: usize) -> Self {
        self.limit = Some(n);
        self
    }

    /// Check if an event matches this filter. All specified fields must match (AND logic).
    /// Prefix matching is used for ids and authors per NIP-01.
    pub fn matches(&self, event: &Event) -> bool {
        if let Some(ids) = &self.ids {
            if !ids.iter().any(|prefix| event.id.starts_with(prefix)) {
                return false;
            }
        }

        if let Some(authors) = &self.authors {
            if !authors
                .iter()
                .any(|prefix| event.pubkey.starts_with(prefix))
            {
                return false;
            }
        }

        if let Some(kinds) = &self.kinds {
            if !kinds.contains(&event.kind) {
                return false;
            }
        }

        if let Some(since) = self.since {
            if event.created_at < since {
                return false;
            }
        }

        if let Some(until) = self.until {
            if event.created_at > until {
                return false;
            }
        }

        if let Some(event_refs) = &self.event_refs {
            let e_tags: HashSet<&str> = event
                .tags
                .iter()
                .filter(|t| t.tag_name() == Some("e"))
                .filter_map(|t| t.value())
                .collect();
            if !event_refs.iter().any(|r| e_tags.contains(r.as_str())) {
                return false;
            }
        }

        if let Some(pubkey_refs) = &self.pubkey_refs {
            let p_tags: HashSet<&str> = event
                .tags
                .iter()
                .filter(|t| t.tag_name() == Some("p"))
                .filter_map(|t| t.value())
                .collect();
            if !pubkey_refs.iter().any(|r| p_tags.contains(r.as_str())) {
                return false;
            }
        }

        true
    }

    /// Filter a set of events, applying the limit if specified.
    pub fn apply<'a>(&self, events: &'a [Event]) -> Vec<&'a Event> {
        let mut matched: Vec<&Event> = events.iter().filter(|e| self.matches(e)).collect();
        // Sort by created_at descending (newest first) per NIP-01
        matched.sort_by(|a, b| b.created_at.cmp(&a.created_at));
        if let Some(limit) = self.limit {
            matched.truncate(limit);
        }
        matched
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::event::{EventBuilder, Tag};
    use ed25519_dalek::SigningKey;
    use rand::rngs::OsRng;

    fn key() -> SigningKey {
        SigningKey::generate(&mut OsRng)
    }

    fn make_event(kind: Kind, content: &str, ts: u64, sk: &SigningKey) -> Event {
        EventBuilder::new(kind, content).created_at(ts).sign(sk)
    }

    #[test]
    fn empty_filter_matches_everything() {
        let f = Filter::new();
        let k = key();
        let e = make_event(Kind::TEXT_NOTE, "hello", 1000, &k);
        assert!(f.matches(&e));
    }

    #[test]
    fn filter_by_kind() {
        let k = key();
        let note = make_event(Kind::TEXT_NOTE, "note", 1000, &k);
        let meta = make_event(Kind::METADATA, "meta", 1000, &k);

        let f = Filter::new().kinds(vec![Kind::TEXT_NOTE]);
        assert!(f.matches(&note));
        assert!(!f.matches(&meta));
    }

    #[test]
    fn filter_by_author_prefix() {
        let k1 = key();
        let k2 = key();
        let e1 = make_event(Kind::TEXT_NOTE, "a", 1000, &k1);
        let e2 = make_event(Kind::TEXT_NOTE, "b", 1000, &k2);

        let prefix = e1.pubkey[..8].to_string();
        let f = Filter::new().authors(vec![prefix]);
        assert!(f.matches(&e1));
        // Very unlikely k2 shares the same 8-char prefix
        assert!(!f.matches(&e2));
    }

    #[test]
    fn filter_by_id_prefix() {
        let k = key();
        let e = make_event(Kind::TEXT_NOTE, "test", 1000, &k);
        let prefix = e.id[..6].to_string();

        let f = Filter::new().ids(vec![prefix]);
        assert!(f.matches(&e));

        let f2 = Filter::new().ids(vec!["0000000000".to_string()]);
        // Very unlikely to match
        assert!(!f2.matches(&e) || e.id.starts_with("0000000000"));
    }

    #[test]
    fn filter_by_time_range() {
        let k = key();
        let e = make_event(Kind::TEXT_NOTE, "t", 1500, &k);

        assert!(Filter::new().since(1000).matches(&e));
        assert!(Filter::new().until(2000).matches(&e));
        assert!(Filter::new().since(1000).until(2000).matches(&e));
        assert!(!Filter::new().since(2000).matches(&e));
        assert!(!Filter::new().until(1000).matches(&e));
    }

    #[test]
    fn filter_by_event_ref_tag() {
        let k = key();
        let e = EventBuilder::text_note("reply")
            .tag(Tag::event("target_id"))
            .created_at(1000)
            .sign(&k);

        let f = Filter::new().event_refs(vec!["target_id".to_string()]);
        assert!(f.matches(&e));

        let f2 = Filter::new().event_refs(vec!["other_id".to_string()]);
        assert!(!f2.matches(&e));
    }

    #[test]
    fn filter_by_pubkey_ref_tag() {
        let k = key();
        let e = EventBuilder::text_note("mention")
            .tag(Tag::pubkey("target_pk"))
            .created_at(1000)
            .sign(&k);

        let f = Filter::new().pubkey_refs(vec!["target_pk".to_string()]);
        assert!(f.matches(&e));

        let f2 = Filter::new().pubkey_refs(vec!["other_pk".to_string()]);
        assert!(!f2.matches(&e));
    }

    #[test]
    fn combined_filters() {
        let k = key();
        let e = EventBuilder::text_note("hello")
            .tag(Tag::pubkey("alice"))
            .created_at(1500)
            .sign(&k);

        let f = Filter::new()
            .kinds(vec![Kind::TEXT_NOTE])
            .pubkey_refs(vec!["alice".to_string()])
            .since(1000)
            .until(2000);
        assert!(f.matches(&e));

        // Wrong kind
        let f2 = Filter::new()
            .kinds(vec![Kind::METADATA])
            .pubkey_refs(vec!["alice".to_string()]);
        assert!(!f2.matches(&e));
    }

    #[test]
    fn apply_with_limit() {
        let k = key();
        let events: Vec<Event> = (0..10)
            .map(|i| make_event(Kind::TEXT_NOTE, &format!("msg {i}"), 1000 + i, &k))
            .collect();

        let f = Filter::new().limit(3);
        let result = f.apply(&events);
        assert_eq!(result.len(), 3);
        // Should be newest first
        assert!(result[0].created_at >= result[1].created_at);
        assert!(result[1].created_at >= result[2].created_at);
    }

    #[test]
    fn apply_returns_newest_first() {
        let k = key();
        let events: Vec<Event> = (0..5)
            .map(|i| make_event(Kind::TEXT_NOTE, &format!("msg"), 1000 + i, &k))
            .collect();

        let f = Filter::new();
        let result = f.apply(&events);
        for w in result.windows(2) {
            assert!(w[0].created_at >= w[1].created_at);
        }
    }

    #[test]
    fn filter_serialization_roundtrip() {
        let f = Filter::new()
            .kinds(vec![Kind::TEXT_NOTE, Kind::METADATA])
            .authors(vec!["abc".into()])
            .since(1000)
            .limit(50);

        let json = serde_json::to_string(&f).unwrap();
        let deserialized: Filter = serde_json::from_str(&json).unwrap();

        assert_eq!(deserialized.kinds.unwrap(), vec![Kind::TEXT_NOTE, Kind::METADATA]);
        assert_eq!(deserialized.authors.unwrap(), vec!["abc".to_string()]);
        assert_eq!(deserialized.since.unwrap(), 1000);
        assert_eq!(deserialized.limit.unwrap(), 50);
    }

    #[test]
    fn filter_skip_serializing_none_fields() {
        let f = Filter::new().kinds(vec![Kind::TEXT_NOTE]);
        let json = serde_json::to_string(&f).unwrap();
        assert!(!json.contains("ids"));
        assert!(!json.contains("authors"));
        assert!(!json.contains("since"));
        assert!(!json.contains("until"));
        assert!(!json.contains("limit"));
        assert!(json.contains("kinds"));
    }
}
