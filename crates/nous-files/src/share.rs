use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};

use nous_core::{Error, Result};

use crate::chunk::ContentId;

/// Access level for shared folder members.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum AccessLevel {
    /// Can read files.
    Read,
    /// Can read and write files.
    Write,
    /// Can read, write, and manage members.
    Admin,
    /// Full control including deletion and transfer.
    Owner,
}

impl AccessLevel {
    pub fn can_read(self) -> bool {
        true // All levels can read.
    }

    pub fn can_write(self) -> bool {
        matches!(self, Self::Write | Self::Admin | Self::Owner)
    }

    pub fn can_manage_members(self) -> bool {
        matches!(self, Self::Admin | Self::Owner)
    }

    pub fn can_delete(self) -> bool {
        matches!(self, Self::Owner)
    }
}

/// A shared folder with DID-based access control.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SharedFolder {
    /// Unique folder identifier.
    pub id: String,
    /// Human-readable name.
    pub name: String,
    /// Owner DID.
    pub owner: String,
    /// Members mapped to their access level.
    members: HashMap<String, AccessLevel>,
    /// File manifest IDs in this folder.
    files: HashSet<ContentId>,
    /// Subfolder IDs.
    subfolders: HashSet<String>,
    /// Creation timestamp.
    pub created_at: chrono::DateTime<chrono::Utc>,
    /// Last modified timestamp.
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

/// An invitation to join a shared folder.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FolderInvite {
    pub folder_id: String,
    pub folder_name: String,
    pub inviter: String,
    pub invitee: String,
    pub access_level: AccessLevel,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub expires_at: Option<chrono::DateTime<chrono::Utc>>,
}

/// An audit log entry for folder operations.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditEntry {
    pub actor: String,
    pub action: AuditAction,
    pub target: String,
    pub timestamp: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AuditAction {
    AddFile,
    RemoveFile,
    AddMember,
    RemoveMember,
    ChangeAccess,
    CreateSubfolder,
    RenameFolder,
}

impl SharedFolder {
    /// Create a new shared folder owned by the given DID.
    pub fn new(name: &str, owner_did: &str) -> Self {
        let id = uuid::Uuid::new_v4().to_string();
        let now = chrono::Utc::now();

        let mut members = HashMap::new();
        members.insert(owner_did.to_string(), AccessLevel::Owner);

        Self {
            id,
            name: name.to_string(),
            owner: owner_did.to_string(),
            members,
            files: HashSet::new(),
            subfolders: HashSet::new(),
            created_at: now,
            updated_at: now,
        }
    }

    /// Check if a DID has the given access level or higher.
    pub fn check_access(&self, did: &str, required: AccessLevel) -> Result<()> {
        let level = self
            .members
            .get(did)
            .ok_or_else(|| Error::PermissionDenied(format!("{did} is not a member")))?;

        if *level >= required {
            Ok(())
        } else {
            Err(Error::PermissionDenied(format!(
                "{did} has {level:?} access, needs {required:?}"
            )))
        }
    }

    /// Get a member's access level.
    pub fn get_access(&self, did: &str) -> Option<AccessLevel> {
        self.members.get(did).copied()
    }

    /// Add a member with the specified access level.
    /// Requires Admin or Owner access from the actor.
    pub fn add_member(
        &mut self,
        actor_did: &str,
        member_did: &str,
        level: AccessLevel,
    ) -> Result<AuditEntry> {
        self.check_access(actor_did, AccessLevel::Admin)?;

        if level == AccessLevel::Owner {
            return Err(Error::PermissionDenied(
                "cannot grant owner access — use transfer_ownership".into(),
            ));
        }

        self.members.insert(member_did.to_string(), level);
        self.updated_at = chrono::Utc::now();

        Ok(AuditEntry {
            actor: actor_did.to_string(),
            action: AuditAction::AddMember,
            target: member_did.to_string(),
            timestamp: self.updated_at,
        })
    }

    /// Remove a member from the folder.
    pub fn remove_member(&mut self, actor_did: &str, member_did: &str) -> Result<AuditEntry> {
        self.check_access(actor_did, AccessLevel::Admin)?;

        if member_did == self.owner {
            return Err(Error::PermissionDenied("cannot remove the owner".into()));
        }

        if self.members.remove(member_did).is_none() {
            return Err(Error::NotFound(format!("{member_did} is not a member")));
        }

        self.updated_at = chrono::Utc::now();

        Ok(AuditEntry {
            actor: actor_did.to_string(),
            action: AuditAction::RemoveMember,
            target: member_did.to_string(),
            timestamp: self.updated_at,
        })
    }

    /// Change a member's access level.
    pub fn change_access(
        &mut self,
        actor_did: &str,
        member_did: &str,
        new_level: AccessLevel,
    ) -> Result<AuditEntry> {
        self.check_access(actor_did, AccessLevel::Admin)?;

        if member_did == self.owner {
            return Err(Error::PermissionDenied(
                "cannot change owner's access level".into(),
            ));
        }

        if new_level == AccessLevel::Owner {
            return Err(Error::PermissionDenied(
                "cannot grant owner access — use transfer_ownership".into(),
            ));
        }

        if !self.members.contains_key(member_did) {
            return Err(Error::NotFound(format!("{member_did} is not a member")));
        }

        self.members.insert(member_did.to_string(), new_level);
        self.updated_at = chrono::Utc::now();

        Ok(AuditEntry {
            actor: actor_did.to_string(),
            action: AuditAction::ChangeAccess,
            target: member_did.to_string(),
            timestamp: self.updated_at,
        })
    }

    /// Transfer ownership to another member. Only the current owner can do this.
    pub fn transfer_ownership(&mut self, new_owner_did: &str) -> Result<()> {
        if !self.members.contains_key(new_owner_did) {
            return Err(Error::NotFound(format!(
                "{new_owner_did} must be a member first"
            )));
        }

        // Demote current owner to Admin.
        self.members.insert(self.owner.clone(), AccessLevel::Admin);
        // Promote new owner.
        self.members
            .insert(new_owner_did.to_string(), AccessLevel::Owner);
        self.owner = new_owner_did.to_string();
        self.updated_at = chrono::Utc::now();

        Ok(())
    }

    /// Add a file to the folder.
    pub fn add_file(&mut self, actor_did: &str, file_id: ContentId) -> Result<AuditEntry> {
        self.check_access(actor_did, AccessLevel::Write)?;
        self.files.insert(file_id.clone());
        self.updated_at = chrono::Utc::now();

        Ok(AuditEntry {
            actor: actor_did.to_string(),
            action: AuditAction::AddFile,
            target: file_id.to_string(),
            timestamp: self.updated_at,
        })
    }

    /// Remove a file from the folder.
    pub fn remove_file(&mut self, actor_did: &str, file_id: &ContentId) -> Result<AuditEntry> {
        self.check_access(actor_did, AccessLevel::Write)?;

        if !self.files.remove(file_id) {
            return Err(Error::NotFound(format!("file {file_id} not in folder")));
        }

        self.updated_at = chrono::Utc::now();

        Ok(AuditEntry {
            actor: actor_did.to_string(),
            action: AuditAction::RemoveFile,
            target: file_id.to_string(),
            timestamp: self.updated_at,
        })
    }

    /// List all file IDs in this folder.
    pub fn list_files(&self) -> Vec<&ContentId> {
        self.files.iter().collect()
    }

    /// List all members and their access levels.
    pub fn list_members(&self) -> Vec<(&str, AccessLevel)> {
        self.members
            .iter()
            .map(|(did, level)| (did.as_str(), *level))
            .collect()
    }

    /// Get the number of files.
    pub fn file_count(&self) -> usize {
        self.files.len()
    }

    /// Get the number of members.
    pub fn member_count(&self) -> usize {
        self.members.len()
    }

    /// Create an invite for a new member.
    pub fn create_invite(
        &self,
        inviter_did: &str,
        invitee_did: &str,
        level: AccessLevel,
    ) -> Result<FolderInvite> {
        self.check_access(inviter_did, AccessLevel::Admin)?;

        Ok(FolderInvite {
            folder_id: self.id.clone(),
            folder_name: self.name.clone(),
            inviter: inviter_did.to_string(),
            invitee: invitee_did.to_string(),
            access_level: level,
            created_at: chrono::Utc::now(),
            expires_at: Some(chrono::Utc::now() + chrono::Duration::days(7)),
        })
    }

    /// Rename the folder.
    pub fn rename(&mut self, actor_did: &str, new_name: &str) -> Result<AuditEntry> {
        self.check_access(actor_did, AccessLevel::Admin)?;
        self.name = new_name.to_string();
        self.updated_at = chrono::Utc::now();

        Ok(AuditEntry {
            actor: actor_did.to_string(),
            action: AuditAction::RenameFolder,
            target: new_name.to_string(),
            timestamp: self.updated_at,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const OWNER: &str = "did:key:zOwner";
    const ALICE: &str = "did:key:zAlice";
    const BOB: &str = "did:key:zBob";

    fn test_folder() -> SharedFolder {
        SharedFolder::new("test-folder", OWNER)
    }

    #[test]
    fn create_folder() {
        let folder = test_folder();
        assert_eq!(folder.name, "test-folder");
        assert_eq!(folder.owner, OWNER);
        assert_eq!(folder.member_count(), 1);
        assert_eq!(folder.get_access(OWNER), Some(AccessLevel::Owner));
    }

    #[test]
    fn add_member() {
        let mut folder = test_folder();
        let entry = folder.add_member(OWNER, ALICE, AccessLevel::Write).unwrap();
        assert_eq!(entry.actor, OWNER);
        assert_eq!(folder.member_count(), 2);
        assert_eq!(folder.get_access(ALICE), Some(AccessLevel::Write));
    }

    #[test]
    fn add_member_requires_admin() {
        let mut folder = test_folder();
        folder.add_member(OWNER, ALICE, AccessLevel::Read).unwrap();
        // Read-level member cannot add others.
        assert!(folder.add_member(ALICE, BOB, AccessLevel::Read).is_err());
    }

    #[test]
    fn cannot_grant_owner_via_add_member() {
        let mut folder = test_folder();
        assert!(folder.add_member(OWNER, ALICE, AccessLevel::Owner).is_err());
    }

    #[test]
    fn remove_member() {
        let mut folder = test_folder();
        folder.add_member(OWNER, ALICE, AccessLevel::Write).unwrap();
        folder.remove_member(OWNER, ALICE).unwrap();
        assert_eq!(folder.member_count(), 1);
        assert!(folder.get_access(ALICE).is_none());
    }

    #[test]
    fn cannot_remove_owner() {
        let mut folder = test_folder();
        assert!(folder.remove_member(OWNER, OWNER).is_err());
    }

    #[test]
    fn remove_nonexistent_member() {
        let mut folder = test_folder();
        assert!(folder.remove_member(OWNER, "did:key:zNobody").is_err());
    }

    #[test]
    fn change_access() {
        let mut folder = test_folder();
        folder.add_member(OWNER, ALICE, AccessLevel::Read).unwrap();
        folder
            .change_access(OWNER, ALICE, AccessLevel::Admin)
            .unwrap();
        assert_eq!(folder.get_access(ALICE), Some(AccessLevel::Admin));
    }

    #[test]
    fn cannot_change_owner_access() {
        let mut folder = test_folder();
        assert!(
            folder
                .change_access(OWNER, OWNER, AccessLevel::Admin)
                .is_err()
        );
    }

    #[test]
    fn transfer_ownership() {
        let mut folder = test_folder();
        folder.add_member(OWNER, ALICE, AccessLevel::Admin).unwrap();
        folder.transfer_ownership(ALICE).unwrap();

        assert_eq!(folder.owner, ALICE);
        assert_eq!(folder.get_access(ALICE), Some(AccessLevel::Owner));
        assert_eq!(folder.get_access(OWNER), Some(AccessLevel::Admin));
    }

    #[test]
    fn transfer_ownership_requires_membership() {
        let mut folder = test_folder();
        assert!(folder.transfer_ownership("did:key:zStranger").is_err());
    }

    #[test]
    fn add_and_remove_file() {
        let mut folder = test_folder();
        let file_id = ContentId("abc123".into());

        folder.add_file(OWNER, file_id.clone()).unwrap();
        assert_eq!(folder.file_count(), 1);

        folder.remove_file(OWNER, &file_id).unwrap();
        assert_eq!(folder.file_count(), 0);
    }

    #[test]
    fn add_file_requires_write() {
        let mut folder = test_folder();
        folder.add_member(OWNER, ALICE, AccessLevel::Read).unwrap();
        assert!(folder.add_file(ALICE, ContentId("x".into())).is_err());
    }

    #[test]
    fn remove_nonexistent_file() {
        let mut folder = test_folder();
        assert!(
            folder
                .remove_file(OWNER, &ContentId("nope".into()))
                .is_err()
        );
    }

    #[test]
    fn access_level_ordering() {
        assert!(AccessLevel::Read < AccessLevel::Write);
        assert!(AccessLevel::Write < AccessLevel::Admin);
        assert!(AccessLevel::Admin < AccessLevel::Owner);
    }

    #[test]
    fn access_level_permissions() {
        assert!(AccessLevel::Read.can_read());
        assert!(!AccessLevel::Read.can_write());
        assert!(!AccessLevel::Read.can_manage_members());
        assert!(!AccessLevel::Read.can_delete());

        assert!(AccessLevel::Write.can_write());
        assert!(!AccessLevel::Write.can_manage_members());

        assert!(AccessLevel::Admin.can_manage_members());
        assert!(!AccessLevel::Admin.can_delete());

        assert!(AccessLevel::Owner.can_delete());
    }

    #[test]
    fn create_invite() {
        let folder = test_folder();
        let invite = folder
            .create_invite(OWNER, ALICE, AccessLevel::Write)
            .unwrap();
        assert_eq!(invite.folder_id, folder.id);
        assert_eq!(invite.invitee, ALICE);
        assert_eq!(invite.access_level, AccessLevel::Write);
        assert!(invite.expires_at.is_some());
    }

    #[test]
    fn rename_folder() {
        let mut folder = test_folder();
        folder.rename(OWNER, "new-name").unwrap();
        assert_eq!(folder.name, "new-name");
    }

    #[test]
    fn rename_requires_admin() {
        let mut folder = test_folder();
        folder.add_member(OWNER, ALICE, AccessLevel::Write).unwrap();
        assert!(folder.rename(ALICE, "x").is_err());
    }

    #[test]
    fn folder_serde_roundtrip() {
        let mut folder = test_folder();
        folder.add_member(OWNER, ALICE, AccessLevel::Write).unwrap();
        folder.add_file(OWNER, ContentId("file1".into())).unwrap();

        let json = serde_json::to_string(&folder).unwrap();
        let deserialized: SharedFolder = serde_json::from_str(&json).unwrap();

        assert_eq!(deserialized.name, folder.name);
        assert_eq!(deserialized.member_count(), folder.member_count());
        assert_eq!(deserialized.file_count(), folder.file_count());
    }

    #[test]
    fn admin_can_add_members() {
        let mut folder = test_folder();
        folder.add_member(OWNER, ALICE, AccessLevel::Admin).unwrap();
        folder.add_member(ALICE, BOB, AccessLevel::Read).unwrap();
        assert_eq!(folder.member_count(), 3);
    }
}
