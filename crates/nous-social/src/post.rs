use crate::event::{EventKind, SignedEvent, Tag};

pub struct PostBuilder {
    author_did: String,
    content: String,
    tags: Vec<Tag>,
    reply_to: Option<String>,
}

impl PostBuilder {
    pub fn new(author_did: impl Into<String>, content: impl Into<String>) -> Self {
        Self {
            author_did: author_did.into(),
            content: content.into(),
            tags: Vec::new(),
            reply_to: None,
        }
    }

    pub fn reply_to(mut self, event_id: impl Into<String>) -> Self {
        let id = event_id.into();
        self.tags.push(Tag::event(&id));
        self.reply_to = Some(id);
        self
    }

    pub fn mention(mut self, pubkey: impl Into<String>) -> Self {
        self.tags.push(Tag::pubkey(&pubkey.into()));
        self
    }

    pub fn hashtag(mut self, tag: impl Into<String>) -> Self {
        self.tags.push(Tag::hashtag(&tag.into()));
        self
    }

    pub fn reference_url(mut self, url: impl Into<String>) -> Self {
        self.tags.push(Tag::reference(&url.into()));
        self
    }

    pub fn build(self) -> SignedEvent {
        SignedEvent::new(self.author_did, EventKind::TextNote, self.content, self.tags)
    }

    pub fn build_signed(self, keypair: &nous_crypto::KeyPair) -> SignedEvent {
        let mut event = self.build();
        event.sign(keypair);
        event
    }
}

pub fn reaction(
    author_did: &str,
    target_event_id: &str,
    emoji: &str,
) -> SignedEvent {
    SignedEvent::new(
        author_did,
        EventKind::Reaction,
        emoji,
        vec![Tag::event(target_event_id)],
    )
}

pub fn repost(author_did: &str, target_event_id: &str) -> SignedEvent {
    SignedEvent::new(
        author_did,
        EventKind::Repost,
        "",
        vec![Tag::event(target_event_id)],
    )
}

pub fn deletion(author_did: &str, event_ids: &[&str], reason: &str) -> SignedEvent {
    let tags: Vec<Tag> = event_ids.iter().map(|id| Tag::event(id)).collect();
    SignedEvent::new(author_did, EventKind::Deletion, reason, tags)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_did() -> String {
        let kp = nous_crypto::KeyPair::generate();
        nous_crypto::keys::public_key_to_did(&kp.verifying_key())
    }

    #[test]
    fn build_simple_post() {
        let did = test_did();
        let post = PostBuilder::new(&did, "hello world").build();
        assert_eq!(post.content, "hello world");
        assert_eq!(post.kind, EventKind::TextNote);
    }

    #[test]
    fn build_reply() {
        let did = test_did();
        let post = PostBuilder::new(&did, "reply")
            .reply_to("event123")
            .build();
        assert!(post.referenced_events().contains(&"event123"));
    }

    #[test]
    fn build_with_mentions_and_hashtags() {
        let did = test_did();
        let post = PostBuilder::new(&did, "Hey @bob check #nous")
            .mention("did:key:bob")
            .hashtag("nous")
            .build();

        assert!(post.referenced_pubkeys().contains(&"did:key:bob"));
        assert!(post.hashtags().contains(&"nous"));
    }

    #[test]
    fn build_signed_post() {
        let kp = nous_crypto::KeyPair::generate();
        let did = nous_crypto::keys::public_key_to_did(&kp.verifying_key());
        let post = PostBuilder::new(&did, "signed").build_signed(&kp);
        assert!(post.verify().is_ok());
    }

    #[test]
    fn create_reaction() {
        let did = test_did();
        let r = reaction(&did, "event123", "+");
        assert_eq!(r.kind, EventKind::Reaction);
        assert_eq!(r.content, "+");
        assert!(r.referenced_events().contains(&"event123"));
    }

    #[test]
    fn create_repost() {
        let did = test_did();
        let r = repost(&did, "event123");
        assert_eq!(r.kind, EventKind::Repost);
        assert!(r.referenced_events().contains(&"event123"));
    }

    #[test]
    fn create_deletion() {
        let did = test_did();
        let d = deletion(&did, &["event1", "event2"], "spam");
        assert_eq!(d.kind, EventKind::Deletion);
        assert_eq!(d.content, "spam");
        let refs = d.referenced_events();
        assert!(refs.contains(&"event1"));
        assert!(refs.contains(&"event2"));
    }
}
