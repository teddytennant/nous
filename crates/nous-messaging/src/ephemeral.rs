use std::collections::BTreeMap;

use chrono::{DateTime, Duration, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Ttl {
    Seconds(u64),
    Minutes(u64),
    Hours(u64),
    Days(u64),
}

impl Ttl {
    pub fn duration(&self) -> Duration {
        match self {
            Self::Seconds(s) => Duration::seconds(*s as i64),
            Self::Minutes(m) => Duration::minutes(*m as i64),
            Self::Hours(h) => Duration::hours(*h as i64),
            Self::Days(d) => Duration::days(*d as i64),
        }
    }

    pub fn as_seconds(&self) -> u64 {
        match self {
            Self::Seconds(s) => *s,
            Self::Minutes(m) => m * 60,
            Self::Hours(h) => h * 3600,
            Self::Days(d) => d * 86400,
        }
    }

    pub fn display(&self) -> String {
        match self {
            Self::Seconds(s) => format!("{s}s"),
            Self::Minutes(m) => format!("{m}m"),
            Self::Hours(h) => format!("{h}h"),
            Self::Days(d) => format!("{d}d"),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EphemeralMessage {
    pub message_id: String,
    pub channel_id: String,
    pub expires_at: DateTime<Utc>,
    pub read_by: Vec<String>,
    pub delete_on_read: bool,
}

impl EphemeralMessage {
    pub fn new(message_id: &str, channel_id: &str, ttl: Ttl) -> Self {
        Self {
            message_id: message_id.into(),
            channel_id: channel_id.into(),
            expires_at: Utc::now() + ttl.duration(),
            read_by: Vec::new(),
            delete_on_read: false,
        }
    }

    pub fn delete_on_read(mut self) -> Self {
        self.delete_on_read = true;
        self
    }

    pub fn is_expired(&self) -> bool {
        Utc::now() >= self.expires_at
    }

    pub fn mark_read(&mut self, reader: &str) {
        if !self.read_by.contains(&reader.to_string()) {
            self.read_by.push(reader.into());
        }
    }

    pub fn should_delete(&self, total_members: usize) -> bool {
        if self.is_expired() {
            return true;
        }
        self.delete_on_read && self.read_by.len() >= total_members
    }

    pub fn remaining_seconds(&self) -> i64 {
        (self.expires_at - Utc::now()).num_seconds()
    }
}

#[derive(Debug, Default)]
pub struct EphemeralStore {
    messages: BTreeMap<String, EphemeralMessage>,
}

impl EphemeralStore {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn track(&mut self, msg: EphemeralMessage) {
        self.messages.insert(msg.message_id.clone(), msg);
    }

    pub fn is_ephemeral(&self, message_id: &str) -> bool {
        self.messages.contains_key(message_id)
    }

    pub fn get(&self, message_id: &str) -> Option<&EphemeralMessage> {
        self.messages.get(message_id)
    }

    pub fn mark_read(&mut self, message_id: &str, reader: &str) -> bool {
        if let Some(msg) = self.messages.get_mut(message_id) {
            msg.mark_read(reader);
            true
        } else {
            false
        }
    }

    pub fn collect_expired(&mut self) -> Vec<String> {
        let expired: Vec<String> = self
            .messages
            .iter()
            .filter(|(_, m)| m.is_expired())
            .map(|(id, _)| id.clone())
            .collect();

        for id in &expired {
            self.messages.remove(id);
        }

        expired
    }

    pub fn collect_read(&mut self, total_members: usize) -> Vec<String> {
        let to_delete: Vec<String> = self
            .messages
            .iter()
            .filter(|(_, m)| m.should_delete(total_members))
            .map(|(id, _)| id.clone())
            .collect();

        for id in &to_delete {
            self.messages.remove(id);
        }

        to_delete
    }

    pub fn len(&self) -> usize {
        self.messages.len()
    }

    pub fn is_empty(&self) -> bool {
        self.messages.is_empty()
    }

    pub fn for_channel(&self, channel_id: &str) -> Vec<&EphemeralMessage> {
        self.messages
            .values()
            .filter(|m| m.channel_id == channel_id)
            .collect()
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ChannelEphemeralPolicy {
    pub default_ttl: Option<Ttl>,
    pub force_ephemeral: bool,
    pub max_ttl: Option<Ttl>,
}

impl ChannelEphemeralPolicy {
    pub fn effective_ttl(&self, requested: Option<Ttl>) -> Option<Ttl> {
        match (requested, self.default_ttl, self.max_ttl) {
            (Some(req), _, Some(max)) => {
                if req.as_seconds() > max.as_seconds() {
                    Some(max)
                } else {
                    Some(req)
                }
            }
            (Some(req), _, None) => Some(req),
            (None, Some(default), _) => Some(default),
            (None, None, _) if self.force_ephemeral => Some(Ttl::Hours(24)),
            (None, None, _) => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ttl_duration() {
        assert_eq!(Ttl::Seconds(30).as_seconds(), 30);
        assert_eq!(Ttl::Minutes(5).as_seconds(), 300);
        assert_eq!(Ttl::Hours(1).as_seconds(), 3600);
        assert_eq!(Ttl::Days(1).as_seconds(), 86400);
    }

    #[test]
    fn ttl_display() {
        assert_eq!(Ttl::Seconds(30).display(), "30s");
        assert_eq!(Ttl::Minutes(5).display(), "5m");
        assert_eq!(Ttl::Hours(1).display(), "1h");
        assert_eq!(Ttl::Days(7).display(), "7d");
    }

    #[test]
    fn ephemeral_message_not_expired() {
        let msg = EphemeralMessage::new("msg1", "ch1", Ttl::Hours(1));
        assert!(!msg.is_expired());
        assert!(msg.remaining_seconds() > 0);
    }

    #[test]
    fn ephemeral_message_expired() {
        let mut msg = EphemeralMessage::new("msg1", "ch1", Ttl::Hours(1));
        msg.expires_at = Utc::now() - Duration::seconds(1);
        assert!(msg.is_expired());
    }

    #[test]
    fn mark_read() {
        let mut msg = EphemeralMessage::new("msg1", "ch1", Ttl::Hours(1));
        msg.mark_read("alice");
        msg.mark_read("alice"); // duplicate
        msg.mark_read("bob");
        assert_eq!(msg.read_by.len(), 2);
    }

    #[test]
    fn should_delete_on_read_when_all_read() {
        let mut msg = EphemeralMessage::new("msg1", "ch1", Ttl::Hours(1)).delete_on_read();
        msg.mark_read("alice");
        msg.mark_read("bob");

        assert!(msg.should_delete(2));
        assert!(!msg.should_delete(3));
    }

    #[test]
    fn should_delete_when_expired() {
        let mut msg = EphemeralMessage::new("msg1", "ch1", Ttl::Hours(1));
        msg.expires_at = Utc::now() - Duration::seconds(1);
        assert!(msg.should_delete(10));
    }

    #[test]
    fn store_track_and_query() {
        let mut store = EphemeralStore::new();
        store.track(EphemeralMessage::new("msg1", "ch1", Ttl::Hours(1)));
        store.track(EphemeralMessage::new("msg2", "ch1", Ttl::Hours(1)));
        store.track(EphemeralMessage::new("msg3", "ch2", Ttl::Hours(1)));

        assert_eq!(store.len(), 3);
        assert!(store.is_ephemeral("msg1"));
        assert!(!store.is_ephemeral("msg99"));
        assert_eq!(store.for_channel("ch1").len(), 2);
        assert_eq!(store.for_channel("ch2").len(), 1);
    }

    #[test]
    fn store_mark_read() {
        let mut store = EphemeralStore::new();
        store.track(EphemeralMessage::new("msg1", "ch1", Ttl::Hours(1)));

        assert!(store.mark_read("msg1", "alice"));
        assert!(!store.mark_read("msg99", "alice"));

        let msg = store.get("msg1").unwrap();
        assert_eq!(msg.read_by.len(), 1);
    }

    #[test]
    fn store_collect_expired() {
        let mut store = EphemeralStore::new();
        let mut expired = EphemeralMessage::new("msg1", "ch1", Ttl::Hours(1));
        expired.expires_at = Utc::now() - Duration::seconds(1);
        store.track(expired);
        store.track(EphemeralMessage::new("msg2", "ch1", Ttl::Hours(1)));

        let collected = store.collect_expired();
        assert_eq!(collected, vec!["msg1"]);
        assert_eq!(store.len(), 1);
    }

    #[test]
    fn store_collect_read() {
        let mut store = EphemeralStore::new();
        let mut msg = EphemeralMessage::new("msg1", "ch1", Ttl::Hours(1)).delete_on_read();
        msg.mark_read("alice");
        msg.mark_read("bob");
        store.track(msg);
        store.track(EphemeralMessage::new("msg2", "ch1", Ttl::Hours(1)));

        let collected = store.collect_read(2);
        assert_eq!(collected, vec!["msg1"]);
        assert_eq!(store.len(), 1);
    }

    #[test]
    fn policy_no_ephemeral() {
        let policy = ChannelEphemeralPolicy::default();
        assert_eq!(policy.effective_ttl(None), None);
    }

    #[test]
    fn policy_with_default() {
        let policy = ChannelEphemeralPolicy {
            default_ttl: Some(Ttl::Hours(24)),
            ..Default::default()
        };
        assert_eq!(policy.effective_ttl(None), Some(Ttl::Hours(24)));
    }

    #[test]
    fn policy_requested_overrides_default() {
        let policy = ChannelEphemeralPolicy {
            default_ttl: Some(Ttl::Hours(24)),
            ..Default::default()
        };
        assert_eq!(
            policy.effective_ttl(Some(Ttl::Hours(1))),
            Some(Ttl::Hours(1))
        );
    }

    #[test]
    fn policy_max_ttl_caps_request() {
        let policy = ChannelEphemeralPolicy {
            max_ttl: Some(Ttl::Hours(12)),
            ..Default::default()
        };
        assert_eq!(
            policy.effective_ttl(Some(Ttl::Days(1))),
            Some(Ttl::Hours(12))
        );
    }

    #[test]
    fn policy_request_within_max() {
        let policy = ChannelEphemeralPolicy {
            max_ttl: Some(Ttl::Hours(12)),
            ..Default::default()
        };
        assert_eq!(
            policy.effective_ttl(Some(Ttl::Hours(6))),
            Some(Ttl::Hours(6))
        );
    }

    #[test]
    fn policy_force_ephemeral() {
        let policy = ChannelEphemeralPolicy {
            force_ephemeral: true,
            ..Default::default()
        };
        assert_eq!(policy.effective_ttl(None), Some(Ttl::Hours(24)));
    }

    #[test]
    fn ephemeral_message_serializes() {
        let msg = EphemeralMessage::new("msg1", "ch1", Ttl::Hours(1)).delete_on_read();
        let json = serde_json::to_string(&msg).unwrap();
        let restored: EphemeralMessage = serde_json::from_str(&json).unwrap();
        assert_eq!(restored.message_id, "msg1");
        assert!(restored.delete_on_read);
    }

    #[test]
    fn policy_serializes() {
        let policy = ChannelEphemeralPolicy {
            default_ttl: Some(Ttl::Hours(24)),
            force_ephemeral: true,
            max_ttl: Some(Ttl::Days(7)),
        };
        let json = serde_json::to_string(&policy).unwrap();
        let restored: ChannelEphemeralPolicy = serde_json::from_str(&json).unwrap();
        assert!(restored.force_ephemeral);
    }
}
