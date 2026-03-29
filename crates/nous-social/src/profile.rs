use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Profile {
    pub did: String,
    pub display_name: Option<String>,
    pub bio: Option<String>,
    pub avatar_hash: Option<String>,
    pub following: Vec<String>,
    pub followers: Vec<String>,
}

impl Profile {
    pub fn new(did: impl Into<String>) -> Self {
        Self {
            did: did.into(),
            display_name: None,
            bio: None,
            avatar_hash: None,
            following: Vec::new(),
            followers: Vec::new(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn create_profile() {
        let profile = Profile::new("did:key:ztest");
        assert_eq!(profile.did, "did:key:ztest");
        assert!(profile.display_name.is_none());
    }
}
