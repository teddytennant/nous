use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ChannelKind {
    DirectMessage,
    Group,
    Public,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Channel {
    pub id: String,
    pub kind: ChannelKind,
    pub name: Option<String>,
    pub members: Vec<String>,
    pub created_at: DateTime<Utc>,
    pub created_by: String,
}

impl Channel {
    pub fn direct(creator_did: &str, peer_did: &str) -> Self {
        let mut members = vec![creator_did.to_string(), peer_did.to_string()];
        members.sort();

        Self {
            id: format!("dm:{}", Uuid::new_v4()),
            kind: ChannelKind::DirectMessage,
            name: None,
            members,
            created_at: Utc::now(),
            created_by: creator_did.to_string(),
        }
    }

    pub fn group(creator_did: &str, name: impl Into<String>, members: Vec<String>) -> Self {
        let mut all_members = members;
        if !all_members.contains(&creator_did.to_string()) {
            all_members.push(creator_did.to_string());
        }

        Self {
            id: format!("grp:{}", Uuid::new_v4()),
            kind: ChannelKind::Group,
            name: Some(name.into()),
            members: all_members,
            created_at: Utc::now(),
            created_by: creator_did.to_string(),
        }
    }

    pub fn public(creator_did: &str, name: impl Into<String>) -> Self {
        Self {
            id: format!("pub:{}", Uuid::new_v4()),
            kind: ChannelKind::Public,
            name: Some(name.into()),
            members: vec![creator_did.to_string()],
            created_at: Utc::now(),
            created_by: creator_did.to_string(),
        }
    }

    pub fn is_member(&self, did: &str) -> bool {
        self.kind == ChannelKind::Public || self.members.iter().any(|m| m == did)
    }

    pub fn add_member(&mut self, did: impl Into<String>) {
        let did = did.into();
        if !self.members.contains(&did) {
            self.members.push(did);
        }
    }

    pub fn remove_member(&mut self, did: &str) -> bool {
        let len_before = self.members.len();
        self.members.retain(|m| m != did);
        self.members.len() < len_before
    }

    pub fn member_count(&self) -> usize {
        self.members.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn create_direct_message() {
        let ch = Channel::direct("did:key:zalice", "did:key:zbob");
        assert_eq!(ch.kind, ChannelKind::DirectMessage);
        assert_eq!(ch.member_count(), 2);
        assert!(ch.is_member("did:key:zalice"));
        assert!(ch.is_member("did:key:zbob"));
        assert!(ch.id.starts_with("dm:"));
    }

    #[test]
    fn dm_members_sorted() {
        let ch = Channel::direct("did:key:zbob", "did:key:zalice");
        assert_eq!(ch.members[0], "did:key:zalice");
        assert_eq!(ch.members[1], "did:key:zbob");
    }

    #[test]
    fn create_group() {
        let ch = Channel::group(
            "did:key:zadmin",
            "engineering",
            vec!["did:key:zdev1".into(), "did:key:zdev2".into()],
        );
        assert_eq!(ch.kind, ChannelKind::Group);
        assert_eq!(ch.name.as_deref(), Some("engineering"));
        assert_eq!(ch.member_count(), 3);
        assert!(ch.id.starts_with("grp:"));
    }

    #[test]
    fn group_includes_creator() {
        let ch = Channel::group("did:key:zcreator", "test", vec![]);
        assert!(ch.is_member("did:key:zcreator"));
    }

    #[test]
    fn create_public_channel() {
        let ch = Channel::public("did:key:zadmin", "general");
        assert_eq!(ch.kind, ChannelKind::Public);
        // public channels allow anyone
        assert!(ch.is_member("did:key:zanyone"));
        assert!(ch.id.starts_with("pub:"));
    }

    #[test]
    fn add_and_remove_member() {
        let mut ch = Channel::group("did:key:za", "test", vec![]);
        ch.add_member("did:key:zb");
        assert_eq!(ch.member_count(), 2);

        // no duplicates
        ch.add_member("did:key:zb");
        assert_eq!(ch.member_count(), 2);

        assert!(ch.remove_member("did:key:zb"));
        assert_eq!(ch.member_count(), 1);
        assert!(!ch.is_member("did:key:zb"));
    }

    #[test]
    fn remove_nonexistent_member() {
        let mut ch = Channel::group("did:key:za", "test", vec![]);
        assert!(!ch.remove_member("did:key:znonexistent"));
    }

    #[test]
    fn channel_serializes() {
        let ch = Channel::group("did:key:za", "test", vec!["did:key:zb".into()]);
        let json = serde_json::to_string(&ch).unwrap();
        let deserialized: Channel = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.id, ch.id);
        assert_eq!(deserialized.kind, ChannelKind::Group);
    }
}
