use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Post {
    pub id: String,
    pub author_did: String,
    pub content: String,
    pub created_at: DateTime<Utc>,
    pub reply_to: Option<String>,
    pub reactions: Vec<Reaction>,
}

impl Post {
    pub fn new(author_did: impl Into<String>, content: impl Into<String>) -> Self {
        Self {
            id: format!("post:{}", Uuid::new_v4()),
            author_did: author_did.into(),
            content: content.into(),
            created_at: Utc::now(),
            reply_to: None,
            reactions: Vec::new(),
        }
    }

    pub fn reply(
        author_did: impl Into<String>,
        content: impl Into<String>,
        parent_id: impl Into<String>,
    ) -> Self {
        let mut post = Self::new(author_did, content);
        post.reply_to = Some(parent_id.into());
        post
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Reaction {
    pub author_did: String,
    pub emoji: String,
    pub timestamp: DateTime<Utc>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn create_post() {
        let post = Post::new("did:key:ztest", "hello world");
        assert!(post.id.starts_with("post:"));
        assert_eq!(post.content, "hello world");
        assert!(post.reply_to.is_none());
    }

    #[test]
    fn create_reply() {
        let post = Post::reply("did:key:ztest", "reply", "post:parent");
        assert_eq!(post.reply_to.as_deref(), Some("post:parent"));
    }
}
