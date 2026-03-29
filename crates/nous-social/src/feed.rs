use std::collections::{BTreeSet, HashMap, HashSet};

use crate::event::{EventKind, SignedEvent};

pub struct Feed {
    events: BTreeSet<SignedEvent>,
    by_author: HashMap<String, Vec<String>>,
    by_kind: HashMap<u32, Vec<String>>,
}

impl Feed {
    pub fn new() -> Self {
        Self {
            events: BTreeSet::new(),
            by_author: HashMap::new(),
            by_kind: HashMap::new(),
        }
    }

    pub fn insert(&mut self, event: SignedEvent) -> bool {
        let id = event.id.clone();
        let author = event.pubkey.clone();
        let kind = event.kind.as_u32();

        if self.events.insert(event) {
            self.by_author
                .entry(author)
                .or_default()
                .push(id.clone());
            self.by_kind.entry(kind).or_default().push(id);
            true
        } else {
            false
        }
    }

    pub fn len(&self) -> usize {
        self.events.len()
    }

    pub fn is_empty(&self) -> bool {
        self.events.is_empty()
    }

    pub fn latest(&self, limit: usize) -> Vec<&SignedEvent> {
        self.events.iter().take(limit).collect()
    }

    pub fn by_author(&self, pubkey: &str) -> Vec<&SignedEvent> {
        let ids: HashSet<&str> = self
            .by_author
            .get(pubkey)
            .map(|ids| ids.iter().map(|s| s.as_str()).collect())
            .unwrap_or_default();

        self.events
            .iter()
            .filter(|e| ids.contains(e.id.as_str()))
            .collect()
    }

    pub fn by_kind(&self, kind: EventKind) -> Vec<&SignedEvent> {
        let ids: HashSet<&str> = self
            .by_kind
            .get(&kind.as_u32())
            .map(|ids| ids.iter().map(|s| s.as_str()).collect())
            .unwrap_or_default();

        self.events
            .iter()
            .filter(|e| ids.contains(e.id.as_str()))
            .collect()
    }

    pub fn text_notes(&self) -> Vec<&SignedEvent> {
        self.by_kind(EventKind::TextNote)
    }

    pub fn timeline(&self, following: &[String], limit: usize) -> Vec<&SignedEvent> {
        let following_set: HashSet<&str> = following.iter().map(|s| s.as_str()).collect();
        self.events
            .iter()
            .filter(|e| following_set.contains(e.pubkey.as_str()))
            .take(limit)
            .collect()
    }

    pub fn replies_to(&self, event_id: &str) -> Vec<&SignedEvent> {
        self.events
            .iter()
            .filter(|e| e.referenced_events().contains(&event_id))
            .collect()
    }

    pub fn by_hashtag(&self, tag: &str) -> Vec<&SignedEvent> {
        self.events
            .iter()
            .filter(|e| e.hashtags().contains(&tag))
            .collect()
    }

    pub fn remove(&mut self, event_id: &str) -> bool {
        let event = self.events.iter().find(|e| e.id == event_id).cloned();
        if let Some(event) = event {
            self.events.remove(&event);
            if let Some(ids) = self.by_author.get_mut(&event.pubkey) {
                ids.retain(|id| id != event_id);
            }
            if let Some(ids) = self.by_kind.get_mut(&event.kind.as_u32()) {
                ids.retain(|id| id != event_id);
            }
            true
        } else {
            false
        }
    }
}

impl Default for Feed {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::event::{EventKind, SignedEvent, Tag};

    fn make_event(pubkey: &str, content: &str) -> SignedEvent {
        SignedEvent::new(pubkey, EventKind::TextNote, content, vec![])
    }

    fn make_event_with_tags(pubkey: &str, content: &str, tags: Vec<Tag>) -> SignedEvent {
        SignedEvent::new(pubkey, EventKind::TextNote, content, tags)
    }

    #[test]
    fn empty_feed() {
        let feed = Feed::new();
        assert!(feed.is_empty());
        assert_eq!(feed.len(), 0);
    }

    #[test]
    fn insert_and_retrieve() {
        let mut feed = Feed::new();
        let event = make_event("did:key:z1", "hello");
        assert!(feed.insert(event));
        assert_eq!(feed.len(), 1);
    }

    #[test]
    fn deduplicates_same_event() {
        let mut feed = Feed::new();
        let event = make_event("did:key:z1", "hello");
        let clone = event.clone();
        assert!(feed.insert(event));
        assert!(!feed.insert(clone));
        assert_eq!(feed.len(), 1);
    }

    #[test]
    fn latest_returns_newest_first() {
        let mut feed = Feed::new();
        let e1 = make_event("did:key:z1", "first");
        std::thread::sleep(std::time::Duration::from_millis(10));
        let e2 = make_event("did:key:z1", "second");
        feed.insert(e1);
        feed.insert(e2);

        let latest = feed.latest(2);
        assert_eq!(latest[0].content, "second");
        assert_eq!(latest[1].content, "first");
    }

    #[test]
    fn latest_respects_limit() {
        let mut feed = Feed::new();
        for i in 0..10 {
            feed.insert(make_event("did:key:z1", &format!("post {i}")));
        }
        assert_eq!(feed.latest(3).len(), 3);
    }

    #[test]
    fn filter_by_author() {
        let mut feed = Feed::new();
        feed.insert(make_event("did:key:alice", "alice post"));
        feed.insert(make_event("did:key:bob", "bob post"));
        feed.insert(make_event("did:key:alice", "alice again"));

        let alice_posts = feed.by_author("did:key:alice");
        assert_eq!(alice_posts.len(), 2);
        assert!(alice_posts.iter().all(|e| e.pubkey == "did:key:alice"));
    }

    #[test]
    fn filter_by_kind() {
        let mut feed = Feed::new();
        feed.insert(SignedEvent::new(
            "did:key:z1",
            EventKind::TextNote,
            "note",
            vec![],
        ));
        feed.insert(SignedEvent::new(
            "did:key:z1",
            EventKind::Reaction,
            "+",
            vec![],
        ));

        assert_eq!(feed.text_notes().len(), 1);
        assert_eq!(feed.by_kind(EventKind::Reaction).len(), 1);
    }

    #[test]
    fn timeline_filters_by_following() {
        let mut feed = Feed::new();
        feed.insert(make_event("did:key:alice", "alice post"));
        feed.insert(make_event("did:key:bob", "bob post"));
        feed.insert(make_event("did:key:carol", "carol post"));

        let following = vec!["did:key:alice".to_string(), "did:key:carol".to_string()];
        let timeline = feed.timeline(&following, 10);
        assert_eq!(timeline.len(), 2);
    }

    #[test]
    fn replies_to_event() {
        let mut feed = Feed::new();
        let parent = make_event("did:key:z1", "parent");
        let parent_id = parent.id.clone();
        let reply = make_event_with_tags(
            "did:key:z2",
            "reply",
            vec![Tag::event(&parent_id)],
        );
        feed.insert(parent);
        feed.insert(reply);

        let replies = feed.replies_to(&parent_id);
        assert_eq!(replies.len(), 1);
        assert_eq!(replies[0].content, "reply");
    }

    #[test]
    fn filter_by_hashtag() {
        let mut feed = Feed::new();
        feed.insert(make_event_with_tags(
            "did:key:z1",
            "tagged",
            vec![Tag::hashtag("nous")],
        ));
        feed.insert(make_event("did:key:z1", "untagged"));

        let tagged = feed.by_hashtag("nous");
        assert_eq!(tagged.len(), 1);
        assert_eq!(tagged[0].content, "tagged");
    }

    #[test]
    fn remove_event() {
        let mut feed = Feed::new();
        let event = make_event("did:key:z1", "to remove");
        let id = event.id.clone();
        feed.insert(event);
        assert_eq!(feed.len(), 1);

        assert!(feed.remove(&id));
        assert_eq!(feed.len(), 0);
        assert!(!feed.remove(&id)); // already removed
    }
}
