use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Profile {
    pub did: String,
    pub display_name: Option<String>,
    pub bio: Option<String>,
    pub avatar_cid: Option<String>,
    pub banner_cid: Option<String>,
    pub website: Option<String>,
    pub location: Option<String>,
    pub updated_at: DateTime<Utc>,
    pub metadata: std::collections::HashMap<String, String>,
}

impl Profile {
    pub fn new(did: impl Into<String>) -> Self {
        Self {
            did: did.into(),
            display_name: None,
            bio: None,
            avatar_cid: None,
            banner_cid: None,
            website: None,
            location: None,
            updated_at: Utc::now(),
            metadata: std::collections::HashMap::new(),
        }
    }

    pub fn with_name(mut self, name: impl Into<String>) -> Self {
        self.display_name = Some(name.into());
        self.updated_at = Utc::now();
        self
    }

    pub fn with_bio(mut self, bio: impl Into<String>) -> Self {
        self.bio = Some(bio.into());
        self.updated_at = Utc::now();
        self
    }

    pub fn with_avatar(mut self, cid: impl Into<String>) -> Self {
        self.avatar_cid = Some(cid.into());
        self.updated_at = Utc::now();
        self
    }

    pub fn with_website(mut self, url: impl Into<String>) -> Self {
        self.website = Some(url.into());
        self.updated_at = Utc::now();
        self
    }

    pub fn set_metadata(&mut self, key: impl Into<String>, value: impl Into<String>) {
        self.metadata.insert(key.into(), value.into());
        self.updated_at = Utc::now();
    }

    pub fn to_event_content(&self) -> String {
        serde_json::to_string(self).unwrap_or_default()
    }

    pub fn from_event_content(content: &str) -> Result<Self, serde_json::Error> {
        serde_json::from_str(content)
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

    #[test]
    fn builder_pattern() {
        let profile = Profile::new("did:key:z1")
            .with_name("Alice")
            .with_bio("Sovereign individual")
            .with_website("https://alice.eth");

        assert_eq!(profile.display_name.as_deref(), Some("Alice"));
        assert_eq!(profile.bio.as_deref(), Some("Sovereign individual"));
        assert_eq!(profile.website.as_deref(), Some("https://alice.eth"));
    }

    #[test]
    fn metadata() {
        let mut profile = Profile::new("did:key:z1");
        profile.set_metadata("twitter", "@alice");
        assert_eq!(profile.metadata.get("twitter").unwrap(), "@alice");
    }

    #[test]
    fn event_content_roundtrip() {
        let profile = Profile::new("did:key:z1")
            .with_name("Alice")
            .with_bio("test");

        let content = profile.to_event_content();
        let restored = Profile::from_event_content(&content).unwrap();
        assert_eq!(restored.display_name, profile.display_name);
        assert_eq!(restored.bio, profile.bio);
    }

    #[test]
    fn serializes() {
        let profile = Profile::new("did:key:z1").with_name("Alice");
        let json = serde_json::to_string(&profile).unwrap();
        let _: Profile = serde_json::from_str(&json).unwrap();
    }
}
