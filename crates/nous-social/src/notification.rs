use std::collections::VecDeque;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum NotificationType {
    Mention,
    Reply,
    Reaction,
    Repost,
    Follow,
    Unfollow,
    DirectMessage,
    GovernanceProposal,
    GovernanceVote,
    TransferReceived,
    EscrowUpdate,
    SystemAlert,
}

impl NotificationType {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Mention => "mention",
            Self::Reply => "reply",
            Self::Reaction => "reaction",
            Self::Repost => "repost",
            Self::Follow => "follow",
            Self::Unfollow => "unfollow",
            Self::DirectMessage => "dm",
            Self::GovernanceProposal => "gov.proposal",
            Self::GovernanceVote => "gov.vote",
            Self::TransferReceived => "transfer.received",
            Self::EscrowUpdate => "escrow.update",
            Self::SystemAlert => "system.alert",
        }
    }

    pub fn is_social(&self) -> bool {
        matches!(
            self,
            Self::Mention
                | Self::Reply
                | Self::Reaction
                | Self::Repost
                | Self::Follow
                | Self::Unfollow
        )
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Notification {
    pub id: String,
    pub recipient: String,
    pub actor: String,
    pub notification_type: NotificationType,
    pub reference_id: Option<String>,
    pub summary: String,
    pub read: bool,
    pub created_at: DateTime<Utc>,
}

impl Notification {
    pub fn new(
        recipient: &str,
        actor: &str,
        notification_type: NotificationType,
        summary: &str,
    ) -> Self {
        Self {
            id: format!("notif:{}", Uuid::new_v4()),
            recipient: recipient.into(),
            actor: actor.into(),
            notification_type,
            reference_id: None,
            summary: summary.into(),
            read: false,
            created_at: Utc::now(),
        }
    }

    pub fn with_reference(mut self, ref_id: impl Into<String>) -> Self {
        self.reference_id = Some(ref_id.into());
        self
    }

    pub fn mark_read(&mut self) {
        self.read = true;
    }
}

#[derive(Debug)]
pub struct NotificationInbox {
    owner: String,
    notifications: VecDeque<Notification>,
    max_notifications: usize,
}

impl NotificationInbox {
    pub fn new(owner: impl Into<String>, max: usize) -> Self {
        Self {
            owner: owner.into(),
            notifications: VecDeque::new(),
            max_notifications: max,
        }
    }

    pub fn push(&mut self, notification: Notification) {
        if notification.recipient != self.owner {
            return;
        }
        if self.notifications.len() >= self.max_notifications {
            self.notifications.pop_back();
        }
        self.notifications.push_front(notification);
    }

    pub fn len(&self) -> usize {
        self.notifications.len()
    }

    pub fn is_empty(&self) -> bool {
        self.notifications.is_empty()
    }

    pub fn unread_count(&self) -> usize {
        self.notifications.iter().filter(|n| !n.read).count()
    }

    pub fn all(&self) -> impl Iterator<Item = &Notification> {
        self.notifications.iter()
    }

    pub fn unread(&self) -> Vec<&Notification> {
        self.notifications.iter().filter(|n| !n.read).collect()
    }

    pub fn by_type(&self, notification_type: NotificationType) -> Vec<&Notification> {
        self.notifications
            .iter()
            .filter(|n| n.notification_type == notification_type)
            .collect()
    }

    pub fn mark_read(&mut self, notification_id: &str) -> bool {
        if let Some(n) = self
            .notifications
            .iter_mut()
            .find(|n| n.id == notification_id)
        {
            n.mark_read();
            true
        } else {
            false
        }
    }

    pub fn mark_all_read(&mut self) -> usize {
        let mut count = 0;
        for n in &mut self.notifications {
            if !n.read {
                n.read = true;
                count += 1;
            }
        }
        count
    }

    pub fn recent(&self, count: usize) -> Vec<&Notification> {
        self.notifications.iter().take(count).collect()
    }

    pub fn clear(&mut self) {
        self.notifications.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn notif(recipient: &str, actor: &str, typ: NotificationType) -> Notification {
        Notification::new(recipient, actor, typ, "test notification")
    }

    #[test]
    fn create_notification() {
        let n = Notification::new(
            "alice",
            "bob",
            NotificationType::Mention,
            "bob mentioned you",
        );
        assert!(n.id.starts_with("notif:"));
        assert!(!n.read);
        assert_eq!(n.recipient, "alice");
    }

    #[test]
    fn with_reference() {
        let n = Notification::new("alice", "bob", NotificationType::Reply, "replied")
            .with_reference("event123");
        assert_eq!(n.reference_id.as_deref(), Some("event123"));
    }

    #[test]
    fn mark_read() {
        let mut n = notif("alice", "bob", NotificationType::Mention);
        assert!(!n.read);
        n.mark_read();
        assert!(n.read);
    }

    #[test]
    fn inbox_push_and_count() {
        let mut inbox = NotificationInbox::new("alice", 100);
        inbox.push(notif("alice", "bob", NotificationType::Follow));
        inbox.push(notif("alice", "carol", NotificationType::Mention));

        assert_eq!(inbox.len(), 2);
        assert_eq!(inbox.unread_count(), 2);
    }

    #[test]
    fn inbox_ignores_wrong_recipient() {
        let mut inbox = NotificationInbox::new("alice", 100);
        inbox.push(notif("bob", "carol", NotificationType::Follow));
        assert!(inbox.is_empty());
    }

    #[test]
    fn inbox_evicts_oldest() {
        let mut inbox = NotificationInbox::new("alice", 3);
        for i in 0..5 {
            inbox.push(Notification::new(
                "alice",
                &format!("user{i}"),
                NotificationType::Mention,
                &format!("mention {i}"),
            ));
        }
        assert_eq!(inbox.len(), 3);
        // Most recent should be first
        let recent = inbox.recent(1);
        assert_eq!(recent[0].summary, "mention 4");
    }

    #[test]
    fn inbox_unread_filter() {
        let mut inbox = NotificationInbox::new("alice", 100);
        inbox.push(notif("alice", "bob", NotificationType::Follow));
        inbox.push(notif("alice", "carol", NotificationType::Mention));

        let first_id = inbox.recent(1)[0].id.clone();
        inbox.mark_read(&first_id);

        assert_eq!(inbox.unread_count(), 1);
        assert_eq!(inbox.unread().len(), 1);
    }

    #[test]
    fn inbox_mark_all_read() {
        let mut inbox = NotificationInbox::new("alice", 100);
        inbox.push(notif("alice", "bob", NotificationType::Follow));
        inbox.push(notif("alice", "carol", NotificationType::Mention));
        inbox.push(notif("alice", "dave", NotificationType::Reaction));

        let count = inbox.mark_all_read();
        assert_eq!(count, 3);
        assert_eq!(inbox.unread_count(), 0);
    }

    #[test]
    fn inbox_mark_read_nonexistent() {
        let mut inbox = NotificationInbox::new("alice", 100);
        assert!(!inbox.mark_read("notif:fake"));
    }

    #[test]
    fn inbox_by_type() {
        let mut inbox = NotificationInbox::new("alice", 100);
        inbox.push(notif("alice", "bob", NotificationType::Follow));
        inbox.push(notif("alice", "carol", NotificationType::Mention));
        inbox.push(notif("alice", "dave", NotificationType::Follow));

        assert_eq!(inbox.by_type(NotificationType::Follow).len(), 2);
        assert_eq!(inbox.by_type(NotificationType::Mention).len(), 1);
        assert_eq!(inbox.by_type(NotificationType::Reaction).len(), 0);
    }

    #[test]
    fn inbox_recent() {
        let mut inbox = NotificationInbox::new("alice", 100);
        inbox.push(notif("alice", "bob", NotificationType::Follow));
        inbox.push(notif("alice", "carol", NotificationType::Mention));
        inbox.push(notif("alice", "dave", NotificationType::Reaction));

        let recent = inbox.recent(2);
        assert_eq!(recent.len(), 2);
    }

    #[test]
    fn inbox_clear() {
        let mut inbox = NotificationInbox::new("alice", 100);
        inbox.push(notif("alice", "bob", NotificationType::Follow));
        inbox.clear();
        assert!(inbox.is_empty());
    }

    #[test]
    fn notification_type_as_str() {
        assert_eq!(NotificationType::Mention.as_str(), "mention");
        assert_eq!(
            NotificationType::TransferReceived.as_str(),
            "transfer.received"
        );
    }

    #[test]
    fn notification_type_is_social() {
        assert!(NotificationType::Mention.is_social());
        assert!(NotificationType::Follow.is_social());
        assert!(!NotificationType::TransferReceived.is_social());
        assert!(!NotificationType::SystemAlert.is_social());
    }

    #[test]
    fn notification_serializes() {
        let n = notif("alice", "bob", NotificationType::Mention);
        let json = serde_json::to_string(&n).unwrap();
        let restored: Notification = serde_json::from_str(&json).unwrap();
        assert_eq!(restored.recipient, "alice");
    }
}
