use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Role {
    System,
    User,
    Assistant,
    Tool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    pub id: String,
    pub role: Role,
    pub content: String,
    pub timestamp: DateTime<Utc>,
    pub tool_call_id: Option<String>,
    pub metadata: std::collections::HashMap<String, String>,
}

impl Message {
    pub fn system(content: impl Into<String>) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            role: Role::System,
            content: content.into(),
            timestamp: Utc::now(),
            tool_call_id: None,
            metadata: std::collections::HashMap::new(),
        }
    }

    pub fn user(content: impl Into<String>) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            role: Role::User,
            content: content.into(),
            timestamp: Utc::now(),
            tool_call_id: None,
            metadata: std::collections::HashMap::new(),
        }
    }

    pub fn assistant(content: impl Into<String>) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            role: Role::Assistant,
            content: content.into(),
            timestamp: Utc::now(),
            tool_call_id: None,
            metadata: std::collections::HashMap::new(),
        }
    }

    pub fn tool_result(content: impl Into<String>, call_id: impl Into<String>) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            role: Role::Tool,
            content: content.into(),
            timestamp: Utc::now(),
            tool_call_id: Some(call_id.into()),
            metadata: std::collections::HashMap::new(),
        }
    }

    pub fn token_estimate(&self) -> usize {
        self.content.len() / 4
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Conversation {
    pub id: String,
    pub agent_id: String,
    pub messages: Vec<Message>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl Conversation {
    pub fn new(agent_id: impl Into<String>) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            agent_id: agent_id.into(),
            messages: Vec::new(),
            created_at: Utc::now(),
            updated_at: Utc::now(),
        }
    }

    pub fn add_message(&mut self, message: Message) {
        self.updated_at = Utc::now();
        self.messages.push(message);
    }

    pub fn len(&self) -> usize {
        self.messages.len()
    }

    pub fn is_empty(&self) -> bool {
        self.messages.is_empty()
    }

    pub fn last_message(&self) -> Option<&Message> {
        self.messages.last()
    }

    pub fn messages_by_role(&self, role: Role) -> Vec<&Message> {
        self.messages.iter().filter(|m| m.role == role).collect()
    }

    pub fn total_tokens_estimate(&self) -> usize {
        self.messages.iter().map(|m| m.token_estimate()).sum()
    }

    pub fn truncate_to_tokens(&self, max_tokens: usize) -> Vec<&Message> {
        let mut result = Vec::new();
        let mut total = 0;

        // Always keep system messages
        for msg in &self.messages {
            if msg.role == Role::System {
                total += msg.token_estimate();
                result.push(msg);
            }
        }

        // Add most recent messages that fit
        for msg in self.messages.iter().rev() {
            if msg.role == Role::System {
                continue;
            }
            let est = msg.token_estimate();
            if total + est > max_tokens {
                break;
            }
            total += est;
            result.push(msg);
        }

        result.sort_by_key(|m| m.timestamp);
        result
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn message_roles() {
        let sys = Message::system("you are helpful");
        assert_eq!(sys.role, Role::System);

        let user = Message::user("hello");
        assert_eq!(user.role, Role::User);

        let asst = Message::assistant("hi there");
        assert_eq!(asst.role, Role::Assistant);

        let tool = Message::tool_result("result", "call-1");
        assert_eq!(tool.role, Role::Tool);
        assert_eq!(tool.tool_call_id.as_deref(), Some("call-1"));
    }

    #[test]
    fn conversation_basics() {
        let mut conv = Conversation::new("agent-1");
        assert!(conv.is_empty());

        conv.add_message(Message::system("system prompt"));
        conv.add_message(Message::user("hello"));
        conv.add_message(Message::assistant("hi"));

        assert_eq!(conv.len(), 3);
        assert_eq!(conv.last_message().unwrap().role, Role::Assistant);
    }

    #[test]
    fn messages_by_role() {
        let mut conv = Conversation::new("agent-1");
        conv.add_message(Message::user("q1"));
        conv.add_message(Message::assistant("a1"));
        conv.add_message(Message::user("q2"));
        conv.add_message(Message::assistant("a2"));

        assert_eq!(conv.messages_by_role(Role::User).len(), 2);
        assert_eq!(conv.messages_by_role(Role::Assistant).len(), 2);
        assert_eq!(conv.messages_by_role(Role::System).len(), 0);
    }

    #[test]
    fn token_estimate() {
        let msg = Message::user("a".repeat(400));
        assert_eq!(msg.token_estimate(), 100);
    }

    #[test]
    fn truncate_keeps_system() {
        let mut conv = Conversation::new("agent-1");
        conv.add_message(Message::system("system"));
        for i in 0..100 {
            conv.add_message(Message::user(&format!("msg {i}")));
        }

        let truncated = conv.truncate_to_tokens(50);
        assert!(truncated.iter().any(|m| m.role == Role::System));
        assert!(truncated.len() < conv.len());
    }

    #[test]
    fn conversation_serializes() {
        let mut conv = Conversation::new("agent-1");
        conv.add_message(Message::user("test"));
        let json = serde_json::to_string(&conv).unwrap();
        let restored: Conversation = serde_json::from_str(&json).unwrap();
        assert_eq!(restored.len(), 1);
    }

    #[test]
    fn unique_message_ids() {
        let m1 = Message::user("a");
        let m2 = Message::user("a");
        assert_ne!(m1.id, m2.id);
    }
}
