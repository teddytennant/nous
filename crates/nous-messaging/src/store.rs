//! In-memory message store with ordering, pagination, search, and editing.
//!
//! Provides a per-channel message history that supports:
//! - Chronological ordering by timestamp
//! - Cursor-based pagination (before/after a message ID)
//! - Full-text search within a channel
//! - Message editing and soft deletion
//! - Pin/unpin messages

use std::collections::{BTreeMap, HashMap, HashSet};

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use nous_core::{Error, Result};

/// A stored message entry with edit/delete metadata.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StoredMessage {
    pub id: String,
    pub channel_id: String,
    pub sender_did: String,
    pub body: String,
    pub timestamp: DateTime<Utc>,
    pub reply_to: Option<String>,
    pub edited_at: Option<DateTime<Utc>>,
    pub deleted: bool,
    pub pinned: bool,
}

/// Pagination cursor: fetch messages before or after a given message ID.
#[derive(Debug, Clone)]
pub enum Cursor {
    Before(String),
    After(String),
}

/// A page of messages returned by a query.
#[derive(Debug)]
pub struct MessagePage {
    pub messages: Vec<StoredMessage>,
    pub has_more: bool,
}

/// In-memory message store indexed by channel.
pub struct MessageStore {
    /// channel_id -> ordered message IDs (by timestamp, ties broken by ID)
    index: HashMap<String, BTreeMap<(DateTime<Utc>, String), ()>>,
    /// message_id -> stored message
    messages: HashMap<String, StoredMessage>,
    /// channel_id -> set of pinned message IDs
    pins: HashMap<String, HashSet<String>>,
}

impl MessageStore {
    pub fn new() -> Self {
        Self {
            index: HashMap::new(),
            messages: HashMap::new(),
            pins: HashMap::new(),
        }
    }

    /// Insert a new message into the store.
    pub fn insert(&mut self, msg: StoredMessage) -> Result<()> {
        if self.messages.contains_key(&msg.id) {
            return Err(Error::InvalidInput(format!(
                "message {} already exists",
                msg.id
            )));
        }

        let key = (msg.timestamp, msg.id.clone());
        self.index
            .entry(msg.channel_id.clone())
            .or_default()
            .insert(key, ());
        self.messages.insert(msg.id.clone(), msg);

        Ok(())
    }

    /// Get a message by ID.
    pub fn get(&self, message_id: &str) -> Option<&StoredMessage> {
        self.messages.get(message_id)
    }

    /// Edit a message's body. Only the sender can edit.
    pub fn edit(&mut self, message_id: &str, sender_did: &str, new_body: &str) -> Result<()> {
        let msg = self
            .messages
            .get_mut(message_id)
            .ok_or_else(|| Error::NotFound("message not found".into()))?;

        if msg.sender_did != sender_did {
            return Err(Error::PermissionDenied("only the sender can edit".into()));
        }

        if msg.deleted {
            return Err(Error::InvalidInput("cannot edit a deleted message".into()));
        }

        msg.body = new_body.to_string();
        msg.edited_at = Some(Utc::now());
        Ok(())
    }

    /// Soft-delete a message. Only the sender can delete.
    pub fn delete(&mut self, message_id: &str, sender_did: &str) -> Result<()> {
        let msg = self
            .messages
            .get_mut(message_id)
            .ok_or_else(|| Error::NotFound("message not found".into()))?;

        if msg.sender_did != sender_did {
            return Err(Error::PermissionDenied("only the sender can delete".into()));
        }

        msg.deleted = true;
        msg.body.clear();
        Ok(())
    }

    /// Pin a message in its channel.
    pub fn pin(&mut self, message_id: &str) -> Result<()> {
        let msg = self
            .messages
            .get_mut(message_id)
            .ok_or_else(|| Error::NotFound("message not found".into()))?;

        msg.pinned = true;
        self.pins
            .entry(msg.channel_id.clone())
            .or_default()
            .insert(message_id.to_string());
        Ok(())
    }

    /// Unpin a message.
    pub fn unpin(&mut self, message_id: &str) -> Result<()> {
        let msg = self
            .messages
            .get_mut(message_id)
            .ok_or_else(|| Error::NotFound("message not found".into()))?;

        msg.pinned = false;
        if let Some(pins) = self.pins.get_mut(&msg.channel_id) {
            pins.remove(message_id);
        }
        Ok(())
    }

    /// Get all pinned messages for a channel, ordered by timestamp.
    pub fn pinned(&self, channel_id: &str) -> Vec<&StoredMessage> {
        let Some(pin_ids) = self.pins.get(channel_id) else {
            return Vec::new();
        };

        let mut msgs: Vec<&StoredMessage> = pin_ids
            .iter()
            .filter_map(|id| self.messages.get(id))
            .collect();
        msgs.sort_by_key(|m| m.timestamp);
        msgs
    }

    /// Fetch a page of messages from a channel.
    /// Returns up to `limit` messages, optionally starting from a cursor position.
    pub fn fetch(&self, channel_id: &str, limit: usize, cursor: Option<&Cursor>) -> MessagePage {
        let Some(channel_idx) = self.index.get(channel_id) else {
            return MessagePage {
                messages: Vec::new(),
                has_more: false,
            };
        };

        match cursor {
            None => {
                // Latest messages (most recent first, but return in chronological order)
                let entries: Vec<_> = channel_idx.iter().rev().take(limit + 1).collect();
                let has_more = entries.len() > limit;
                let mut messages: Vec<StoredMessage> = entries
                    .into_iter()
                    .take(limit)
                    .filter_map(|((_, id), _)| self.messages.get(id).cloned())
                    .collect();
                messages.reverse(); // chronological order
                MessagePage { messages, has_more }
            }
            Some(Cursor::Before(msg_id)) => {
                let Some(anchor) = self.messages.get(msg_id) else {
                    return MessagePage {
                        messages: Vec::new(),
                        has_more: false,
                    };
                };
                let anchor_key = (anchor.timestamp, msg_id.clone());

                let entries: Vec<_> = channel_idx
                    .range(..&anchor_key)
                    .rev()
                    .take(limit + 1)
                    .collect();
                let has_more = entries.len() > limit;
                let mut messages: Vec<StoredMessage> = entries
                    .into_iter()
                    .take(limit)
                    .filter_map(|((_, id), _)| self.messages.get(id).cloned())
                    .collect();
                messages.reverse();
                MessagePage { messages, has_more }
            }
            Some(Cursor::After(msg_id)) => {
                let Some(anchor) = self.messages.get(msg_id) else {
                    return MessagePage {
                        messages: Vec::new(),
                        has_more: false,
                    };
                };
                let anchor_key = (anchor.timestamp, msg_id.clone());

                let entries: Vec<_> = channel_idx
                    .range(&anchor_key..)
                    .skip(1) // skip the anchor itself
                    .take(limit + 1)
                    .collect();
                let has_more = entries.len() > limit;
                let messages: Vec<StoredMessage> = entries
                    .into_iter()
                    .take(limit)
                    .filter_map(|((_, id), _)| self.messages.get(id).cloned())
                    .collect();
                MessagePage { messages, has_more }
            }
        }
    }

    /// Search messages in a channel by substring (case-insensitive).
    /// Only searches non-deleted messages. Returns results in chronological order.
    pub fn search(&self, channel_id: &str, query: &str, limit: usize) -> Vec<&StoredMessage> {
        let Some(channel_idx) = self.index.get(channel_id) else {
            return Vec::new();
        };

        let query_lower = query.to_lowercase();

        channel_idx
            .iter()
            .filter_map(|((_, id), _)| self.messages.get(id))
            .filter(|m| !m.deleted && m.body.to_lowercase().contains(&query_lower))
            .take(limit)
            .collect()
    }

    /// Count messages in a channel (excluding deleted).
    pub fn count(&self, channel_id: &str) -> usize {
        let Some(channel_idx) = self.index.get(channel_id) else {
            return 0;
        };

        channel_idx
            .keys()
            .filter(|(_, id)| self.messages.get(id).is_some_and(|m| !m.deleted))
            .count()
    }

    /// Total messages stored across all channels.
    pub fn total(&self) -> usize {
        self.messages.len()
    }

    /// Get all replies to a specific message.
    pub fn replies_to(&self, message_id: &str) -> Vec<&StoredMessage> {
        let Some(msg) = self.messages.get(message_id) else {
            return Vec::new();
        };

        let Some(channel_idx) = self.index.get(&msg.channel_id) else {
            return Vec::new();
        };

        channel_idx
            .keys()
            .filter_map(|(_, id)| self.messages.get(id))
            .filter(|m| m.reply_to.as_deref() == Some(message_id) && !m.deleted)
            .collect()
    }
}

impl Default for MessageStore {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Duration;

    fn msg(id: &str, channel: &str, sender: &str, body: &str, offset_secs: i64) -> StoredMessage {
        StoredMessage {
            id: id.into(),
            channel_id: channel.into(),
            sender_did: sender.into(),
            body: body.into(),
            timestamp: Utc::now() + Duration::seconds(offset_secs),
            reply_to: None,
            edited_at: None,
            deleted: false,
            pinned: false,
        }
    }

    fn reply(
        id: &str,
        channel: &str,
        sender: &str,
        body: &str,
        reply_to: &str,
        offset_secs: i64,
    ) -> StoredMessage {
        let mut m = msg(id, channel, sender, body, offset_secs);
        m.reply_to = Some(reply_to.into());
        m
    }

    #[test]
    fn insert_and_get() {
        let mut store = MessageStore::new();
        store.insert(msg("m1", "ch1", "alice", "hello", 0)).unwrap();

        let m = store.get("m1").unwrap();
        assert_eq!(m.body, "hello");
        assert_eq!(m.sender_did, "alice");
    }

    #[test]
    fn duplicate_insert_fails() {
        let mut store = MessageStore::new();
        store.insert(msg("m1", "ch1", "alice", "hello", 0)).unwrap();
        assert!(store.insert(msg("m1", "ch1", "alice", "dupe", 1)).is_err());
    }

    #[test]
    fn edit_message() {
        let mut store = MessageStore::new();
        store
            .insert(msg("m1", "ch1", "alice", "original", 0))
            .unwrap();

        store.edit("m1", "alice", "edited").unwrap();
        let m = store.get("m1").unwrap();
        assert_eq!(m.body, "edited");
        assert!(m.edited_at.is_some());
    }

    #[test]
    fn edit_by_non_sender_fails() {
        let mut store = MessageStore::new();
        store.insert(msg("m1", "ch1", "alice", "hello", 0)).unwrap();
        assert!(store.edit("m1", "bob", "hacked").is_err());
    }

    #[test]
    fn edit_deleted_message_fails() {
        let mut store = MessageStore::new();
        store.insert(msg("m1", "ch1", "alice", "hello", 0)).unwrap();
        store.delete("m1", "alice").unwrap();
        assert!(store.edit("m1", "alice", "too late").is_err());
    }

    #[test]
    fn edit_nonexistent_fails() {
        let mut store = MessageStore::new();
        assert!(store.edit("nope", "alice", "fail").is_err());
    }

    #[test]
    fn delete_message() {
        let mut store = MessageStore::new();
        store
            .insert(msg("m1", "ch1", "alice", "secret", 0))
            .unwrap();

        store.delete("m1", "alice").unwrap();
        let m = store.get("m1").unwrap();
        assert!(m.deleted);
        assert!(m.body.is_empty());
    }

    #[test]
    fn delete_by_non_sender_fails() {
        let mut store = MessageStore::new();
        store.insert(msg("m1", "ch1", "alice", "hello", 0)).unwrap();
        assert!(store.delete("m1", "bob").is_err());
    }

    #[test]
    fn fetch_latest() {
        let mut store = MessageStore::new();
        for i in 0..5 {
            store
                .insert(msg(
                    &format!("m{i}"),
                    "ch1",
                    "alice",
                    &format!("msg {i}"),
                    i,
                ))
                .unwrap();
        }

        let page = store.fetch("ch1", 3, None);
        assert_eq!(page.messages.len(), 3);
        assert!(page.has_more);
        // Should be the 3 most recent in chronological order
        assert_eq!(page.messages[0].id, "m2");
        assert_eq!(page.messages[1].id, "m3");
        assert_eq!(page.messages[2].id, "m4");
    }

    #[test]
    fn fetch_all() {
        let mut store = MessageStore::new();
        for i in 0..3 {
            store
                .insert(msg(
                    &format!("m{i}"),
                    "ch1",
                    "alice",
                    &format!("msg {i}"),
                    i,
                ))
                .unwrap();
        }

        let page = store.fetch("ch1", 10, None);
        assert_eq!(page.messages.len(), 3);
        assert!(!page.has_more);
    }

    #[test]
    fn fetch_with_cursor_before() {
        let mut store = MessageStore::new();
        for i in 0..5 {
            store
                .insert(msg(
                    &format!("m{i}"),
                    "ch1",
                    "alice",
                    &format!("msg {i}"),
                    i,
                ))
                .unwrap();
        }

        let page = store.fetch("ch1", 2, Some(&Cursor::Before("m3".into())));
        assert_eq!(page.messages.len(), 2);
        assert_eq!(page.messages[0].id, "m1");
        assert_eq!(page.messages[1].id, "m2");
        assert!(page.has_more); // m0 still exists before
    }

    #[test]
    fn fetch_with_cursor_after() {
        let mut store = MessageStore::new();
        for i in 0..5 {
            store
                .insert(msg(
                    &format!("m{i}"),
                    "ch1",
                    "alice",
                    &format!("msg {i}"),
                    i,
                ))
                .unwrap();
        }

        let page = store.fetch("ch1", 2, Some(&Cursor::After("m1".into())));
        assert_eq!(page.messages.len(), 2);
        assert_eq!(page.messages[0].id, "m2");
        assert_eq!(page.messages[1].id, "m3");
        assert!(page.has_more);
    }

    #[test]
    fn fetch_empty_channel() {
        let store = MessageStore::new();
        let page = store.fetch("nonexistent", 10, None);
        assert!(page.messages.is_empty());
        assert!(!page.has_more);
    }

    #[test]
    fn search_messages() {
        let mut store = MessageStore::new();
        store
            .insert(msg("m1", "ch1", "alice", "hello world", 0))
            .unwrap();
        store
            .insert(msg("m2", "ch1", "bob", "goodbye world", 1))
            .unwrap();
        store
            .insert(msg("m3", "ch1", "carol", "hello there", 2))
            .unwrap();

        let results = store.search("ch1", "hello", 10);
        assert_eq!(results.len(), 2);
        assert_eq!(results[0].id, "m1");
        assert_eq!(results[1].id, "m3");
    }

    #[test]
    fn search_case_insensitive() {
        let mut store = MessageStore::new();
        store
            .insert(msg("m1", "ch1", "alice", "Hello World", 0))
            .unwrap();

        let results = store.search("ch1", "hello", 10);
        assert_eq!(results.len(), 1);
    }

    #[test]
    fn search_excludes_deleted() {
        let mut store = MessageStore::new();
        store
            .insert(msg("m1", "ch1", "alice", "secret hello", 0))
            .unwrap();
        store.delete("m1", "alice").unwrap();

        let results = store.search("ch1", "hello", 10);
        assert!(results.is_empty());
    }

    #[test]
    fn search_respects_limit() {
        let mut store = MessageStore::new();
        for i in 0..10 {
            store
                .insert(msg(
                    &format!("m{i}"),
                    "ch1",
                    "alice",
                    &format!("match {i}"),
                    i,
                ))
                .unwrap();
        }

        let results = store.search("ch1", "match", 3);
        assert_eq!(results.len(), 3);
    }

    #[test]
    fn count_messages() {
        let mut store = MessageStore::new();
        store.insert(msg("m1", "ch1", "alice", "hello", 0)).unwrap();
        store.insert(msg("m2", "ch1", "bob", "world", 1)).unwrap();
        store
            .insert(msg("m3", "ch1", "carol", "deleted", 2))
            .unwrap();
        store.delete("m3", "carol").unwrap();

        assert_eq!(store.count("ch1"), 2);
        assert_eq!(store.total(), 3); // total includes deleted
    }

    #[test]
    fn pin_and_unpin() {
        let mut store = MessageStore::new();
        store
            .insert(msg("m1", "ch1", "alice", "important", 0))
            .unwrap();
        store
            .insert(msg("m2", "ch1", "bob", "also important", 1))
            .unwrap();
        store
            .insert(msg("m3", "ch1", "carol", "not important", 2))
            .unwrap();

        store.pin("m1").unwrap();
        store.pin("m2").unwrap();

        let pinned = store.pinned("ch1");
        assert_eq!(pinned.len(), 2);
        assert!(store.get("m1").unwrap().pinned);

        store.unpin("m1").unwrap();
        let pinned = store.pinned("ch1");
        assert_eq!(pinned.len(), 1);
        assert_eq!(pinned[0].id, "m2");
    }

    #[test]
    fn pin_nonexistent_fails() {
        let mut store = MessageStore::new();
        assert!(store.pin("nope").is_err());
    }

    #[test]
    fn replies_to_message() {
        let mut store = MessageStore::new();
        store
            .insert(msg("m1", "ch1", "alice", "question?", 0))
            .unwrap();
        store
            .insert(reply("m2", "ch1", "bob", "answer 1", "m1", 1))
            .unwrap();
        store
            .insert(reply("m3", "ch1", "carol", "answer 2", "m1", 2))
            .unwrap();
        store
            .insert(msg("m4", "ch1", "dave", "unrelated", 3))
            .unwrap();

        let replies = store.replies_to("m1");
        assert_eq!(replies.len(), 2);
    }

    #[test]
    fn replies_excludes_deleted() {
        let mut store = MessageStore::new();
        store
            .insert(msg("m1", "ch1", "alice", "question?", 0))
            .unwrap();
        store
            .insert(reply("m2", "ch1", "bob", "answer", "m1", 1))
            .unwrap();
        store.delete("m2", "bob").unwrap();

        let replies = store.replies_to("m1");
        assert!(replies.is_empty());
    }

    #[test]
    fn channels_isolated() {
        let mut store = MessageStore::new();
        store
            .insert(msg("m1", "ch1", "alice", "in ch1", 0))
            .unwrap();
        store.insert(msg("m2", "ch2", "bob", "in ch2", 1)).unwrap();

        assert_eq!(store.count("ch1"), 1);
        assert_eq!(store.count("ch2"), 1);
        assert_eq!(store.search("ch1", "ch2", 10).len(), 0);
    }

    #[test]
    fn message_serializes() {
        let m = msg("m1", "ch1", "alice", "hello", 0);
        let json = serde_json::to_string(&m).unwrap();
        let restored: StoredMessage = serde_json::from_str(&json).unwrap();
        assert_eq!(restored.id, "m1");
        assert_eq!(restored.body, "hello");
    }

    #[test]
    fn pinned_empty_channel() {
        let store = MessageStore::new();
        assert!(store.pinned("nonexistent").is_empty());
    }
}
