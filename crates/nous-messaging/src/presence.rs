//! Presence signals: typing indicators, read receipts, and online status.
//!
//! These lightweight signals are transmitted alongside messages to provide
//! real-time feedback in chat UIs. They are ephemeral and not stored
//! permanently — only the latest state matters.

use std::collections::{HashMap, HashSet};

use chrono::{DateTime, Duration, Utc};
use serde::{Deserialize, Serialize};

/// A typing indicator signal.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TypingIndicator {
    pub channel_id: String,
    pub user_did: String,
    pub started_at: DateTime<Utc>,
}

impl TypingIndicator {
    pub fn new(channel_id: &str, user_did: &str) -> Self {
        Self {
            channel_id: channel_id.into(),
            user_did: user_did.into(),
            started_at: Utc::now(),
        }
    }

    /// Typing indicators expire after this duration (no refresh).
    pub fn is_expired(&self) -> bool {
        Utc::now() - self.started_at > Duration::seconds(10)
    }
}

/// Tracks who is currently typing in each channel.
#[derive(Default)]
pub struct TypingTracker {
    /// channel_id -> (user_did -> started_at)
    typing: HashMap<String, HashMap<String, DateTime<Utc>>>,
}

impl TypingTracker {
    pub fn new() -> Self {
        Self::default()
    }

    /// Record that a user started typing.
    pub fn set_typing(&mut self, channel_id: &str, user_did: &str) {
        self.typing
            .entry(channel_id.into())
            .or_default()
            .insert(user_did.into(), Utc::now());
    }

    /// Record that a user stopped typing (sent message or cancelled).
    pub fn clear_typing(&mut self, channel_id: &str, user_did: &str) {
        if let Some(channel) = self.typing.get_mut(channel_id) {
            channel.remove(user_did);
        }
    }

    /// Get all users currently typing in a channel (excluding expired).
    pub fn typing_in(&self, channel_id: &str) -> Vec<&str> {
        let now = Utc::now();
        let Some(channel) = self.typing.get(channel_id) else {
            return Vec::new();
        };

        channel
            .iter()
            .filter(|(_, started)| now - **started <= Duration::seconds(10))
            .map(|(did, _)| did.as_str())
            .collect()
    }

    /// Prune expired typing indicators across all channels.
    pub fn prune_expired(&mut self) {
        let now = Utc::now();
        for channel in self.typing.values_mut() {
            channel.retain(|_, started| now - *started <= Duration::seconds(10));
        }
        self.typing.retain(|_, channel| !channel.is_empty());
    }
}

/// A read receipt for a message.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReadReceipt {
    pub message_id: String,
    pub channel_id: String,
    pub reader_did: String,
    pub read_at: DateTime<Utc>,
}

impl ReadReceipt {
    pub fn new(message_id: &str, channel_id: &str, reader_did: &str) -> Self {
        Self {
            message_id: message_id.into(),
            channel_id: channel_id.into(),
            reader_did: reader_did.into(),
            read_at: Utc::now(),
        }
    }
}

/// Tracks read receipts per channel.
#[derive(Default)]
pub struct ReadReceiptTracker {
    /// message_id -> set of reader DIDs
    receipts: HashMap<String, HashSet<String>>,
    /// channel_id -> user_did -> last read message_id
    last_read: HashMap<String, HashMap<String, String>>,
}

impl ReadReceiptTracker {
    pub fn new() -> Self {
        Self::default()
    }

    /// Record that a user read a message.
    pub fn mark_read(&mut self, receipt: &ReadReceipt) {
        self.receipts
            .entry(receipt.message_id.clone())
            .or_default()
            .insert(receipt.reader_did.clone());

        self.last_read
            .entry(receipt.channel_id.clone())
            .or_default()
            .insert(receipt.reader_did.clone(), receipt.message_id.clone());
    }

    /// Get all readers of a specific message.
    pub fn readers_of(&self, message_id: &str) -> Vec<&str> {
        self.receipts
            .get(message_id)
            .map(|readers| readers.iter().map(|s| s.as_str()).collect())
            .unwrap_or_default()
    }

    /// Get the number of readers for a message.
    pub fn read_count(&self, message_id: &str) -> usize {
        self.receipts
            .get(message_id)
            .map(|r| r.len())
            .unwrap_or(0)
    }

    /// Check if a specific user has read a message.
    pub fn has_read(&self, message_id: &str, user_did: &str) -> bool {
        self.receipts
            .get(message_id)
            .is_some_and(|r| r.contains(user_did))
    }

    /// Get the last message a user read in a channel.
    pub fn last_read_in(&self, channel_id: &str, user_did: &str) -> Option<&str> {
        self.last_read
            .get(channel_id)
            .and_then(|m| m.get(user_did))
            .map(|s| s.as_str())
    }

    /// Count of unread messages for a user in a channel.
    /// Requires a list of message IDs in order (newest first).
    pub fn unread_count(&self, channel_id: &str, user_did: &str, message_ids: &[&str]) -> usize {
        let last = self.last_read_in(channel_id, user_did);
        match last {
            None => message_ids.len(), // never read anything
            Some(last_id) => {
                message_ids
                    .iter()
                    .position(|id| *id == last_id)
                    .unwrap_or(message_ids.len())
            }
        }
    }
}

/// Online/offline presence status.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PresenceStatus {
    Online,
    Away,
    DoNotDisturb,
    Offline,
}

impl PresenceStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Online => "online",
            Self::Away => "away",
            Self::DoNotDisturb => "dnd",
            Self::Offline => "offline",
        }
    }

    pub fn is_available(&self) -> bool {
        matches!(self, Self::Online | Self::Away)
    }
}

/// A user's presence state.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserPresence {
    pub user_did: String,
    pub status: PresenceStatus,
    pub last_seen: DateTime<Utc>,
    pub status_text: Option<String>,
}

impl UserPresence {
    pub fn online(user_did: &str) -> Self {
        Self {
            user_did: user_did.into(),
            status: PresenceStatus::Online,
            last_seen: Utc::now(),
            status_text: None,
        }
    }

    pub fn with_status_text(mut self, text: impl Into<String>) -> Self {
        self.status_text = Some(text.into());
        self
    }

    pub fn is_stale(&self, timeout_minutes: i64) -> bool {
        Utc::now() - self.last_seen > Duration::minutes(timeout_minutes)
    }
}

/// Tracks presence for all known users.
#[derive(Default)]
pub struct PresenceTracker {
    users: HashMap<String, UserPresence>,
}

impl PresenceTracker {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn update(&mut self, presence: UserPresence) {
        self.users.insert(presence.user_did.clone(), presence);
    }

    pub fn get(&self, user_did: &str) -> Option<&UserPresence> {
        self.users.get(user_did)
    }

    pub fn status_of(&self, user_did: &str) -> PresenceStatus {
        self.users
            .get(user_did)
            .map(|p| p.status)
            .unwrap_or(PresenceStatus::Offline)
    }

    pub fn online_users(&self) -> Vec<&str> {
        self.users
            .values()
            .filter(|p| p.status.is_available())
            .map(|p| p.user_did.as_str())
            .collect()
    }

    pub fn online_count(&self) -> usize {
        self.users.values().filter(|p| p.status.is_available()).count()
    }

    /// Mark stale users as offline.
    pub fn expire_stale(&mut self, timeout_minutes: i64) {
        for presence in self.users.values_mut() {
            if presence.is_stale(timeout_minutes) && presence.status != PresenceStatus::Offline {
                presence.status = PresenceStatus::Offline;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ── Typing Indicators ──

    #[test]
    fn typing_set_and_query() {
        let mut tracker = TypingTracker::new();
        tracker.set_typing("ch1", "alice");
        tracker.set_typing("ch1", "bob");

        let typing = tracker.typing_in("ch1");
        assert_eq!(typing.len(), 2);
    }

    #[test]
    fn typing_clear() {
        let mut tracker = TypingTracker::new();
        tracker.set_typing("ch1", "alice");
        tracker.clear_typing("ch1", "alice");

        assert!(tracker.typing_in("ch1").is_empty());
    }

    #[test]
    fn typing_empty_channel() {
        let tracker = TypingTracker::new();
        assert!(tracker.typing_in("nonexistent").is_empty());
    }

    #[test]
    fn typing_indicator_fresh() {
        let indicator = TypingIndicator::new("ch1", "alice");
        assert!(!indicator.is_expired());
    }

    #[test]
    fn typing_indicator_expired() {
        let mut indicator = TypingIndicator::new("ch1", "alice");
        indicator.started_at = Utc::now() - Duration::seconds(15);
        assert!(indicator.is_expired());
    }

    // ── Read Receipts ──

    #[test]
    fn mark_read_and_query() {
        let mut tracker = ReadReceiptTracker::new();
        let receipt = ReadReceipt::new("msg:1", "ch1", "alice");
        tracker.mark_read(&receipt);

        assert!(tracker.has_read("msg:1", "alice"));
        assert!(!tracker.has_read("msg:1", "bob"));
        assert_eq!(tracker.read_count("msg:1"), 1);
    }

    #[test]
    fn multiple_readers() {
        let mut tracker = ReadReceiptTracker::new();
        tracker.mark_read(&ReadReceipt::new("msg:1", "ch1", "alice"));
        tracker.mark_read(&ReadReceipt::new("msg:1", "ch1", "bob"));
        tracker.mark_read(&ReadReceipt::new("msg:1", "ch1", "carol"));

        assert_eq!(tracker.read_count("msg:1"), 3);
        let readers = tracker.readers_of("msg:1");
        assert_eq!(readers.len(), 3);
    }

    #[test]
    fn last_read_in_channel() {
        let mut tracker = ReadReceiptTracker::new();
        tracker.mark_read(&ReadReceipt::new("msg:1", "ch1", "alice"));
        tracker.mark_read(&ReadReceipt::new("msg:2", "ch1", "alice"));

        assert_eq!(tracker.last_read_in("ch1", "alice"), Some("msg:2"));
    }

    #[test]
    fn unread_count() {
        let mut tracker = ReadReceiptTracker::new();
        tracker.mark_read(&ReadReceipt::new("msg:3", "ch1", "alice"));

        // Messages in order (newest first): msg:5, msg:4, msg:3, msg:2, msg:1
        let msgs = vec!["msg:5", "msg:4", "msg:3", "msg:2", "msg:1"];
        assert_eq!(tracker.unread_count("ch1", "alice", &msgs), 2);
    }

    #[test]
    fn unread_count_never_read() {
        let tracker = ReadReceiptTracker::new();
        let msgs = vec!["msg:3", "msg:2", "msg:1"];
        assert_eq!(tracker.unread_count("ch1", "alice", &msgs), 3);
    }

    #[test]
    fn unread_count_all_read() {
        let mut tracker = ReadReceiptTracker::new();
        tracker.mark_read(&ReadReceipt::new("msg:3", "ch1", "alice"));
        let msgs = vec!["msg:3", "msg:2", "msg:1"];
        assert_eq!(tracker.unread_count("ch1", "alice", &msgs), 0);
    }

    #[test]
    fn read_receipt_serializes() {
        let receipt = ReadReceipt::new("msg:1", "ch1", "alice");
        let json = serde_json::to_string(&receipt).unwrap();
        let restored: ReadReceipt = serde_json::from_str(&json).unwrap();
        assert_eq!(restored.message_id, "msg:1");
        assert_eq!(restored.reader_did, "alice");
    }

    // ── Presence ──

    #[test]
    fn presence_online() {
        let presence = UserPresence::online("alice");
        assert_eq!(presence.status, PresenceStatus::Online);
        assert!(!presence.is_stale(5));
    }

    #[test]
    fn presence_with_status_text() {
        let presence = UserPresence::online("alice").with_status_text("reviewing PRs");
        assert_eq!(presence.status_text.as_deref(), Some("reviewing PRs"));
    }

    #[test]
    fn presence_stale() {
        let mut presence = UserPresence::online("alice");
        presence.last_seen = Utc::now() - Duration::minutes(10);
        assert!(presence.is_stale(5));
        assert!(!presence.is_stale(15));
    }

    #[test]
    fn presence_tracker_update_and_query() {
        let mut tracker = PresenceTracker::new();
        tracker.update(UserPresence::online("alice"));
        tracker.update(UserPresence::online("bob"));

        assert_eq!(tracker.status_of("alice"), PresenceStatus::Online);
        assert_eq!(tracker.status_of("unknown"), PresenceStatus::Offline);
        assert_eq!(tracker.online_count(), 2);
    }

    #[test]
    fn presence_tracker_expire_stale() {
        let mut tracker = PresenceTracker::new();
        let mut stale = UserPresence::online("alice");
        stale.last_seen = Utc::now() - Duration::minutes(10);
        tracker.update(stale);
        tracker.update(UserPresence::online("bob"));

        tracker.expire_stale(5);
        assert_eq!(tracker.status_of("alice"), PresenceStatus::Offline);
        assert_eq!(tracker.status_of("bob"), PresenceStatus::Online);
        assert_eq!(tracker.online_count(), 1);
    }

    #[test]
    fn presence_status_str() {
        assert_eq!(PresenceStatus::Online.as_str(), "online");
        assert_eq!(PresenceStatus::Away.as_str(), "away");
        assert_eq!(PresenceStatus::DoNotDisturb.as_str(), "dnd");
        assert_eq!(PresenceStatus::Offline.as_str(), "offline");
    }

    #[test]
    fn presence_availability() {
        assert!(PresenceStatus::Online.is_available());
        assert!(PresenceStatus::Away.is_available());
        assert!(!PresenceStatus::DoNotDisturb.is_available());
        assert!(!PresenceStatus::Offline.is_available());
    }

    #[test]
    fn presence_serializes() {
        let presence = UserPresence::online("alice").with_status_text("coding");
        let json = serde_json::to_string(&presence).unwrap();
        let restored: UserPresence = serde_json::from_str(&json).unwrap();
        assert_eq!(restored.user_did, "alice");
        assert_eq!(restored.status, PresenceStatus::Online);
    }

    #[test]
    fn typing_indicator_serializes() {
        let indicator = TypingIndicator::new("ch1", "alice");
        let json = serde_json::to_string(&indicator).unwrap();
        let restored: TypingIndicator = serde_json::from_str(&json).unwrap();
        assert_eq!(restored.channel_id, "ch1");
        assert_eq!(restored.user_did, "alice");
    }

    #[test]
    fn online_users_list() {
        let mut tracker = PresenceTracker::new();
        tracker.update(UserPresence::online("alice"));
        tracker.update(UserPresence {
            user_did: "bob".into(),
            status: PresenceStatus::DoNotDisturb,
            last_seen: Utc::now(),
            status_text: None,
        });
        tracker.update(UserPresence::online("carol"));

        let online = tracker.online_users();
        assert_eq!(online.len(), 2); // alice and carol
        assert!(!online.contains(&"bob"));
    }
}
