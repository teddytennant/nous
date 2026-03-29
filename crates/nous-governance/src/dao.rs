use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

use nous_core::{Error, Result};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Member {
    pub did: String,
    pub joined_at: DateTime<Utc>,
    pub credits: u64,
    pub role: MemberRole,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum MemberRole {
    Member,
    Admin,
    Founder,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Dao {
    pub id: String,
    pub name: String,
    pub description: String,
    pub founder_did: String,
    pub created_at: DateTime<Utc>,
    pub members: HashMap<String, Member>,
    pub default_quorum: f64,
    pub default_threshold: f64,
    pub default_credits: u64,
}

impl Dao {
    pub fn create(
        founder_did: impl Into<String>,
        name: impl Into<String>,
        description: impl Into<String>,
    ) -> Self {
        let founder_did = founder_did.into();
        let now = Utc::now();

        let founder = Member {
            did: founder_did.clone(),
            joined_at: now,
            credits: 100,
            role: MemberRole::Founder,
        };

        let mut members = HashMap::new();
        members.insert(founder_did.clone(), founder);

        Self {
            id: format!("dao:{}", Uuid::new_v4()),
            name: name.into(),
            description: description.into(),
            founder_did,
            created_at: now,
            members,
            default_quorum: 0.1,
            default_threshold: 0.5,
            default_credits: 10,
        }
    }

    pub fn add_member(&mut self, did: impl Into<String>) -> Result<()> {
        let did = did.into();
        if self.members.contains_key(&did) {
            return Err(Error::InvalidInput("member already exists".into()));
        }

        self.members.insert(
            did.clone(),
            Member {
                did,
                joined_at: Utc::now(),
                credits: self.default_credits,
                role: MemberRole::Member,
            },
        );
        Ok(())
    }

    pub fn remove_member(&mut self, did: &str) -> Result<()> {
        if did == self.founder_did {
            return Err(Error::PermissionDenied("cannot remove founder".into()));
        }

        self.members
            .remove(did)
            .ok_or_else(|| Error::NotFound("member not found".into()))?;
        Ok(())
    }

    pub fn promote(&mut self, did: &str) -> Result<()> {
        let member = self
            .members
            .get_mut(did)
            .ok_or_else(|| Error::NotFound("member not found".into()))?;
        member.role = MemberRole::Admin;
        Ok(())
    }

    pub fn grant_credits(&mut self, did: &str, amount: u64) -> Result<()> {
        let member = self
            .members
            .get_mut(did)
            .ok_or_else(|| Error::NotFound("member not found".into()))?;
        member.credits += amount;
        Ok(())
    }

    pub fn is_member(&self, did: &str) -> bool {
        self.members.contains_key(did)
    }

    pub fn member_count(&self) -> usize {
        self.members.len()
    }

    pub fn get_member(&self, did: &str) -> Option<&Member> {
        self.members.get(did)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn create_dao() {
        let dao = Dao::create("did:key:zfounder", "TestDAO", "A test DAO");
        assert_eq!(dao.name, "TestDAO");
        assert_eq!(dao.member_count(), 1);
        assert!(dao.is_member("did:key:zfounder"));
        assert!(dao.id.starts_with("dao:"));
    }

    #[test]
    fn founder_has_100_credits() {
        let dao = Dao::create("did:key:zfounder", "Test", "Test");
        let founder = dao.get_member("did:key:zfounder").unwrap();
        assert_eq!(founder.credits, 100);
        assert_eq!(founder.role, MemberRole::Founder);
    }

    #[test]
    fn add_member() {
        let mut dao = Dao::create("did:key:zfounder", "Test", "Test");
        dao.add_member("did:key:zmember").unwrap();

        assert_eq!(dao.member_count(), 2);
        let member = dao.get_member("did:key:zmember").unwrap();
        assert_eq!(member.credits, 10);
        assert_eq!(member.role, MemberRole::Member);
    }

    #[test]
    fn add_duplicate_member_fails() {
        let mut dao = Dao::create("did:key:zfounder", "Test", "Test");
        dao.add_member("did:key:zmember").unwrap();
        assert!(dao.add_member("did:key:zmember").is_err());
    }

    #[test]
    fn remove_member() {
        let mut dao = Dao::create("did:key:zfounder", "Test", "Test");
        dao.add_member("did:key:zmember").unwrap();
        dao.remove_member("did:key:zmember").unwrap();
        assert_eq!(dao.member_count(), 1);
        assert!(!dao.is_member("did:key:zmember"));
    }

    #[test]
    fn cannot_remove_founder() {
        let mut dao = Dao::create("did:key:zfounder", "Test", "Test");
        assert!(dao.remove_member("did:key:zfounder").is_err());
    }

    #[test]
    fn remove_nonexistent_member_fails() {
        let mut dao = Dao::create("did:key:zfounder", "Test", "Test");
        assert!(dao.remove_member("did:key:zghost").is_err());
    }

    #[test]
    fn promote_member() {
        let mut dao = Dao::create("did:key:zfounder", "Test", "Test");
        dao.add_member("did:key:zmember").unwrap();
        dao.promote("did:key:zmember").unwrap();

        let member = dao.get_member("did:key:zmember").unwrap();
        assert_eq!(member.role, MemberRole::Admin);
    }

    #[test]
    fn grant_credits() {
        let mut dao = Dao::create("did:key:zfounder", "Test", "Test");
        dao.add_member("did:key:zmember").unwrap();
        dao.grant_credits("did:key:zmember", 50).unwrap();

        let member = dao.get_member("did:key:zmember").unwrap();
        assert_eq!(member.credits, 60); // 10 default + 50
    }

    #[test]
    fn dao_serializes() {
        let mut dao = Dao::create("did:key:zfounder", "SerdeDAO", "Serialization test");
        dao.add_member("did:key:zmember").unwrap();

        let json = serde_json::to_string(&dao).unwrap();
        let deserialized: Dao = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.name, "SerdeDAO");
        assert_eq!(deserialized.member_count(), 2);
    }
}
