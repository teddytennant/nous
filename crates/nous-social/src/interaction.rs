//! Interaction aggregation: reaction counts, bookmarks, trending.
//!
//! Turns raw events into aggregated social metrics — reaction counts per post,
//! trending hashtags, and personal bookmark collections.

use std::collections::HashMap;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::event::{EventKind, SignedEvent};

// ── Interaction Counts ─────────────────────────────────────────

/// Aggregated interaction data for a single post.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct InteractionSummary {
    pub event_id: String,
    pub reaction_count: usize,
    pub repost_count: usize,
    pub reply_count: usize,
    /// Reaction breakdown: emoji → count.
    pub reactions: HashMap<String, usize>,
    /// Unique users who reacted.
    pub reactors: Vec<String>,
    /// Unique users who reposted.
    pub reposters: Vec<String>,
}

/// Aggregates interactions across multiple events.
#[derive(Debug, Default)]
pub struct InteractionIndex {
    summaries: HashMap<String, InteractionSummary>,
}

impl InteractionIndex {
    pub fn new() -> Self {
        Self::default()
    }

    /// Index a batch of events. Reactions, reposts, and replies are attributed
    /// to their target events.
    pub fn index_events(&mut self, events: &[SignedEvent]) {
        for event in events {
            match event.kind {
                EventKind::Reaction => {
                    for target in event.referenced_events() {
                        let summary = self.get_or_create(target);
                        summary.reaction_count += 1;
                        let emoji = if event.content.is_empty() {
                            "+".to_string()
                        } else {
                            event.content.clone()
                        };
                        *summary.reactions.entry(emoji).or_insert(0) += 1;
                        if !summary.reactors.contains(&event.pubkey) {
                            summary.reactors.push(event.pubkey.clone());
                        }
                    }
                }
                EventKind::Repost => {
                    for target in event.referenced_events() {
                        let summary = self.get_or_create(target);
                        summary.repost_count += 1;
                        if !summary.reposters.contains(&event.pubkey) {
                            summary.reposters.push(event.pubkey.clone());
                        }
                    }
                }
                EventKind::TextNote => {
                    // Count as reply if it references another event
                    for target in event.referenced_events() {
                        let summary = self.get_or_create(target);
                        summary.reply_count += 1;
                    }
                }
                _ => {}
            }
        }
    }

    /// Get summary for an event.
    pub fn get(&self, event_id: &str) -> Option<&InteractionSummary> {
        self.summaries.get(event_id)
    }

    /// Total interaction score (reactions + reposts + replies).
    pub fn score(&self, event_id: &str) -> usize {
        self.summaries
            .get(event_id)
            .map(|s| s.reaction_count + s.repost_count + s.reply_count)
            .unwrap_or(0)
    }

    /// Events sorted by interaction score (descending).
    pub fn top_events(&self, limit: usize) -> Vec<&InteractionSummary> {
        let mut sorted: Vec<&InteractionSummary> = self.summaries.values().collect();
        sorted.sort_by(|a, b| {
            let sa = a.reaction_count + a.repost_count + a.reply_count;
            let sb = b.reaction_count + b.repost_count + b.reply_count;
            sb.cmp(&sa)
        });
        sorted.truncate(limit);
        sorted
    }

    /// Number of indexed events.
    pub fn len(&self) -> usize {
        self.summaries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.summaries.is_empty()
    }

    fn get_or_create(&mut self, event_id: &str) -> &mut InteractionSummary {
        self.summaries
            .entry(event_id.to_string())
            .or_insert_with(|| InteractionSummary {
                event_id: event_id.to_string(),
                ..Default::default()
            })
    }
}

// ── Bookmarks ──────────────────────────────────────────────────

/// A bookmark with optional annotation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Bookmark {
    pub event_id: String,
    pub added_at: DateTime<Utc>,
    pub note: Option<String>,
    pub tags: Vec<String>,
}

/// A user's bookmark collection.
#[derive(Debug, Default, Serialize, Deserialize)]
pub struct BookmarkCollection {
    owner: String,
    bookmarks: Vec<Bookmark>,
}

impl BookmarkCollection {
    pub fn new(owner: impl Into<String>) -> Self {
        Self {
            owner: owner.into(),
            bookmarks: Vec::new(),
        }
    }

    pub fn owner(&self) -> &str {
        &self.owner
    }

    /// Add a bookmark.
    pub fn add(&mut self, event_id: &str) {
        if self.contains(event_id) {
            return;
        }
        self.bookmarks.push(Bookmark {
            event_id: event_id.to_string(),
            added_at: Utc::now(),
            note: None,
            tags: Vec::new(),
        });
    }

    /// Add a bookmark with a note and tags.
    pub fn add_with_metadata(&mut self, event_id: &str, note: &str, tags: &[&str]) {
        self.remove(event_id);
        self.bookmarks.push(Bookmark {
            event_id: event_id.to_string(),
            added_at: Utc::now(),
            note: if note.is_empty() {
                None
            } else {
                Some(note.to_string())
            },
            tags: tags.iter().map(|s| s.to_string()).collect(),
        });
    }

    /// Remove a bookmark.
    pub fn remove(&mut self, event_id: &str) -> bool {
        let before = self.bookmarks.len();
        self.bookmarks.retain(|b| b.event_id != event_id);
        self.bookmarks.len() < before
    }

    /// Check if an event is bookmarked.
    pub fn contains(&self, event_id: &str) -> bool {
        self.bookmarks.iter().any(|b| b.event_id == event_id)
    }

    /// Get all bookmarks.
    pub fn all(&self) -> &[Bookmark] {
        &self.bookmarks
    }

    /// Filter bookmarks by tag.
    pub fn by_tag(&self, tag: &str) -> Vec<&Bookmark> {
        self.bookmarks
            .iter()
            .filter(|b| b.tags.iter().any(|t| t == tag))
            .collect()
    }

    /// Number of bookmarks.
    pub fn len(&self) -> usize {
        self.bookmarks.len()
    }

    pub fn is_empty(&self) -> bool {
        self.bookmarks.is_empty()
    }
}

// ── Trending ───────────────────────────────────────────────────

/// Time-decayed trending score for a hashtag.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrendingHashtag {
    pub tag: String,
    pub count: usize,
    pub score: f64,
    pub recent_authors: Vec<String>,
}

/// Computes trending hashtags from a set of events.
pub fn compute_trending(
    events: &[SignedEvent],
    now: DateTime<Utc>,
    limit: usize,
) -> Vec<TrendingHashtag> {
    let mut tag_data: HashMap<String, (usize, f64, Vec<String>)> = HashMap::new();

    for event in events {
        if event.kind != EventKind::TextNote {
            continue;
        }
        let age_hours = (now - event.created_at).num_hours().max(0) as f64;
        // Exponential decay: half-life of 6 hours
        let weight = (-age_hours / 6.0_f64).exp();

        for hashtag in event.hashtags() {
            let tag = hashtag.to_lowercase();
            let entry = tag_data.entry(tag).or_insert((0, 0.0, Vec::new()));
            entry.0 += 1;
            entry.1 += weight;
            if !entry.2.contains(&event.pubkey) && entry.2.len() < 10 {
                entry.2.push(event.pubkey.clone());
            }
        }
    }

    let mut trending: Vec<TrendingHashtag> = tag_data
        .into_iter()
        .map(|(tag, (count, score, authors))| TrendingHashtag {
            tag,
            count,
            score,
            recent_authors: authors,
        })
        .collect();

    trending.sort_by(|a, b| {
        b.score
            .partial_cmp(&a.score)
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    trending.truncate(limit);
    trending
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::event::{EventKind, SignedEvent, Tag};
    use chrono::Duration;

    fn text_event(id: &str, author: &str, content: &str) -> SignedEvent {
        let mut event = SignedEvent::new(author, EventKind::TextNote, content, vec![]);
        event.id = id.to_string();
        event
    }

    fn reaction_event(id: &str, author: &str, target: &str, emoji: &str) -> SignedEvent {
        let mut event =
            SignedEvent::new(author, EventKind::Reaction, emoji, vec![Tag::event(target)]);
        event.id = id.to_string();
        event
    }

    fn repost_event(id: &str, author: &str, target: &str) -> SignedEvent {
        let mut event = SignedEvent::new(author, EventKind::Repost, "", vec![Tag::event(target)]);
        event.id = id.to_string();
        event
    }

    fn reply_event(id: &str, author: &str, content: &str, target: &str) -> SignedEvent {
        let mut event = SignedEvent::new(
            author,
            EventKind::TextNote,
            content,
            vec![Tag::event(target)],
        );
        event.id = id.to_string();
        event
    }

    // ── InteractionIndex ───────────────────────────────────────

    #[test]
    fn index_reactions() {
        let mut idx = InteractionIndex::new();
        idx.index_events(&[
            reaction_event("r1", "bob", "post1", "+"),
            reaction_event("r2", "carol", "post1", "❤️"),
        ]);

        let summary = idx.get("post1").unwrap();
        assert_eq!(summary.reaction_count, 2);
        assert_eq!(summary.reactions["+"], 1);
        assert_eq!(summary.reactions["❤️"], 1);
        assert_eq!(summary.reactors.len(), 2);
    }

    #[test]
    fn index_reposts() {
        let mut idx = InteractionIndex::new();
        idx.index_events(&[
            repost_event("rp1", "bob", "post1"),
            repost_event("rp2", "carol", "post1"),
        ]);

        let summary = idx.get("post1").unwrap();
        assert_eq!(summary.repost_count, 2);
        assert_eq!(summary.reposters.len(), 2);
    }

    #[test]
    fn index_replies() {
        let mut idx = InteractionIndex::new();
        idx.index_events(&[
            reply_event("reply1", "bob", "nice!", "post1"),
            reply_event("reply2", "carol", "agreed", "post1"),
        ]);

        let summary = idx.get("post1").unwrap();
        assert_eq!(summary.reply_count, 2);
    }

    #[test]
    fn score_aggregates_all_types() {
        let mut idx = InteractionIndex::new();
        idx.index_events(&[
            reaction_event("r1", "bob", "post1", "+"),
            repost_event("rp1", "carol", "post1"),
            reply_event("re1", "dave", "reply", "post1"),
        ]);

        assert_eq!(idx.score("post1"), 3);
    }

    #[test]
    fn top_events_sorted_by_score() {
        let mut idx = InteractionIndex::new();
        idx.index_events(&[
            reaction_event("r1", "bob", "post1", "+"),
            reaction_event("r2", "carol", "post2", "+"),
            reaction_event("r3", "dave", "post2", "❤️"),
        ]);

        let top = idx.top_events(2);
        assert_eq!(top[0].event_id, "post2");
        assert_eq!(top[1].event_id, "post1");
    }

    #[test]
    fn duplicate_reactor_not_counted_twice() {
        let mut idx = InteractionIndex::new();
        idx.index_events(&[
            reaction_event("r1", "bob", "post1", "+"),
            reaction_event("r2", "bob", "post1", "❤️"),
        ]);

        let summary = idx.get("post1").unwrap();
        assert_eq!(summary.reaction_count, 2);
        assert_eq!(summary.reactors.len(), 1); // same person
    }

    #[test]
    fn empty_index() {
        let idx = InteractionIndex::new();
        assert!(idx.is_empty());
        assert_eq!(idx.score("nonexistent"), 0);
    }

    // ── Bookmarks ──────────────────────────────────────────────

    #[test]
    fn add_and_check_bookmark() {
        let mut coll = BookmarkCollection::new("alice");
        coll.add("post1");
        assert!(coll.contains("post1"));
        assert_eq!(coll.len(), 1);
    }

    #[test]
    fn add_duplicate_is_noop() {
        let mut coll = BookmarkCollection::new("alice");
        coll.add("post1");
        coll.add("post1");
        assert_eq!(coll.len(), 1);
    }

    #[test]
    fn remove_bookmark() {
        let mut coll = BookmarkCollection::new("alice");
        coll.add("post1");
        assert!(coll.remove("post1"));
        assert!(!coll.contains("post1"));
        assert_eq!(coll.len(), 0);
    }

    #[test]
    fn remove_nonexistent() {
        let mut coll = BookmarkCollection::new("alice");
        assert!(!coll.remove("post1"));
    }

    #[test]
    fn bookmark_with_metadata() {
        let mut coll = BookmarkCollection::new("alice");
        coll.add_with_metadata("post1", "great thread", &["crypto", "governance"]);

        let b = &coll.all()[0];
        assert_eq!(b.note.as_deref(), Some("great thread"));
        assert_eq!(b.tags, vec!["crypto", "governance"]);
    }

    #[test]
    fn filter_by_tag() {
        let mut coll = BookmarkCollection::new("alice");
        coll.add_with_metadata("post1", "", &["crypto"]);
        coll.add_with_metadata("post2", "", &["governance"]);
        coll.add_with_metadata("post3", "", &["crypto", "governance"]);

        assert_eq!(coll.by_tag("crypto").len(), 2);
        assert_eq!(coll.by_tag("governance").len(), 2);
        assert_eq!(coll.by_tag("ai").len(), 0);
    }

    #[test]
    fn bookmark_collection_serializes() {
        let mut coll = BookmarkCollection::new("alice");
        coll.add("post1");
        coll.add_with_metadata("post2", "saved", &["tag1"]);

        let json = serde_json::to_string(&coll).unwrap();
        let restored: BookmarkCollection = serde_json::from_str(&json).unwrap();
        assert_eq!(restored.len(), 2);
        assert_eq!(restored.owner(), "alice");
    }

    // ── Trending ───────────────────────────────────────────────

    fn tagged_event(id: &str, author: &str, tags: &[&str], age_hours: i64) -> SignedEvent {
        let hashtag_tags: Vec<Tag> = tags.iter().map(|t| Tag::hashtag(t)).collect();
        let mut event = SignedEvent::new(author, EventKind::TextNote, "content", hashtag_tags);
        event.id = id.to_string();
        event.created_at = Utc::now() - Duration::hours(age_hours);
        event
    }

    #[test]
    fn trending_ranks_by_recency() {
        let now = Utc::now();
        let events = vec![
            tagged_event("e1", "alice", &["rust"], 0), // very recent
            tagged_event("e2", "bob", &["rust"], 0),
            tagged_event("e3", "carol", &["python"], 24), // old
            tagged_event("e4", "dave", &["python"], 24),
            tagged_event("e5", "eve", &["python"], 24),
        ];

        let trending = compute_trending(&events, now, 10);
        // Rust should rank higher despite fewer events due to recency
        assert_eq!(trending[0].tag, "rust");
    }

    #[test]
    fn trending_limits_results() {
        let now = Utc::now();
        let events = vec![
            tagged_event("e1", "alice", &["a"], 0),
            tagged_event("e2", "bob", &["b"], 0),
            tagged_event("e3", "carol", &["c"], 0),
        ];

        let trending = compute_trending(&events, now, 2);
        assert_eq!(trending.len(), 2);
    }

    #[test]
    fn trending_case_insensitive() {
        let now = Utc::now();
        let events = vec![
            tagged_event("e1", "alice", &["Rust"], 0),
            tagged_event("e2", "bob", &["rust"], 0),
            tagged_event("e3", "carol", &["RUST"], 0),
        ];

        let trending = compute_trending(&events, now, 10);
        assert_eq!(trending.len(), 1);
        assert_eq!(trending[0].tag, "rust");
        assert_eq!(trending[0].count, 3);
    }

    #[test]
    fn trending_tracks_authors() {
        let now = Utc::now();
        let events = vec![
            tagged_event("e1", "alice", &["rust"], 0),
            tagged_event("e2", "bob", &["rust"], 0),
        ];

        let trending = compute_trending(&events, now, 10);
        assert_eq!(trending[0].recent_authors.len(), 2);
    }

    #[test]
    fn trending_empty_events() {
        let trending = compute_trending(&[], Utc::now(), 10);
        assert!(trending.is_empty());
    }

    #[test]
    fn trending_hashtag_serializes() {
        let th = TrendingHashtag {
            tag: "rust".into(),
            count: 5,
            score: 3.14,
            recent_authors: vec!["alice".into()],
        };
        let json = serde_json::to_string(&th).unwrap();
        let restored: TrendingHashtag = serde_json::from_str(&json).unwrap();
        assert_eq!(restored.tag, "rust");
    }
}
