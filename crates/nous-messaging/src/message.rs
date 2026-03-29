use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use nous_core::Result;
use nous_crypto::signing::{Signature, Verifier};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MessageContent {
    Text(String),
    File {
        name: String,
        mime_type: String,
        size: u64,
        hash: String,
    },
    Reaction {
        target_id: String,
        emoji: String,
    },
    System(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    pub id: String,
    pub channel_id: String,
    pub sender_did: String,
    pub content: MessageContent,
    pub timestamp: DateTime<Utc>,
    pub reply_to: Option<String>,
    pub edited: bool,
    pub signature: Signature,
}

impl Message {
    pub fn signable_bytes(&self) -> Result<Vec<u8>> {
        let signable = serde_json::json!({
            "id": self.id,
            "channel_id": self.channel_id,
            "sender_did": self.sender_did,
            "content": self.content,
            "timestamp": self.timestamp,
            "reply_to": self.reply_to,
        });
        Ok(serde_json::to_vec(&signable)?)
    }

    pub fn verify(&self) -> Result<()> {
        let sender_key = nous_crypto::keys::did_to_public_key(&self.sender_did)?;
        let payload = self.signable_bytes()?;
        Verifier::verify(&sender_key, &payload, &self.signature)
    }
}

pub struct MessageBuilder {
    channel_id: String,
    content: MessageContent,
    reply_to: Option<String>,
}

impl MessageBuilder {
    pub fn text(channel_id: impl Into<String>, text: impl Into<String>) -> Self {
        Self {
            channel_id: channel_id.into(),
            content: MessageContent::Text(text.into()),
            reply_to: None,
        }
    }

    pub fn file(
        channel_id: impl Into<String>,
        name: impl Into<String>,
        mime_type: impl Into<String>,
        size: u64,
        hash: impl Into<String>,
    ) -> Self {
        Self {
            channel_id: channel_id.into(),
            content: MessageContent::File {
                name: name.into(),
                mime_type: mime_type.into(),
                size,
                hash: hash.into(),
            },
            reply_to: None,
        }
    }

    pub fn reaction(
        channel_id: impl Into<String>,
        target_id: impl Into<String>,
        emoji: impl Into<String>,
    ) -> Self {
        Self {
            channel_id: channel_id.into(),
            content: MessageContent::Reaction {
                target_id: target_id.into(),
                emoji: emoji.into(),
            },
            reply_to: None,
        }
    }

    pub fn reply_to(mut self, message_id: impl Into<String>) -> Self {
        self.reply_to = Some(message_id.into());
        self
    }

    pub fn sign(self, identity: &nous_identity::Identity) -> Result<Message> {
        let id = format!("msg:{}", Uuid::new_v4());
        let timestamp = Utc::now();

        let signable = serde_json::json!({
            "id": id,
            "channel_id": self.channel_id,
            "sender_did": identity.did(),
            "content": self.content,
            "timestamp": timestamp,
            "reply_to": self.reply_to,
        });
        let payload = serde_json::to_vec(&signable)?;
        let signature = identity.sign(&payload);

        Ok(Message {
            id,
            channel_id: self.channel_id,
            sender_did: identity.did().to_string(),
            content: self.content,
            timestamp,
            reply_to: self.reply_to,
            edited: false,
            signature,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use nous_identity::Identity;

    #[test]
    fn create_and_verify_text_message() {
        let sender = Identity::generate();
        let msg = MessageBuilder::text("channel-1", "hello from the overman")
            .sign(&sender)
            .unwrap();

        assert!(msg.verify().is_ok());
        assert_eq!(msg.sender_did, sender.did());
        assert_eq!(msg.channel_id, "channel-1");
        assert!(!msg.edited);
    }

    #[test]
    fn create_reply_message() {
        let sender = Identity::generate();
        let original = MessageBuilder::text("ch", "original")
            .sign(&sender)
            .unwrap();

        let reply = MessageBuilder::text("ch", "reply")
            .reply_to(&original.id)
            .sign(&sender)
            .unwrap();

        assert_eq!(reply.reply_to.as_deref(), Some(original.id.as_str()));
        assert!(reply.verify().is_ok());
    }

    #[test]
    fn create_file_message() {
        let sender = Identity::generate();
        let msg = MessageBuilder::file("ch", "doc.pdf", "application/pdf", 1024, "sha256:abc123")
            .sign(&sender)
            .unwrap();

        assert!(msg.verify().is_ok());
        match &msg.content {
            MessageContent::File { name, size, .. } => {
                assert_eq!(name, "doc.pdf");
                assert_eq!(*size, 1024);
            }
            _ => panic!("expected file message"),
        }
    }

    #[test]
    fn create_reaction_message() {
        let sender = Identity::generate();
        let msg = MessageBuilder::reaction("ch", "msg:123", "+1")
            .sign(&sender)
            .unwrap();

        assert!(msg.verify().is_ok());
        match &msg.content {
            MessageContent::Reaction { target_id, emoji } => {
                assert_eq!(target_id, "msg:123");
                assert_eq!(emoji, "+1");
            }
            _ => panic!("expected reaction message"),
        }
    }

    #[test]
    fn tampered_message_fails_verification() {
        let sender = Identity::generate();
        let mut msg = MessageBuilder::text("ch", "original")
            .sign(&sender)
            .unwrap();

        msg.content = MessageContent::Text("tampered".into());
        assert!(msg.verify().is_err());
    }

    #[test]
    fn message_serializes() {
        let sender = Identity::generate();
        let msg = MessageBuilder::text("ch", "serde test")
            .sign(&sender)
            .unwrap();

        let json = serde_json::to_string(&msg).unwrap();
        let deserialized: Message = serde_json::from_str(&json).unwrap();

        assert_eq!(deserialized.id, msg.id);
        assert!(deserialized.verify().is_ok());
    }

    #[test]
    fn unique_message_ids() {
        let sender = Identity::generate();
        let a = MessageBuilder::text("ch", "a").sign(&sender).unwrap();
        let b = MessageBuilder::text("ch", "b").sign(&sender).unwrap();
        assert_ne!(a.id, b.id);
    }
}
