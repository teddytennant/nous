use std::collections::VecDeque;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum AuditSeverity {
    Debug,
    Info,
    Warning,
    Critical,
}

impl AuditSeverity {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Debug => "debug",
            Self::Info => "info",
            Self::Warning => "warning",
            Self::Critical => "critical",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum AuditAction {
    IdentityCreated,
    IdentityRestored,
    KeyRotated,
    CredentialIssued,
    CredentialVerified,
    CredentialRevoked,
    MessageSent,
    MessageReceived,
    MessageDecrypted,
    ChannelCreated,
    ChannelJoined,
    VoteCast,
    ProposalCreated,
    ProposalExecuted,
    TransferSent,
    TransferReceived,
    EscrowCreated,
    EscrowReleased,
    EscrowRefunded,
    CapabilityGranted,
    CapabilityRevoked,
    CapabilityDelegated,
    AuthSuccess,
    AuthFailure,
    PeerConnected,
    PeerDisconnected,
    FileUploaded,
    FileDownloaded,
    FileShared,
    ListingCreated,
    OrderPlaced,
    NodeStarted,
    NodeStopped,
    ConfigChanged,
}

impl AuditAction {
    pub fn severity(&self) -> AuditSeverity {
        match self {
            Self::AuthFailure | Self::CapabilityRevoked | Self::CredentialRevoked => {
                AuditSeverity::Warning
            }
            Self::ConfigChanged | Self::KeyRotated | Self::NodeStopped => AuditSeverity::Warning,
            Self::PeerConnected
            | Self::PeerDisconnected
            | Self::MessageReceived
            | Self::MessageDecrypted => AuditSeverity::Debug,
            _ => AuditSeverity::Info,
        }
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            Self::IdentityCreated => "identity.created",
            Self::IdentityRestored => "identity.restored",
            Self::KeyRotated => "key.rotated",
            Self::CredentialIssued => "credential.issued",
            Self::CredentialVerified => "credential.verified",
            Self::CredentialRevoked => "credential.revoked",
            Self::MessageSent => "message.sent",
            Self::MessageReceived => "message.received",
            Self::MessageDecrypted => "message.decrypted",
            Self::ChannelCreated => "channel.created",
            Self::ChannelJoined => "channel.joined",
            Self::VoteCast => "vote.cast",
            Self::ProposalCreated => "proposal.created",
            Self::ProposalExecuted => "proposal.executed",
            Self::TransferSent => "transfer.sent",
            Self::TransferReceived => "transfer.received",
            Self::EscrowCreated => "escrow.created",
            Self::EscrowReleased => "escrow.released",
            Self::EscrowRefunded => "escrow.refunded",
            Self::CapabilityGranted => "capability.granted",
            Self::CapabilityRevoked => "capability.revoked",
            Self::CapabilityDelegated => "capability.delegated",
            Self::AuthSuccess => "auth.success",
            Self::AuthFailure => "auth.failure",
            Self::PeerConnected => "peer.connected",
            Self::PeerDisconnected => "peer.disconnected",
            Self::FileUploaded => "file.uploaded",
            Self::FileDownloaded => "file.downloaded",
            Self::FileShared => "file.shared",
            Self::ListingCreated => "listing.created",
            Self::OrderPlaced => "order.placed",
            Self::NodeStarted => "node.started",
            Self::NodeStopped => "node.stopped",
            Self::ConfigChanged => "config.changed",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditEntry {
    pub id: String,
    pub timestamp: DateTime<Utc>,
    pub actor: String,
    pub action: AuditAction,
    pub resource: Option<String>,
    pub detail: Option<String>,
    pub severity: AuditSeverity,
    pub success: bool,
}

impl AuditEntry {
    pub fn new(actor: &str, action: AuditAction) -> Self {
        Self {
            id: format!("audit:{}", Uuid::new_v4()),
            timestamp: Utc::now(),
            actor: actor.into(),
            severity: action.severity(),
            action,
            resource: None,
            detail: None,
            success: true,
        }
    }

    pub fn with_resource(mut self, resource: impl Into<String>) -> Self {
        self.resource = Some(resource.into());
        self
    }

    pub fn with_detail(mut self, detail: impl Into<String>) -> Self {
        self.detail = Some(detail.into());
        self
    }

    pub fn failed(mut self) -> Self {
        self.success = false;
        self
    }
}

pub struct AuditLog {
    entries: VecDeque<AuditEntry>,
    max_entries: usize,
}

impl AuditLog {
    pub fn new(max_entries: usize) -> Self {
        Self {
            entries: VecDeque::new(),
            max_entries,
        }
    }

    pub fn record(&mut self, entry: AuditEntry) {
        if self.entries.len() >= self.max_entries {
            self.entries.pop_front();
        }
        self.entries.push_back(entry);
    }

    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    pub fn entries(&self) -> impl Iterator<Item = &AuditEntry> {
        self.entries.iter()
    }

    pub fn recent(&self, count: usize) -> Vec<&AuditEntry> {
        self.entries.iter().rev().take(count).collect()
    }

    pub fn by_actor(&self, actor: &str) -> Vec<&AuditEntry> {
        self.entries.iter().filter(|e| e.actor == actor).collect()
    }

    pub fn by_action(&self, action: AuditAction) -> Vec<&AuditEntry> {
        self.entries.iter().filter(|e| e.action == action).collect()
    }

    pub fn by_severity(&self, severity: AuditSeverity) -> Vec<&AuditEntry> {
        self.entries
            .iter()
            .filter(|e| e.severity == severity)
            .collect()
    }

    pub fn failures(&self) -> Vec<&AuditEntry> {
        self.entries.iter().filter(|e| !e.success).collect()
    }

    pub fn since(&self, since: DateTime<Utc>) -> Vec<&AuditEntry> {
        self.entries
            .iter()
            .filter(|e| e.timestamp >= since)
            .collect()
    }

    pub fn clear(&mut self) {
        self.entries.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Duration;

    #[test]
    fn create_audit_entry() {
        let entry = AuditEntry::new("did:key:zalice", AuditAction::IdentityCreated);
        assert!(entry.id.starts_with("audit:"));
        assert!(entry.success);
        assert_eq!(entry.severity, AuditSeverity::Info);
    }

    #[test]
    fn entry_with_resource_and_detail() {
        let entry = AuditEntry::new("alice", AuditAction::TransferSent)
            .with_resource("/wallet/eth")
            .with_detail("sent 100 ETH to bob");

        assert_eq!(entry.resource.as_deref(), Some("/wallet/eth"));
        assert_eq!(entry.detail.as_deref(), Some("sent 100 ETH to bob"));
    }

    #[test]
    fn failed_entry() {
        let entry = AuditEntry::new("alice", AuditAction::AuthFailure).failed();
        assert!(!entry.success);
    }

    #[test]
    fn auth_failure_is_warning() {
        let entry = AuditEntry::new("alice", AuditAction::AuthFailure);
        assert_eq!(entry.severity, AuditSeverity::Warning);
    }

    #[test]
    fn peer_connected_is_debug() {
        let entry = AuditEntry::new("alice", AuditAction::PeerConnected);
        assert_eq!(entry.severity, AuditSeverity::Debug);
    }

    #[test]
    fn audit_log_record_and_len() {
        let mut log = AuditLog::new(100);
        assert!(log.is_empty());

        log.record(AuditEntry::new("alice", AuditAction::NodeStarted));
        assert_eq!(log.len(), 1);
        assert!(!log.is_empty());
    }

    #[test]
    fn audit_log_evicts_oldest() {
        let mut log = AuditLog::new(3);
        for i in 0..5 {
            log.record(
                AuditEntry::new("alice", AuditAction::MessageSent).with_detail(format!("msg {i}")),
            );
        }
        assert_eq!(log.len(), 3);
        let recent: Vec<_> = log.entries().collect();
        assert_eq!(recent[0].detail.as_deref(), Some("msg 2"));
    }

    #[test]
    fn audit_log_recent() {
        let mut log = AuditLog::new(100);
        log.record(AuditEntry::new("alice", AuditAction::MessageSent).with_detail("first"));
        log.record(AuditEntry::new("alice", AuditAction::MessageSent).with_detail("second"));
        log.record(AuditEntry::new("alice", AuditAction::MessageSent).with_detail("third"));

        let recent = log.recent(2);
        assert_eq!(recent.len(), 2);
        assert_eq!(recent[0].detail.as_deref(), Some("third"));
        assert_eq!(recent[1].detail.as_deref(), Some("second"));
    }

    #[test]
    fn audit_log_by_actor() {
        let mut log = AuditLog::new(100);
        log.record(AuditEntry::new("alice", AuditAction::VoteCast));
        log.record(AuditEntry::new("bob", AuditAction::VoteCast));
        log.record(AuditEntry::new("alice", AuditAction::TransferSent));

        assert_eq!(log.by_actor("alice").len(), 2);
        assert_eq!(log.by_actor("bob").len(), 1);
        assert_eq!(log.by_actor("charlie").len(), 0);
    }

    #[test]
    fn audit_log_by_action() {
        let mut log = AuditLog::new(100);
        log.record(AuditEntry::new("alice", AuditAction::VoteCast));
        log.record(AuditEntry::new("bob", AuditAction::VoteCast));
        log.record(AuditEntry::new("alice", AuditAction::TransferSent));

        assert_eq!(log.by_action(AuditAction::VoteCast).len(), 2);
        assert_eq!(log.by_action(AuditAction::TransferSent).len(), 1);
    }

    #[test]
    fn audit_log_by_severity() {
        let mut log = AuditLog::new(100);
        log.record(AuditEntry::new("alice", AuditAction::AuthFailure));
        log.record(AuditEntry::new("alice", AuditAction::AuthSuccess));
        log.record(AuditEntry::new("alice", AuditAction::KeyRotated));

        assert_eq!(log.by_severity(AuditSeverity::Warning).len(), 2);
        assert_eq!(log.by_severity(AuditSeverity::Info).len(), 1);
    }

    #[test]
    fn audit_log_failures() {
        let mut log = AuditLog::new(100);
        log.record(AuditEntry::new("alice", AuditAction::AuthSuccess));
        log.record(AuditEntry::new("alice", AuditAction::AuthFailure).failed());
        log.record(AuditEntry::new("alice", AuditAction::TransferSent).failed());

        assert_eq!(log.failures().len(), 2);
    }

    #[test]
    fn audit_log_since() {
        let mut log = AuditLog::new(100);
        log.record(AuditEntry::new("alice", AuditAction::NodeStarted));

        let future = Utc::now() + Duration::hours(1);
        assert_eq!(log.since(future).len(), 0);

        let past = Utc::now() - Duration::hours(1);
        assert_eq!(log.since(past).len(), 1);
    }

    #[test]
    fn audit_log_clear() {
        let mut log = AuditLog::new(100);
        log.record(AuditEntry::new("alice", AuditAction::NodeStarted));
        log.record(AuditEntry::new("alice", AuditAction::NodeStopped));

        log.clear();
        assert!(log.is_empty());
    }

    #[test]
    fn action_as_str() {
        assert_eq!(AuditAction::IdentityCreated.as_str(), "identity.created");
        assert_eq!(AuditAction::AuthFailure.as_str(), "auth.failure");
        assert_eq!(AuditAction::TransferSent.as_str(), "transfer.sent");
    }

    #[test]
    fn severity_as_str() {
        assert_eq!(AuditSeverity::Debug.as_str(), "debug");
        assert_eq!(AuditSeverity::Info.as_str(), "info");
        assert_eq!(AuditSeverity::Warning.as_str(), "warning");
        assert_eq!(AuditSeverity::Critical.as_str(), "critical");
    }

    #[test]
    fn audit_entry_serializes() {
        let entry = AuditEntry::new("alice", AuditAction::VoteCast)
            .with_resource("/dao/1")
            .with_detail("voted yes on proposal 42");

        let json = serde_json::to_string(&entry).unwrap();
        let restored: AuditEntry = serde_json::from_str(&json).unwrap();
        assert_eq!(restored.actor, "alice");
        assert_eq!(restored.action, AuditAction::VoteCast);
    }
}
