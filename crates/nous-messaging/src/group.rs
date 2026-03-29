use std::collections::{HashMap, HashSet};

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use nous_core::{Error, Result};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum GroupRole {
    Member,
    Moderator,
    Admin,
    Owner,
}

impl GroupRole {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Member => "member",
            Self::Moderator => "moderator",
            Self::Admin => "admin",
            Self::Owner => "owner",
        }
    }

    pub fn can_invite(&self) -> bool {
        *self >= Self::Moderator
    }

    pub fn can_kick(&self) -> bool {
        *self >= Self::Moderator
    }

    pub fn can_promote(&self) -> bool {
        *self >= Self::Admin
    }

    pub fn can_configure(&self) -> bool {
        *self >= Self::Admin
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GroupMember {
    pub did: String,
    pub role: GroupRole,
    pub joined_at: DateTime<Utc>,
    pub invited_by: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum JoinPolicy {
    Open,
    InviteOnly,
    ApprovalRequired,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GroupSettings {
    pub join_policy: JoinPolicy,
    pub max_members: usize,
    pub allow_member_invites: bool,
    pub read_only: bool,
}

impl Default for GroupSettings {
    fn default() -> Self {
        Self {
            join_policy: JoinPolicy::InviteOnly,
            max_members: 256,
            allow_member_invites: false,
            read_only: false,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Group {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    pub members: HashMap<String, GroupMember>,
    pub settings: GroupSettings,
    pub created_at: DateTime<Utc>,
    pub pending_joins: HashSet<String>,
}

impl Group {
    pub fn create(owner_did: &str, name: impl Into<String>) -> Self {
        let mut members = HashMap::new();
        members.insert(
            owner_did.to_string(),
            GroupMember {
                did: owner_did.into(),
                role: GroupRole::Owner,
                joined_at: Utc::now(),
                invited_by: None,
            },
        );

        Self {
            id: format!("group:{}", Uuid::new_v4()),
            name: name.into(),
            description: None,
            members,
            settings: GroupSettings::default(),
            created_at: Utc::now(),
            pending_joins: HashSet::new(),
        }
    }

    pub fn with_description(mut self, desc: impl Into<String>) -> Self {
        self.description = Some(desc.into());
        self
    }

    pub fn with_settings(mut self, settings: GroupSettings) -> Self {
        self.settings = settings;
        self
    }

    pub fn member_count(&self) -> usize {
        self.members.len()
    }

    pub fn is_member(&self, did: &str) -> bool {
        self.members.contains_key(did)
    }

    pub fn role_of(&self, did: &str) -> Option<GroupRole> {
        self.members.get(did).map(|m| m.role)
    }

    pub fn invite(&mut self, inviter: &str, invitee: &str) -> Result<()> {
        let inviter_role = self
            .role_of(inviter)
            .ok_or_else(|| Error::PermissionDenied("inviter is not a member".into()))?;

        if !inviter_role.can_invite() && !self.settings.allow_member_invites {
            return Err(Error::PermissionDenied(
                "insufficient role to invite".into(),
            ));
        }

        if self.is_member(invitee) {
            return Err(Error::InvalidInput("already a member".into()));
        }

        if self.members.len() >= self.settings.max_members {
            return Err(Error::InvalidInput("group is full".into()));
        }

        self.members.insert(
            invitee.to_string(),
            GroupMember {
                did: invitee.into(),
                role: GroupRole::Member,
                joined_at: Utc::now(),
                invited_by: Some(inviter.into()),
            },
        );

        Ok(())
    }

    pub fn request_join(&mut self, did: &str) -> Result<()> {
        if self.is_member(did) {
            return Err(Error::InvalidInput("already a member".into()));
        }
        match self.settings.join_policy {
            JoinPolicy::Open => {
                self.members.insert(
                    did.to_string(),
                    GroupMember {
                        did: did.into(),
                        role: GroupRole::Member,
                        joined_at: Utc::now(),
                        invited_by: None,
                    },
                );
                Ok(())
            }
            JoinPolicy::ApprovalRequired => {
                self.pending_joins.insert(did.to_string());
                Ok(())
            }
            JoinPolicy::InviteOnly => Err(Error::PermissionDenied("group is invite only".into())),
        }
    }

    pub fn approve_join(&mut self, approver: &str, applicant: &str) -> Result<()> {
        let role = self
            .role_of(approver)
            .ok_or_else(|| Error::PermissionDenied("not a member".into()))?;

        if !role.can_invite() {
            return Err(Error::PermissionDenied(
                "insufficient role to approve".into(),
            ));
        }

        if !self.pending_joins.remove(applicant) {
            return Err(Error::NotFound("no pending join request".into()));
        }

        self.members.insert(
            applicant.to_string(),
            GroupMember {
                did: applicant.into(),
                role: GroupRole::Member,
                joined_at: Utc::now(),
                invited_by: Some(approver.into()),
            },
        );

        Ok(())
    }

    pub fn kick(&mut self, kicker: &str, target: &str) -> Result<()> {
        let kicker_role = self
            .role_of(kicker)
            .ok_or_else(|| Error::PermissionDenied("kicker is not a member".into()))?;

        let target_role = self
            .role_of(target)
            .ok_or_else(|| Error::NotFound("target is not a member".into()))?;

        if !kicker_role.can_kick() {
            return Err(Error::PermissionDenied("insufficient role to kick".into()));
        }

        if target_role >= kicker_role {
            return Err(Error::PermissionDenied(
                "cannot kick member with equal or higher role".into(),
            ));
        }

        self.members.remove(target);
        Ok(())
    }

    pub fn leave(&mut self, did: &str) -> Result<()> {
        let role = self
            .role_of(did)
            .ok_or_else(|| Error::NotFound("not a member".into()))?;

        if role == GroupRole::Owner {
            let admin_count = self
                .members
                .values()
                .filter(|m| m.role >= GroupRole::Admin && m.did != did)
                .count();
            if admin_count == 0 && self.member_count() > 1 {
                return Err(Error::InvalidInput(
                    "owner must transfer ownership before leaving".into(),
                ));
            }
        }

        self.members.remove(did);
        Ok(())
    }

    pub fn promote(&mut self, promoter: &str, target: &str, new_role: GroupRole) -> Result<()> {
        let promoter_role = self
            .role_of(promoter)
            .ok_or_else(|| Error::PermissionDenied("not a member".into()))?;

        if !promoter_role.can_promote() {
            return Err(Error::PermissionDenied(
                "insufficient role to promote".into(),
            ));
        }

        if new_role >= promoter_role {
            return Err(Error::PermissionDenied(
                "cannot promote to equal or higher role".into(),
            ));
        }

        let member = self
            .members
            .get_mut(target)
            .ok_or_else(|| Error::NotFound("target is not a member".into()))?;

        member.role = new_role;
        Ok(())
    }

    pub fn transfer_ownership(&mut self, owner: &str, new_owner: &str) -> Result<()> {
        if self.role_of(owner) != Some(GroupRole::Owner) {
            return Err(Error::PermissionDenied("not the owner".into()));
        }

        if !self.is_member(new_owner) {
            return Err(Error::NotFound("new owner is not a member".into()));
        }

        self.members.get_mut(owner).unwrap().role = GroupRole::Admin;
        self.members.get_mut(new_owner).unwrap().role = GroupRole::Owner;
        Ok(())
    }

    pub fn members_with_role(&self, role: GroupRole) -> Vec<&GroupMember> {
        self.members.values().filter(|m| m.role == role).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn create_group() {
        let group = Group::create("alice", "Engineering");
        assert!(group.id.starts_with("group:"));
        assert_eq!(group.name, "Engineering");
        assert_eq!(group.member_count(), 1);
        assert_eq!(group.role_of("alice"), Some(GroupRole::Owner));
    }

    #[test]
    fn invite_member() {
        let mut group = Group::create("alice", "Test");
        group.invite("alice", "bob").unwrap();
        assert!(group.is_member("bob"));
        assert_eq!(group.role_of("bob"), Some(GroupRole::Member));
        assert_eq!(group.member_count(), 2);
    }

    #[test]
    fn invite_duplicate_fails() {
        let mut group = Group::create("alice", "Test");
        group.invite("alice", "bob").unwrap();
        assert!(group.invite("alice", "bob").is_err());
    }

    #[test]
    fn member_cannot_invite_by_default() {
        let mut group = Group::create("alice", "Test");
        group.invite("alice", "bob").unwrap();
        assert!(group.invite("bob", "charlie").is_err());
    }

    #[test]
    fn member_can_invite_when_allowed() {
        let mut group = Group::create("alice", "Test");
        group.settings.allow_member_invites = true;
        group.invite("alice", "bob").unwrap();
        group.invite("bob", "charlie").unwrap();
        assert!(group.is_member("charlie"));
    }

    #[test]
    fn non_member_cannot_invite() {
        let mut group = Group::create("alice", "Test");
        assert!(group.invite("stranger", "bob").is_err());
    }

    #[test]
    fn group_full_rejects_invite() {
        let mut group = Group::create("alice", "Test");
        group.settings.max_members = 2;
        group.invite("alice", "bob").unwrap();
        assert!(group.invite("alice", "charlie").is_err());
    }

    #[test]
    fn kick_member() {
        let mut group = Group::create("alice", "Test");
        group.invite("alice", "bob").unwrap();
        group.kick("alice", "bob").unwrap();
        assert!(!group.is_member("bob"));
    }

    #[test]
    fn member_cannot_kick() {
        let mut group = Group::create("alice", "Test");
        group.invite("alice", "bob").unwrap();
        group.invite("alice", "charlie").unwrap();
        assert!(group.kick("bob", "charlie").is_err());
    }

    #[test]
    fn cannot_kick_equal_or_higher_role() {
        let mut group = Group::create("alice", "Test");
        group.invite("alice", "bob").unwrap();
        group.promote("alice", "bob", GroupRole::Moderator).unwrap();
        // Moderator cannot kick another moderator
        group.invite("alice", "carol").unwrap();
        group
            .promote("alice", "carol", GroupRole::Moderator)
            .unwrap();
        assert!(group.kick("bob", "carol").is_err());
    }

    #[test]
    fn leave_group() {
        let mut group = Group::create("alice", "Test");
        group.invite("alice", "bob").unwrap();
        group.leave("bob").unwrap();
        assert!(!group.is_member("bob"));
    }

    #[test]
    fn owner_cannot_leave_without_transfer() {
        let mut group = Group::create("alice", "Test");
        group.invite("alice", "bob").unwrap();
        assert!(group.leave("alice").is_err());
    }

    #[test]
    fn owner_can_leave_solo_group() {
        let mut group = Group::create("alice", "Test");
        group.leave("alice").unwrap();
        assert_eq!(group.member_count(), 0);
    }

    #[test]
    fn promote_member() {
        let mut group = Group::create("alice", "Test");
        group.invite("alice", "bob").unwrap();
        group.promote("alice", "bob", GroupRole::Moderator).unwrap();
        assert_eq!(group.role_of("bob"), Some(GroupRole::Moderator));
    }

    #[test]
    fn cannot_promote_to_equal_role() {
        let mut group = Group::create("alice", "Test");
        group.invite("alice", "bob").unwrap();
        // Admin can't promote to Admin
        group.promote("alice", "bob", GroupRole::Admin).unwrap();
        group.invite("alice", "charlie").unwrap();
        assert!(group.promote("bob", "charlie", GroupRole::Admin).is_err());
    }

    #[test]
    fn transfer_ownership() {
        let mut group = Group::create("alice", "Test");
        group.invite("alice", "bob").unwrap();
        group.transfer_ownership("alice", "bob").unwrap();

        assert_eq!(group.role_of("bob"), Some(GroupRole::Owner));
        assert_eq!(group.role_of("alice"), Some(GroupRole::Admin));
    }

    #[test]
    fn non_owner_cannot_transfer() {
        let mut group = Group::create("alice", "Test");
        group.invite("alice", "bob").unwrap();
        assert!(group.transfer_ownership("bob", "alice").is_err());
    }

    #[test]
    fn open_join_policy() {
        let mut group = Group::create("alice", "Test");
        group.settings.join_policy = JoinPolicy::Open;
        group.request_join("bob").unwrap();
        assert!(group.is_member("bob"));
    }

    #[test]
    fn invite_only_rejects_request() {
        let mut group = Group::create("alice", "Test");
        assert!(group.request_join("bob").is_err());
    }

    #[test]
    fn approval_required_flow() {
        let mut group = Group::create("alice", "Test");
        group.settings.join_policy = JoinPolicy::ApprovalRequired;

        group.request_join("bob").unwrap();
        assert!(!group.is_member("bob"));
        assert!(group.pending_joins.contains("bob"));

        group.approve_join("alice", "bob").unwrap();
        assert!(group.is_member("bob"));
        assert!(!group.pending_joins.contains("bob"));
    }

    #[test]
    fn approve_nonexistent_request_fails() {
        let mut group = Group::create("alice", "Test");
        assert!(group.approve_join("alice", "nobody").is_err());
    }

    #[test]
    fn members_with_role() {
        let mut group = Group::create("alice", "Test");
        group.invite("alice", "bob").unwrap();
        group.invite("alice", "carol").unwrap();
        group.promote("alice", "bob", GroupRole::Moderator).unwrap();

        assert_eq!(group.members_with_role(GroupRole::Owner).len(), 1);
        assert_eq!(group.members_with_role(GroupRole::Moderator).len(), 1);
        assert_eq!(group.members_with_role(GroupRole::Member).len(), 1);
    }

    #[test]
    fn role_as_str() {
        assert_eq!(GroupRole::Member.as_str(), "member");
        assert_eq!(GroupRole::Owner.as_str(), "owner");
    }

    #[test]
    fn role_permissions() {
        assert!(!GroupRole::Member.can_invite());
        assert!(GroupRole::Moderator.can_invite());
        assert!(GroupRole::Moderator.can_kick());
        assert!(!GroupRole::Moderator.can_promote());
        assert!(GroupRole::Admin.can_promote());
        assert!(GroupRole::Admin.can_configure());
    }

    #[test]
    fn group_serializes() {
        let mut group = Group::create("alice", "Test");
        group.invite("alice", "bob").unwrap();
        let json = serde_json::to_string(&group).unwrap();
        let restored: Group = serde_json::from_str(&json).unwrap();
        assert_eq!(restored.name, "Test");
        assert!(restored.is_member("bob"));
    }

    #[test]
    fn with_description() {
        let group = Group::create("alice", "Test").with_description("A test group");
        assert_eq!(group.description.as_deref(), Some("A test group"));
    }
}
