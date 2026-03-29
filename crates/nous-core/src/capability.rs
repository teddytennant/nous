use std::collections::HashSet;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::{Error, Result};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Permission {
    Read,
    Write,
    Delete,
    Admin,
    Sign,
    Encrypt,
    Decrypt,
    Vote,
    Propose,
    Transfer,
    Invite,
    Moderate,
}

impl Permission {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Read => "read",
            Self::Write => "write",
            Self::Delete => "delete",
            Self::Admin => "admin",
            Self::Sign => "sign",
            Self::Encrypt => "encrypt",
            Self::Decrypt => "decrypt",
            Self::Vote => "vote",
            Self::Propose => "propose",
            Self::Transfer => "transfer",
            Self::Invite => "invite",
            Self::Moderate => "moderate",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Capability {
    pub id: String,
    pub issuer: String,
    pub subject: String,
    pub resource: String,
    pub permissions: HashSet<Permission>,
    pub issued_at: DateTime<Utc>,
    pub expires_at: Option<DateTime<Utc>>,
    pub delegatable: bool,
    pub parent_id: Option<String>,
    pub revoked: bool,
}

impl Capability {
    pub fn new(issuer: &str, subject: &str, resource: &str) -> Self {
        Self {
            id: format!("cap:{}", Uuid::new_v4()),
            issuer: issuer.into(),
            subject: subject.into(),
            resource: resource.into(),
            permissions: HashSet::new(),
            issued_at: Utc::now(),
            expires_at: None,
            delegatable: false,
            parent_id: None,
            revoked: false,
        }
    }

    pub fn with_permission(mut self, perm: Permission) -> Self {
        self.permissions.insert(perm);
        self
    }

    pub fn with_permissions(mut self, perms: impl IntoIterator<Item = Permission>) -> Self {
        self.permissions.extend(perms);
        self
    }

    pub fn with_expiry(mut self, expires_at: DateTime<Utc>) -> Self {
        self.expires_at = Some(expires_at);
        self
    }

    pub fn delegatable(mut self) -> Self {
        self.delegatable = true;
        self
    }

    pub fn is_expired(&self) -> bool {
        self.expires_at
            .map(|exp| exp < Utc::now())
            .unwrap_or(false)
    }

    pub fn is_valid(&self) -> bool {
        !self.revoked && !self.is_expired() && !self.permissions.is_empty()
    }

    pub fn has_permission(&self, perm: Permission) -> bool {
        self.permissions.contains(&Permission::Admin) || self.permissions.contains(&perm)
    }

    pub fn check(&self, caller: &str, perm: Permission) -> Result<()> {
        if self.revoked {
            return Err(Error::PermissionDenied("capability has been revoked".into()));
        }
        if self.is_expired() {
            return Err(Error::Expired("capability has expired".into()));
        }
        if self.subject != caller {
            return Err(Error::PermissionDenied(format!(
                "capability belongs to {}, not {caller}",
                self.subject
            )));
        }
        if !self.has_permission(perm) {
            return Err(Error::PermissionDenied(format!(
                "missing permission: {}",
                perm.as_str()
            )));
        }
        Ok(())
    }

    pub fn revoke(&mut self) {
        self.revoked = true;
    }

    pub fn delegate(&self, new_subject: &str, perms: HashSet<Permission>) -> Result<Self> {
        if !self.delegatable {
            return Err(Error::PermissionDenied(
                "capability is not delegatable".into(),
            ));
        }
        if !self.is_valid() {
            return Err(Error::PermissionDenied(
                "cannot delegate invalid capability".into(),
            ));
        }

        // Delegated permissions must be a subset
        for perm in &perms {
            if !self.has_permission(*perm) {
                return Err(Error::PermissionDenied(format!(
                    "cannot delegate permission {} not held by parent",
                    perm.as_str()
                )));
            }
        }

        Ok(Self {
            id: format!("cap:{}", Uuid::new_v4()),
            issuer: self.subject.clone(),
            subject: new_subject.into(),
            resource: self.resource.clone(),
            permissions: perms,
            issued_at: Utc::now(),
            expires_at: self.expires_at,
            delegatable: false,
            parent_id: Some(self.id.clone()),
            revoked: false,
        })
    }
}

#[derive(Debug, Default)]
pub struct CapabilityStore {
    capabilities: Vec<Capability>,
}

impl CapabilityStore {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn grant(&mut self, cap: Capability) {
        self.capabilities.push(cap);
    }

    pub fn check(&self, caller: &str, resource: &str, perm: Permission) -> Result<()> {
        let cap = self
            .capabilities
            .iter()
            .find(|c| c.subject == caller && c.resource == resource && c.is_valid())
            .ok_or_else(|| {
                Error::PermissionDenied(format!(
                    "no valid capability for {caller} on {resource}"
                ))
            })?;

        cap.check(caller, perm)
    }

    pub fn revoke(&mut self, cap_id: &str) -> bool {
        if let Some(cap) = self.capabilities.iter_mut().find(|c| c.id == cap_id) {
            cap.revoke();
            true
        } else {
            false
        }
    }

    pub fn for_subject(&self, subject: &str) -> Vec<&Capability> {
        self.capabilities
            .iter()
            .filter(|c| c.subject == subject && c.is_valid())
            .collect()
    }

    pub fn for_resource(&self, resource: &str) -> Vec<&Capability> {
        self.capabilities
            .iter()
            .filter(|c| c.resource == resource && c.is_valid())
            .collect()
    }

    pub fn prune_expired(&mut self) -> usize {
        let before = self.capabilities.len();
        self.capabilities.retain(|c| !c.is_expired());
        before - self.capabilities.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Duration;

    #[test]
    fn create_capability() {
        let cap = Capability::new("alice", "bob", "/dao/votes")
            .with_permission(Permission::Read)
            .with_permission(Permission::Vote);

        assert!(cap.id.starts_with("cap:"));
        assert_eq!(cap.permissions.len(), 2);
        assert!(cap.is_valid());
    }

    #[test]
    fn admin_has_all_permissions() {
        let cap = Capability::new("alice", "bob", "/admin").with_permission(Permission::Admin);

        assert!(cap.has_permission(Permission::Read));
        assert!(cap.has_permission(Permission::Write));
        assert!(cap.has_permission(Permission::Delete));
        assert!(cap.has_permission(Permission::Transfer));
    }

    #[test]
    fn check_correct_caller_passes() {
        let cap = Capability::new("alice", "bob", "/data")
            .with_permission(Permission::Read);

        assert!(cap.check("bob", Permission::Read).is_ok());
    }

    #[test]
    fn check_wrong_caller_fails() {
        let cap = Capability::new("alice", "bob", "/data")
            .with_permission(Permission::Read);

        assert!(cap.check("charlie", Permission::Read).is_err());
    }

    #[test]
    fn check_missing_permission_fails() {
        let cap = Capability::new("alice", "bob", "/data")
            .with_permission(Permission::Read);

        assert!(cap.check("bob", Permission::Write).is_err());
    }

    #[test]
    fn expired_capability_is_invalid() {
        let cap = Capability::new("alice", "bob", "/data")
            .with_permission(Permission::Read)
            .with_expiry(Utc::now() - Duration::hours(1));

        assert!(cap.is_expired());
        assert!(!cap.is_valid());
        assert!(cap.check("bob", Permission::Read).is_err());
    }

    #[test]
    fn future_expiry_is_valid() {
        let cap = Capability::new("alice", "bob", "/data")
            .with_permission(Permission::Read)
            .with_expiry(Utc::now() + Duration::hours(1));

        assert!(!cap.is_expired());
        assert!(cap.is_valid());
    }

    #[test]
    fn revoked_capability_is_invalid() {
        let mut cap = Capability::new("alice", "bob", "/data")
            .with_permission(Permission::Read);

        cap.revoke();
        assert!(!cap.is_valid());
        assert!(cap.check("bob", Permission::Read).is_err());
    }

    #[test]
    fn delegate_capability() {
        let cap = Capability::new("alice", "bob", "/data")
            .with_permissions([Permission::Read, Permission::Write])
            .delegatable();

        let delegated = cap
            .delegate("charlie", HashSet::from([Permission::Read]))
            .unwrap();

        assert_eq!(delegated.issuer, "bob");
        assert_eq!(delegated.subject, "charlie");
        assert_eq!(delegated.permissions.len(), 1);
        assert!(delegated.has_permission(Permission::Read));
        assert!(!delegated.delegatable);
        assert_eq!(delegated.parent_id.as_deref(), Some(cap.id.as_str()));
    }

    #[test]
    fn delegate_non_delegatable_fails() {
        let cap = Capability::new("alice", "bob", "/data")
            .with_permission(Permission::Read);

        assert!(cap
            .delegate("charlie", HashSet::from([Permission::Read]))
            .is_err());
    }

    #[test]
    fn delegate_escalation_fails() {
        let cap = Capability::new("alice", "bob", "/data")
            .with_permission(Permission::Read)
            .delegatable();

        assert!(cap
            .delegate("charlie", HashSet::from([Permission::Write]))
            .is_err());
    }

    #[test]
    fn capability_store_grant_and_check() {
        let mut store = CapabilityStore::new();
        store.grant(
            Capability::new("alice", "bob", "/data")
                .with_permission(Permission::Read),
        );

        assert!(store.check("bob", "/data", Permission::Read).is_ok());
        assert!(store.check("bob", "/data", Permission::Write).is_err());
        assert!(store.check("charlie", "/data", Permission::Read).is_err());
    }

    #[test]
    fn capability_store_revoke() {
        let mut store = CapabilityStore::new();
        let cap = Capability::new("alice", "bob", "/data")
            .with_permission(Permission::Read);
        let cap_id = cap.id.clone();
        store.grant(cap);

        assert!(store.check("bob", "/data", Permission::Read).is_ok());
        assert!(store.revoke(&cap_id));
        assert!(store.check("bob", "/data", Permission::Read).is_err());
    }

    #[test]
    fn capability_store_for_subject() {
        let mut store = CapabilityStore::new();
        store.grant(
            Capability::new("alice", "bob", "/data")
                .with_permission(Permission::Read),
        );
        store.grant(
            Capability::new("alice", "bob", "/files")
                .with_permission(Permission::Write),
        );
        store.grant(
            Capability::new("alice", "charlie", "/data")
                .with_permission(Permission::Read),
        );

        assert_eq!(store.for_subject("bob").len(), 2);
        assert_eq!(store.for_subject("charlie").len(), 1);
        assert_eq!(store.for_subject("dave").len(), 0);
    }

    #[test]
    fn capability_store_for_resource() {
        let mut store = CapabilityStore::new();
        store.grant(
            Capability::new("alice", "bob", "/data")
                .with_permission(Permission::Read),
        );
        store.grant(
            Capability::new("alice", "charlie", "/data")
                .with_permission(Permission::Write),
        );

        assert_eq!(store.for_resource("/data").len(), 2);
        assert_eq!(store.for_resource("/files").len(), 0);
    }

    #[test]
    fn capability_store_prune_expired() {
        let mut store = CapabilityStore::new();
        store.grant(
            Capability::new("alice", "bob", "/data")
                .with_permission(Permission::Read)
                .with_expiry(Utc::now() - Duration::hours(1)),
        );
        store.grant(
            Capability::new("alice", "charlie", "/data")
                .with_permission(Permission::Read)
                .with_expiry(Utc::now() + Duration::hours(1)),
        );

        let pruned = store.prune_expired();
        assert_eq!(pruned, 1);
        assert_eq!(store.for_resource("/data").len(), 1);
    }

    #[test]
    fn permission_as_str() {
        assert_eq!(Permission::Read.as_str(), "read");
        assert_eq!(Permission::Admin.as_str(), "admin");
        assert_eq!(Permission::Transfer.as_str(), "transfer");
    }

    #[test]
    fn with_permissions_batch() {
        let cap = Capability::new("alice", "bob", "/dao")
            .with_permissions([Permission::Read, Permission::Vote, Permission::Propose]);

        assert_eq!(cap.permissions.len(), 3);
        assert!(cap.has_permission(Permission::Vote));
    }

    #[test]
    fn empty_permissions_is_invalid() {
        let cap = Capability::new("alice", "bob", "/data");
        assert!(!cap.is_valid());
    }

    #[test]
    fn capability_serializes() {
        let cap = Capability::new("alice", "bob", "/data")
            .with_permission(Permission::Read)
            .with_permission(Permission::Write);

        let json = serde_json::to_string(&cap).unwrap();
        let restored: Capability = serde_json::from_str(&json).unwrap();
        assert_eq!(restored.subject, "bob");
        assert_eq!(restored.permissions.len(), 2);
    }

    #[test]
    fn revoke_nonexistent_returns_false() {
        let mut store = CapabilityStore::new();
        assert!(!store.revoke("cap:nonexistent"));
    }
}
