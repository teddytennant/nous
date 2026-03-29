use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[repr(u32)]
pub enum EventKind {
    Metadata = 0,
    TextNote = 1,
    RecommendRelay = 2,
    FollowList = 3,
    DirectMessage = 4,
    Deletion = 5,
    Repost = 6,
    Reaction = 7,
    ChannelCreate = 40,
    ChannelMessage = 42,
    Custom(u32) = 65535,
}

impl From<u32> for EventKind {
    fn from(n: u32) -> Self {
        match n {
            0 => Self::Metadata,
            1 => Self::TextNote,
            2 => Self::RecommendRelay,
            3 => Self::FollowList,
            4 => Self::DirectMessage,
            5 => Self::Deletion,
            6 => Self::Repost,
            7 => Self::Reaction,
            40 => Self::ChannelCreate,
            42 => Self::ChannelMessage,
            n => Self::Custom(n),
        }
    }
}

impl EventKind {
    pub fn as_u32(&self) -> u32 {
        match self {
            Self::Metadata => 0,
            Self::TextNote => 1,
            Self::RecommendRelay => 2,
            Self::FollowList => 3,
            Self::DirectMessage => 4,
            Self::Deletion => 5,
            Self::Repost => 6,
            Self::Reaction => 7,
            Self::ChannelCreate => 40,
            Self::ChannelMessage => 42,
            Self::Custom(n) => *n,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Tag(pub Vec<String>);

impl Tag {
    pub fn event(event_id: &str) -> Self {
        Self(vec!["e".to_string(), event_id.to_string()])
    }

    pub fn pubkey(pubkey: &str) -> Self {
        Self(vec!["p".to_string(), pubkey.to_string()])
    }

    pub fn reference(url: &str) -> Self {
        Self(vec!["r".to_string(), url.to_string()])
    }

    pub fn hashtag(tag: &str) -> Self {
        Self(vec!["t".to_string(), tag.to_string()])
    }

    pub fn key(&self) -> Option<&str> {
        self.0.first().map(|s| s.as_str())
    }

    pub fn value(&self) -> Option<&str> {
        self.0.get(1).map(|s| s.as_str())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SignedEvent {
    pub id: String,
    pub pubkey: String,
    pub created_at: DateTime<Utc>,
    pub kind: EventKind,
    pub tags: Vec<Tag>,
    pub content: String,
    pub sig: Vec<u8>,
}

impl SignedEvent {
    pub fn new(
        pubkey: impl Into<String>,
        kind: EventKind,
        content: impl Into<String>,
        tags: Vec<Tag>,
    ) -> Self {
        let pubkey = pubkey.into();
        let content = content.into();
        let created_at = Utc::now();

        let id = Self::compute_id(&pubkey, &created_at, &kind, &tags, &content);

        Self {
            id,
            pubkey,
            created_at,
            kind,
            tags,
            content,
            sig: Vec::new(),
        }
    }

    fn compute_id(
        pubkey: &str,
        created_at: &DateTime<Utc>,
        kind: &EventKind,
        tags: &[Tag],
        content: &str,
    ) -> String {
        let serializable = serde_json::json!([
            0,
            pubkey,
            created_at.timestamp(),
            kind.as_u32(),
            tags,
            content,
        ]);
        let bytes = serde_json::to_vec(&serializable).unwrap_or_default();
        let hash = Sha256::digest(&bytes);
        hex::encode(hash)
    }

    pub fn sign(&mut self, keypair: &nous_crypto::KeyPair) {
        let signer = nous_crypto::Signer::new(keypair);
        let sig = signer.sign(self.id.as_bytes());
        self.sig = sig.as_bytes().to_vec();
    }

    pub fn verify(&self) -> nous_core::Result<()> {
        if self.sig.is_empty() {
            return Err(nous_core::Error::Crypto("event is unsigned".into()));
        }

        let expected_id =
            Self::compute_id(&self.pubkey, &self.created_at, &self.kind, &self.tags, &self.content);
        if self.id != expected_id {
            return Err(nous_core::Error::Crypto("event id mismatch".into()));
        }

        let verifying_key = nous_crypto::keys::did_to_public_key(&self.pubkey)?;
        let sig = nous_crypto::Signature(self.sig.clone());
        nous_crypto::Verifier::verify(&verifying_key, self.id.as_bytes(), &sig)
    }

    pub fn is_text_note(&self) -> bool {
        self.kind == EventKind::TextNote
    }

    pub fn is_metadata(&self) -> bool {
        self.kind == EventKind::Metadata
    }

    pub fn referenced_events(&self) -> Vec<&str> {
        self.tags
            .iter()
            .filter(|t| t.key() == Some("e"))
            .filter_map(|t| t.value())
            .collect()
    }

    pub fn referenced_pubkeys(&self) -> Vec<&str> {
        self.tags
            .iter()
            .filter(|t| t.key() == Some("p"))
            .filter_map(|t| t.value())
            .collect()
    }

    pub fn hashtags(&self) -> Vec<&str> {
        self.tags
            .iter()
            .filter(|t| t.key() == Some("t"))
            .filter_map(|t| t.value())
            .collect()
    }
}

impl PartialEq for SignedEvent {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
    }
}

impl Eq for SignedEvent {}

impl std::hash::Hash for SignedEvent {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.id.hash(state);
    }
}

impl PartialOrd for SignedEvent {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for SignedEvent {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        other.created_at.cmp(&self.created_at)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_keypair_and_did() -> (nous_crypto::KeyPair, String) {
        let kp = nous_crypto::KeyPair::generate();
        let did = nous_crypto::keys::public_key_to_did(&kp.verifying_key());
        (kp, did)
    }

    #[test]
    fn create_text_note() {
        let (_, did) = test_keypair_and_did();
        let event = SignedEvent::new(&did, EventKind::TextNote, "hello world", vec![]);
        assert!(event.is_text_note());
        assert_eq!(event.content, "hello world");
        assert!(!event.id.is_empty());
    }

    #[test]
    fn event_id_is_deterministic() {
        let id1 = SignedEvent::compute_id(
            "did:key:z123",
            &DateTime::from_timestamp(1000, 0).unwrap(),
            &EventKind::TextNote,
            &[],
            "test",
        );
        let id2 = SignedEvent::compute_id(
            "did:key:z123",
            &DateTime::from_timestamp(1000, 0).unwrap(),
            &EventKind::TextNote,
            &[],
            "test",
        );
        assert_eq!(id1, id2);
    }

    #[test]
    fn event_id_changes_with_content() {
        let ts = DateTime::from_timestamp(1000, 0).unwrap();
        let id1 = SignedEvent::compute_id("did:key:z123", &ts, &EventKind::TextNote, &[], "hello");
        let id2 = SignedEvent::compute_id("did:key:z123", &ts, &EventKind::TextNote, &[], "world");
        assert_ne!(id1, id2);
    }

    #[test]
    fn sign_and_verify_event() {
        let (kp, did) = test_keypair_and_did();
        let mut event = SignedEvent::new(&did, EventKind::TextNote, "signed note", vec![]);
        event.sign(&kp);
        assert!(event.verify().is_ok());
    }

    #[test]
    fn unsigned_event_fails_verification() {
        let (_, did) = test_keypair_and_did();
        let event = SignedEvent::new(&did, EventKind::TextNote, "unsigned", vec![]);
        assert!(event.verify().is_err());
    }

    #[test]
    fn tampered_content_fails_verification() {
        let (kp, did) = test_keypair_and_did();
        let mut event = SignedEvent::new(&did, EventKind::TextNote, "original", vec![]);
        event.sign(&kp);
        event.content = "tampered".to_string();
        assert!(event.verify().is_err());
    }

    #[test]
    fn event_tags() {
        let tags = vec![
            Tag::event("abc123"),
            Tag::pubkey("did:key:z456"),
            Tag::hashtag("nous"),
        ];
        let (_, did) = test_keypair_and_did();
        let event = SignedEvent::new(&did, EventKind::TextNote, "tagged post", tags);

        assert_eq!(event.referenced_events(), vec!["abc123"]);
        assert_eq!(event.referenced_pubkeys(), vec!["did:key:z456"]);
        assert_eq!(event.hashtags(), vec!["nous"]);
    }

    #[test]
    fn event_kind_roundtrip() {
        assert_eq!(EventKind::from(0), EventKind::Metadata);
        assert_eq!(EventKind::from(1), EventKind::TextNote);
        assert_eq!(EventKind::from(7), EventKind::Reaction);
        assert_eq!(EventKind::from(999).as_u32(), 999);
    }

    #[test]
    fn events_sort_newest_first() {
        let (_, did) = test_keypair_and_did();
        let e1 = SignedEvent::new(&did, EventKind::TextNote, "first", vec![]);
        std::thread::sleep(std::time::Duration::from_millis(10));
        let e2 = SignedEvent::new(&did, EventKind::TextNote, "second", vec![]);

        let mut events = vec![e1.clone(), e2.clone()];
        events.sort();
        assert_eq!(events[0].content, "second");
        assert_eq!(events[1].content, "first");
    }

    #[test]
    fn tag_key_value() {
        let tag = Tag::event("abc");
        assert_eq!(tag.key(), Some("e"));
        assert_eq!(tag.value(), Some("abc"));
    }

    #[test]
    fn event_serializes() {
        let (_, did) = test_keypair_and_did();
        let event = SignedEvent::new(&did, EventKind::TextNote, "serde test", vec![]);
        let json = serde_json::to_string(&event).unwrap();
        let _: SignedEvent = serde_json::from_str(&json).unwrap();
    }
}
