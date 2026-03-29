use libp2p::gossipsub::IdentTopic;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum NousTopic {
    Messages,
    Social,
    Governance,
    Payments,
    Identity,
    Marketplace,
    Sync,
    Custom(String),
}

impl NousTopic {
    pub fn as_str(&self) -> &str {
        match self {
            Self::Messages => "/nous/messages/1.0",
            Self::Social => "/nous/social/1.0",
            Self::Governance => "/nous/governance/1.0",
            Self::Payments => "/nous/payments/1.0",
            Self::Identity => "/nous/identity/1.0",
            Self::Marketplace => "/nous/marketplace/1.0",
            Self::Sync => "/nous/sync/1.0",
            Self::Custom(s) => s.as_str(),
        }
    }

    pub fn as_topic(&self) -> IdentTopic {
        IdentTopic::new(self.as_str())
    }

    pub fn all_default() -> Vec<Self> {
        vec![
            Self::Messages,
            Self::Social,
            Self::Governance,
            Self::Payments,
            Self::Identity,
            Self::Marketplace,
            Self::Sync,
        ]
    }

    pub fn from_topic_hash(hash: &libp2p::gossipsub::TopicHash) -> Option<Self> {
        let s = hash.as_str();
        match s {
            "/nous/messages/1.0" => Some(Self::Messages),
            "/nous/social/1.0" => Some(Self::Social),
            "/nous/governance/1.0" => Some(Self::Governance),
            "/nous/payments/1.0" => Some(Self::Payments),
            "/nous/identity/1.0" => Some(Self::Identity),
            "/nous/marketplace/1.0" => Some(Self::Marketplace),
            "/nous/sync/1.0" => Some(Self::Sync),
            _ => Some(Self::Custom(s.to_string())),
        }
    }
}

impl std::fmt::Display for NousTopic {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn topic_strings_are_namespaced() {
        for topic in NousTopic::all_default() {
            assert!(topic.as_str().starts_with("/nous/"));
        }
    }

    #[test]
    fn topic_roundtrip_via_hash() {
        for original in NousTopic::all_default() {
            let ident = original.as_topic();
            let hash = ident.hash();
            let recovered = NousTopic::from_topic_hash(&hash).unwrap();
            assert_eq!(original, recovered);
        }
    }

    #[test]
    fn custom_topic() {
        let topic = NousTopic::Custom("/app/custom/1.0".to_string());
        assert_eq!(topic.as_str(), "/app/custom/1.0");
    }

    #[test]
    fn all_default_has_seven_topics() {
        assert_eq!(NousTopic::all_default().len(), 7);
    }

    #[test]
    fn topic_display() {
        let topic = NousTopic::Messages;
        assert_eq!(format!("{topic}"), "/nous/messages/1.0");
    }

    #[test]
    fn topic_serializes() {
        let topic = NousTopic::Governance;
        let json = serde_json::to_string(&topic).unwrap();
        let deserialized: NousTopic = serde_json::from_str(&json).unwrap();
        assert_eq!(topic, deserialized);
    }
}
