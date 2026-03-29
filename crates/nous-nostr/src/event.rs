use ed25519_dalek::{Signer, SigningKey, Verifier, VerifyingKey};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::fmt;

/// NIP-01 event kinds.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct Kind(pub u16);

impl Kind {
    pub const METADATA: Self = Self(0);
    pub const TEXT_NOTE: Self = Self(1);
    pub const RECOMMEND_RELAY: Self = Self(2);
    pub const CONTACTS: Self = Self(3);
    pub const ENCRYPTED_DM: Self = Self(4);
    pub const DELETE: Self = Self(5);
    pub const REPOST: Self = Self(6);
    pub const REACTION: Self = Self(7);
    pub const CHANNEL_CREATE: Self = Self(40);
    pub const CHANNEL_METADATA: Self = Self(41);
    pub const CHANNEL_MESSAGE: Self = Self(42);
}

impl fmt::Display for Kind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// A single tag entry: `["tag_name", "value1", "value2", ...]`
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct Tag(pub Vec<String>);

impl Tag {
    pub fn new(entries: Vec<String>) -> Self {
        Self(entries)
    }

    pub fn tag_name(&self) -> Option<&str> {
        self.0.first().map(|s| s.as_str())
    }

    pub fn value(&self) -> Option<&str> {
        self.0.get(1).map(|s| s.as_str())
    }

    pub fn event(event_id: &str) -> Self {
        Self(vec!["e".to_string(), event_id.to_string()])
    }

    pub fn pubkey(pubkey: &str) -> Self {
        Self(vec!["p".to_string(), pubkey.to_string()])
    }
}

/// A Nostr event per NIP-01.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Event {
    pub id: String,
    pub pubkey: String,
    pub created_at: u64,
    pub kind: Kind,
    pub tags: Vec<Tag>,
    pub content: String,
    pub sig: String,
}

impl Event {
    /// Compute the event ID per NIP-01: SHA256 of `[0, pubkey, created_at, kind, tags, content]`.
    pub fn compute_id(
        pubkey: &str,
        created_at: u64,
        kind: Kind,
        tags: &[Tag],
        content: &str,
    ) -> String {
        let serialized = serde_json::to_string(&(0u8, pubkey, created_at, kind, tags, content))
            .expect("serialization cannot fail");
        let hash = Sha256::digest(serialized.as_bytes());
        hex::encode(hash)
    }

    /// Verify the event's ID matches its content.
    pub fn verify_id(&self) -> bool {
        let computed = Self::compute_id(
            &self.pubkey,
            self.created_at,
            self.kind,
            &self.tags,
            &self.content,
        );
        computed == self.id
    }

    /// Verify the event's signature against its pubkey.
    pub fn verify_signature(&self) -> bool {
        let Ok(pubkey_bytes) = hex::decode(&self.pubkey) else {
            return false;
        };
        let Ok(sig_bytes) = hex::decode(&self.sig) else {
            return false;
        };
        let Ok(pubkey_arr): Result<[u8; 32], _> = pubkey_bytes.try_into() else {
            return false;
        };
        let Ok(sig_arr): Result<[u8; 64], _> = sig_bytes.try_into() else {
            return false;
        };
        let Ok(verifying_key) = VerifyingKey::from_bytes(&pubkey_arr) else {
            return false;
        };
        let signature = ed25519_dalek::Signature::from_bytes(&sig_arr);
        verifying_key.verify(self.id.as_bytes(), &signature).is_ok()
    }

    /// Verify both ID and signature.
    pub fn verify(&self) -> bool {
        self.verify_id() && self.verify_signature()
    }

    /// Check if a tag with the given name and value exists.
    pub fn has_tag(&self, name: &str, value: &str) -> bool {
        self.tags
            .iter()
            .any(|t| t.tag_name() == Some(name) && t.value() == Some(value))
    }
}

/// Builder for creating signed Nostr events.
pub struct EventBuilder {
    kind: Kind,
    content: String,
    tags: Vec<Tag>,
    created_at: Option<u64>,
}

impl EventBuilder {
    pub fn new(kind: Kind, content: impl Into<String>) -> Self {
        Self {
            kind,
            content: content.into(),
            tags: Vec::new(),
            created_at: None,
        }
    }

    pub fn tag(mut self, tag: Tag) -> Self {
        self.tags.push(tag);
        self
    }

    pub fn tags(mut self, tags: Vec<Tag>) -> Self {
        self.tags = tags;
        self
    }

    pub fn created_at(mut self, ts: u64) -> Self {
        self.created_at = Some(ts);
        self
    }

    pub fn sign(self, signing_key: &SigningKey) -> Event {
        let pubkey = hex::encode(signing_key.verifying_key().as_bytes());
        let created_at = self.created_at.unwrap_or_else(|| {
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs()
        });

        let id = Event::compute_id(&pubkey, created_at, self.kind, &self.tags, &self.content);
        let signature = signing_key.sign(id.as_bytes());
        let sig = hex::encode(signature.to_bytes());

        Event {
            id,
            pubkey,
            created_at,
            kind: self.kind,
            tags: self.tags,
            content: self.content,
            sig,
        }
    }

    pub fn text_note(content: impl Into<String>) -> Self {
        Self::new(Kind::TEXT_NOTE, content)
    }

    pub fn metadata(content: impl Into<String>) -> Self {
        Self::new(Kind::METADATA, content)
    }

    /// NIP-02: Create a contact list event.
    ///
    /// `contacts` is a list of `(pubkey, relay_url, petname)` tuples.
    /// Relay URL and petname can be empty strings if not applicable.
    pub fn contact_list(contacts: &[(&str, &str, &str)]) -> Self {
        let tags: Vec<Tag> = contacts
            .iter()
            .map(|(pubkey, relay_url, petname)| {
                Tag::new(vec![
                    "p".to_string(),
                    pubkey.to_string(),
                    relay_url.to_string(),
                    petname.to_string(),
                ])
            })
            .collect();
        Self::new(Kind::CONTACTS, "").tags(tags)
    }

    /// NIP-09: Create a deletion event for the given event IDs.
    ///
    /// The `reason` field is optional and stored in the content field.
    /// Only events authored by the same pubkey will be deleted by the relay.
    pub fn deletion(event_ids: &[&str], reason: impl Into<String>) -> Self {
        let tags: Vec<Tag> = event_ids.iter().map(|id| Tag::event(id)).collect();
        Self::new(Kind::DELETE, reason).tags(tags)
    }

    /// NIP-04: Create an encrypted direct message.
    ///
    /// Uses x25519 key exchange (ed25519 key converted to x25519) and
    /// AES-256-GCM for encryption. The ciphertext is base64-encoded in the
    /// content field, and the recipient's pubkey is tagged.
    pub fn encrypted_dm(
        plaintext: &str,
        recipient_pubkey_hex: &str,
        sender_signing_key: &SigningKey,
    ) -> Result<Self, String> {
        let ciphertext =
            crate::nip04::encrypt(plaintext, recipient_pubkey_hex, sender_signing_key)?;
        Ok(Self::new(Kind::ENCRYPTED_DM, ciphertext).tag(Tag::pubkey(recipient_pubkey_hex)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rand::rngs::OsRng;

    fn test_key() -> SigningKey {
        SigningKey::generate(&mut OsRng)
    }

    #[test]
    fn event_builder_creates_valid_event() {
        let key = test_key();
        let event = EventBuilder::text_note("hello nostr")
            .created_at(1700000000)
            .sign(&key);

        assert_eq!(event.kind, Kind::TEXT_NOTE);
        assert_eq!(event.content, "hello nostr");
        assert_eq!(event.created_at, 1700000000);
        assert!(event.verify_id());
        assert!(event.verify_signature());
        assert!(event.verify());
    }

    #[test]
    fn event_id_is_sha256_of_serialized_content() {
        let key = test_key();
        let event = EventBuilder::text_note("test").created_at(1000).sign(&key);

        let expected_id = Event::compute_id(&event.pubkey, 1000, Kind::TEXT_NOTE, &[], "test");
        assert_eq!(event.id, expected_id);
    }

    #[test]
    fn tampered_content_fails_id_verification() {
        let key = test_key();
        let mut event = EventBuilder::text_note("original").sign(&key);
        event.content = "tampered".to_string();
        assert!(!event.verify_id());
    }

    #[test]
    fn tampered_signature_fails_verification() {
        let key = test_key();
        let mut event = EventBuilder::text_note("test").sign(&key);
        let mut sig_bytes = hex::decode(&event.sig).unwrap();
        sig_bytes[0] ^= 0xff;
        event.sig = hex::encode(&sig_bytes);
        assert!(!event.verify_signature());
    }

    #[test]
    fn wrong_pubkey_fails_verification() {
        let key = test_key();
        let other_key = test_key();
        let mut event = EventBuilder::text_note("test").sign(&key);
        event.pubkey = hex::encode(other_key.verifying_key().as_bytes());
        assert!(!event.verify_id()); // ID includes pubkey
    }

    #[test]
    fn event_with_tags() {
        let key = test_key();
        let event = EventBuilder::text_note("reply")
            .tag(Tag::event("abc123"))
            .tag(Tag::pubkey("def456"))
            .sign(&key);

        assert_eq!(event.tags.len(), 2);
        assert!(event.has_tag("e", "abc123"));
        assert!(event.has_tag("p", "def456"));
        assert!(!event.has_tag("e", "nonexistent"));
        assert!(event.verify());
    }

    #[test]
    fn metadata_event() {
        let key = test_key();
        let meta = r#"{"name":"alice","about":"hello"}"#;
        let event = EventBuilder::metadata(meta).sign(&key);
        assert_eq!(event.kind, Kind::METADATA);
        assert_eq!(event.content, meta);
        assert!(event.verify());
    }

    #[test]
    fn tag_accessors() {
        let tag = Tag::new(vec!["e".into(), "eventid".into(), "relay_url".into()]);
        assert_eq!(tag.tag_name(), Some("e"));
        assert_eq!(tag.value(), Some("eventid"));
    }

    #[test]
    fn empty_tag() {
        let tag = Tag::new(vec![]);
        assert_eq!(tag.tag_name(), None);
        assert_eq!(tag.value(), None);
    }

    #[test]
    fn event_serialization_roundtrip() {
        let key = test_key();
        let event = EventBuilder::text_note("hello")
            .tag(Tag::pubkey("abc"))
            .created_at(999)
            .sign(&key);

        let json = serde_json::to_string(&event).unwrap();
        let deserialized: Event = serde_json::from_str(&json).unwrap();
        assert_eq!(event, deserialized);
    }

    #[test]
    fn kind_constants() {
        assert_eq!(Kind::METADATA.0, 0);
        assert_eq!(Kind::TEXT_NOTE.0, 1);
        assert_eq!(Kind::RECOMMEND_RELAY.0, 2);
        assert_eq!(Kind::CONTACTS.0, 3);
        assert_eq!(Kind::ENCRYPTED_DM.0, 4);
        assert_eq!(Kind::DELETE.0, 5);
        assert_eq!(Kind::REPOST.0, 6);
        assert_eq!(Kind::REACTION.0, 7);
    }

    #[test]
    fn kind_display() {
        assert_eq!(Kind::TEXT_NOTE.to_string(), "1");
        assert_eq!(Kind(42).to_string(), "42");
    }

    // ── NIP-02: Contact Lists ─────────────────────────────────

    #[test]
    fn nip02_contact_list_event() {
        let key = test_key();
        let contacts = vec![
            ("abc123", "wss://relay.example.com", "alice"),
            ("def456", "", "bob"),
        ];
        let event = EventBuilder::contact_list(&contacts).sign(&key);

        assert_eq!(event.kind, Kind::CONTACTS);
        assert_eq!(event.content, "");
        assert_eq!(event.tags.len(), 2);
        assert!(event.has_tag("p", "abc123"));
        assert!(event.has_tag("p", "def456"));
        assert!(event.verify());
    }

    #[test]
    fn nip02_empty_contact_list() {
        let key = test_key();
        let event = EventBuilder::contact_list(&[]).sign(&key);

        assert_eq!(event.kind, Kind::CONTACTS);
        assert!(event.tags.is_empty());
        assert!(event.verify());
    }

    #[test]
    fn nip02_contact_list_tag_structure() {
        let key = test_key();
        let contacts = vec![("pubkey1", "wss://relay.test", "petname1")];
        let event = EventBuilder::contact_list(&contacts).sign(&key);

        let tag = &event.tags[0];
        assert_eq!(tag.0.len(), 4);
        assert_eq!(tag.0[0], "p");
        assert_eq!(tag.0[1], "pubkey1");
        assert_eq!(tag.0[2], "wss://relay.test");
        assert_eq!(tag.0[3], "petname1");
    }

    // ── NIP-09: Event Deletion ─────────────────────────────────

    #[test]
    fn nip09_deletion_event() {
        let key = test_key();
        let event = EventBuilder::deletion(&["event1", "event2"], "spam").sign(&key);

        assert_eq!(event.kind, Kind::DELETE);
        assert_eq!(event.content, "spam");
        assert_eq!(event.tags.len(), 2);
        assert!(event.has_tag("e", "event1"));
        assert!(event.has_tag("e", "event2"));
        assert!(event.verify());
    }

    #[test]
    fn nip09_deletion_empty_reason() {
        let key = test_key();
        let event = EventBuilder::deletion(&["abc"], "").sign(&key);
        assert_eq!(event.kind, Kind::DELETE);
        assert_eq!(event.content, "");
        assert_eq!(event.tags.len(), 1);
        assert!(event.verify());
    }

    #[test]
    fn nip09_deletion_no_events() {
        let key = test_key();
        let event = EventBuilder::deletion(&[], "cleanup").sign(&key);
        assert_eq!(event.kind, Kind::DELETE);
        assert!(event.tags.is_empty());
        assert!(event.verify());
    }

    // ── NIP-04: Encrypted DMs ─────────────────────────────────

    #[test]
    fn nip04_encrypted_dm_event() {
        let alice = test_key();
        let bob = test_key();
        let bob_pubkey = hex::encode(bob.verifying_key().as_bytes());

        let event = EventBuilder::encrypted_dm("secret message", &bob_pubkey, &alice)
            .unwrap()
            .sign(&alice);

        assert_eq!(event.kind, Kind::ENCRYPTED_DM);
        assert!(event.has_tag("p", &bob_pubkey));
        // Content is encrypted, not the original plaintext
        assert_ne!(event.content, "secret message");
        assert!(event.content.contains("?iv="));
        assert!(event.verify());
    }

    #[test]
    fn nip04_encrypted_dm_decryptable() {
        let alice = test_key();
        let bob = test_key();
        let alice_pubkey = hex::encode(alice.verifying_key().as_bytes());
        let bob_pubkey = hex::encode(bob.verifying_key().as_bytes());

        let event = EventBuilder::encrypted_dm("hello bob", &bob_pubkey, &alice)
            .unwrap()
            .sign(&alice);

        // Bob decrypts
        let decrypted = crate::nip04::decrypt(&event.content, &alice_pubkey, &bob).unwrap();
        assert_eq!(decrypted, "hello bob");
    }
}
