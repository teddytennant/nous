use std::collections::{HashMap, HashSet};

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use nous_core::{Error, Result};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ReportReason {
    Spam,
    Harassment,
    HateSpeech,
    Violence,
    Misinformation,
    Copyright,
    Nsfw,
    Impersonation,
    Other,
}

impl ReportReason {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Spam => "spam",
            Self::Harassment => "harassment",
            Self::HateSpeech => "hate_speech",
            Self::Violence => "violence",
            Self::Misinformation => "misinformation",
            Self::Copyright => "copyright",
            Self::Nsfw => "nsfw",
            Self::Impersonation => "impersonation",
            Self::Other => "other",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ReportStatus {
    Open,
    UnderReview,
    ActionTaken,
    Dismissed,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ModerationAction {
    Warn,
    Mute,
    HideContent,
    RemoveContent,
    SuspendUser,
    None,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Report {
    pub id: String,
    pub reporter_did: String,
    pub target_did: String,
    pub target_event_id: Option<String>,
    pub reason: ReportReason,
    pub description: String,
    pub status: ReportStatus,
    pub action_taken: Option<ModerationAction>,
    pub moderator_did: Option<String>,
    pub created_at: DateTime<Utc>,
    pub resolved_at: Option<DateTime<Utc>>,
}

impl Report {
    pub fn new(
        reporter: &str,
        target: &str,
        reason: ReportReason,
        description: &str,
    ) -> Result<Self> {
        if reporter == target {
            return Err(Error::InvalidInput("cannot report yourself".into()));
        }
        Ok(Self {
            id: format!("report:{}", Uuid::new_v4()),
            reporter_did: reporter.into(),
            target_did: target.into(),
            target_event_id: None,
            reason,
            description: description.into(),
            status: ReportStatus::Open,
            action_taken: None,
            moderator_did: None,
            created_at: Utc::now(),
            resolved_at: None,
        })
    }

    pub fn for_event(mut self, event_id: impl Into<String>) -> Self {
        self.target_event_id = Some(event_id.into());
        self
    }

    pub fn review(&mut self, moderator_did: &str) -> Result<()> {
        if self.status != ReportStatus::Open {
            return Err(Error::InvalidInput("report is not open".into()));
        }
        self.status = ReportStatus::UnderReview;
        self.moderator_did = Some(moderator_did.into());
        Ok(())
    }

    pub fn resolve(&mut self, action: ModerationAction) -> Result<()> {
        if self.status != ReportStatus::UnderReview {
            return Err(Error::InvalidInput("report is not under review".into()));
        }
        self.action_taken = Some(action);
        self.status = if action == ModerationAction::None {
            ReportStatus::Dismissed
        } else {
            ReportStatus::ActionTaken
        };
        self.resolved_at = Some(Utc::now());
        Ok(())
    }

    pub fn is_resolved(&self) -> bool {
        matches!(
            self.status,
            ReportStatus::ActionTaken | ReportStatus::Dismissed
        )
    }
}

#[derive(Debug, Default)]
pub struct ModerationQueue {
    reports: Vec<Report>,
    muted_users: HashMap<String, HashSet<String>>,
    hidden_events: HashSet<String>,
}

impl ModerationQueue {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn submit(&mut self, report: Report) {
        self.reports.push(report);
    }

    pub fn open_reports(&self) -> Vec<&Report> {
        self.reports
            .iter()
            .filter(|r| r.status == ReportStatus::Open)
            .collect()
    }

    pub fn reports_for_target(&self, target_did: &str) -> Vec<&Report> {
        self.reports
            .iter()
            .filter(|r| r.target_did == target_did)
            .collect()
    }

    pub fn reports_by_reason(&self, reason: ReportReason) -> Vec<&Report> {
        self.reports.iter().filter(|r| r.reason == reason).collect()
    }

    pub fn report_count_for_target(&self, target_did: &str) -> usize {
        self.reports_for_target(target_did).len()
    }

    pub fn mute_user(&mut self, user: &str, target: &str) {
        self.muted_users
            .entry(user.to_string())
            .or_default()
            .insert(target.to_string());
    }

    pub fn unmute_user(&mut self, user: &str, target: &str) -> bool {
        self.muted_users
            .get_mut(user)
            .map(|set| set.remove(target))
            .unwrap_or(false)
    }

    pub fn is_muted(&self, user: &str, target: &str) -> bool {
        self.muted_users
            .get(user)
            .map(|set| set.contains(target))
            .unwrap_or(false)
    }

    pub fn muted_by(&self, user: &str) -> Vec<&str> {
        self.muted_users
            .get(user)
            .map(|set| set.iter().map(|s| s.as_str()).collect())
            .unwrap_or_default()
    }

    pub fn hide_event(&mut self, event_id: &str) {
        self.hidden_events.insert(event_id.to_string());
    }

    pub fn unhide_event(&mut self, event_id: &str) -> bool {
        self.hidden_events.remove(event_id)
    }

    pub fn is_hidden(&self, event_id: &str) -> bool {
        self.hidden_events.contains(event_id)
    }

    pub fn hidden_count(&self) -> usize {
        self.hidden_events.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn create_report() {
        let report = Report::new("alice", "bob", ReportReason::Spam, "posting spam links").unwrap();
        assert!(report.id.starts_with("report:"));
        assert_eq!(report.status, ReportStatus::Open);
        assert!(!report.is_resolved());
    }

    #[test]
    fn cannot_report_self() {
        assert!(Report::new("alice", "alice", ReportReason::Spam, "test").is_err());
    }

    #[test]
    fn report_for_event() {
        let report = Report::new("alice", "bob", ReportReason::Harassment, "threatening")
            .unwrap()
            .for_event("event123");
        assert_eq!(report.target_event_id.as_deref(), Some("event123"));
    }

    #[test]
    fn report_review_and_resolve() {
        let mut report = Report::new("alice", "bob", ReportReason::Spam, "spam").unwrap();

        report.review("moderator").unwrap();
        assert_eq!(report.status, ReportStatus::UnderReview);
        assert_eq!(report.moderator_did.as_deref(), Some("moderator"));

        report.resolve(ModerationAction::HideContent).unwrap();
        assert_eq!(report.status, ReportStatus::ActionTaken);
        assert!(report.is_resolved());
        assert!(report.resolved_at.is_some());
    }

    #[test]
    fn report_dismiss() {
        let mut report = Report::new("alice", "bob", ReportReason::Other, "not sure").unwrap();
        report.review("mod").unwrap();
        report.resolve(ModerationAction::None).unwrap();
        assert_eq!(report.status, ReportStatus::Dismissed);
        assert!(report.is_resolved());
    }

    #[test]
    fn review_already_reviewed_fails() {
        let mut report = Report::new("alice", "bob", ReportReason::Spam, "spam").unwrap();
        report.review("mod").unwrap();
        assert!(report.review("mod2").is_err());
    }

    #[test]
    fn resolve_without_review_fails() {
        let mut report = Report::new("alice", "bob", ReportReason::Spam, "spam").unwrap();
        assert!(report.resolve(ModerationAction::Warn).is_err());
    }

    #[test]
    fn moderation_queue_submit_and_filter() {
        let mut queue = ModerationQueue::new();
        queue.submit(Report::new("alice", "bob", ReportReason::Spam, "spam").unwrap());
        queue.submit(Report::new("carol", "bob", ReportReason::Harassment, "mean").unwrap());
        queue.submit(Report::new("alice", "dave", ReportReason::Spam, "spam").unwrap());

        assert_eq!(queue.open_reports().len(), 3);
        assert_eq!(queue.reports_for_target("bob").len(), 2);
        assert_eq!(queue.reports_by_reason(ReportReason::Spam).len(), 2);
        assert_eq!(queue.report_count_for_target("bob"), 2);
    }

    #[test]
    fn mute_and_unmute() {
        let mut queue = ModerationQueue::new();
        queue.mute_user("alice", "bob");

        assert!(queue.is_muted("alice", "bob"));
        assert!(!queue.is_muted("bob", "alice"));
        assert_eq!(queue.muted_by("alice").len(), 1);

        assert!(queue.unmute_user("alice", "bob"));
        assert!(!queue.is_muted("alice", "bob"));
    }

    #[test]
    fn unmute_nonexistent() {
        let mut queue = ModerationQueue::new();
        assert!(!queue.unmute_user("alice", "bob"));
    }

    #[test]
    fn hide_and_unhide_event() {
        let mut queue = ModerationQueue::new();
        queue.hide_event("event123");

        assert!(queue.is_hidden("event123"));
        assert!(!queue.is_hidden("event456"));
        assert_eq!(queue.hidden_count(), 1);

        assert!(queue.unhide_event("event123"));
        assert!(!queue.is_hidden("event123"));
        assert_eq!(queue.hidden_count(), 0);
    }

    #[test]
    fn unhide_nonexistent() {
        let mut queue = ModerationQueue::new();
        assert!(!queue.unhide_event("fake"));
    }

    #[test]
    fn report_reason_as_str() {
        assert_eq!(ReportReason::Spam.as_str(), "spam");
        assert_eq!(ReportReason::HateSpeech.as_str(), "hate_speech");
        assert_eq!(ReportReason::Copyright.as_str(), "copyright");
    }

    #[test]
    fn report_serializes() {
        let report = Report::new("alice", "bob", ReportReason::Spam, "spam").unwrap();
        let json = serde_json::to_string(&report).unwrap();
        let restored: Report = serde_json::from_str(&json).unwrap();
        assert_eq!(restored.reporter_did, "alice");
        assert_eq!(restored.reason, ReportReason::Spam);
    }
}
