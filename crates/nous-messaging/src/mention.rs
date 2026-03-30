//! Mentions: extract and resolve @-mentions from message text.
//!
//! Supports DID mentions (`@did:key:z...`), display name mentions (`@alice`),
//! and special mentions (`@everyone`, `@here`). Parsed mentions can be used
//! for notifications, highlighting, and access control.

use serde::{Deserialize, Serialize};

/// A parsed mention from a message body.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum Mention {
    /// Mention a specific user by DID.
    Did(String),
    /// Mention a user by display name (needs resolution).
    Name(String),
    /// Mention all members in the channel.
    Everyone,
    /// Mention online/active members.
    Here,
}

impl Mention {
    /// Whether this mention targets a specific DID.
    pub fn targets(&self, did: &str) -> bool {
        match self {
            Self::Did(d) => d == did,
            Self::Everyone | Self::Here => true,
            Self::Name(_) => false, // needs resolution
        }
    }

    /// Whether this is a broadcast mention (@everyone or @here).
    pub fn is_broadcast(&self) -> bool {
        matches!(self, Self::Everyone | Self::Here)
    }
}

/// Extract all mentions from a message body.
///
/// Recognized patterns:
/// - `@did:key:z...` — DID mention (word boundary delimited)
/// - `@everyone` — all members
/// - `@here` — active members
/// - `@name` — display name (alphanumeric + underscore + hyphen, 1-64 chars)
pub fn extract_mentions(text: &str) -> Vec<Mention> {
    let mut mentions = Vec::new();
    let mut seen = std::collections::HashSet::new();

    let mut chars = text.char_indices().peekable();

    while let Some((i, ch)) = chars.next() {
        if ch != '@' {
            continue;
        }

        // Must be at start or preceded by whitespace/punctuation
        if i > 0 {
            let prev = text[..i].chars().last().unwrap();
            if prev.is_alphanumeric() || prev == '_' {
                continue;
            }
        }

        // Collect the mention content
        let start = i + 1;
        let mut end = start;
        while let Some(&(j, c)) = chars.peek() {
            if c.is_alphanumeric() || c == ':' || c == '_' || c == '-' || c == '.' {
                end = j + c.len_utf8();
                chars.next();
            } else {
                break;
            }
        }

        if end <= start {
            continue;
        }

        let word = &text[start..end];

        let mention = if word.eq_ignore_ascii_case("everyone") {
            Mention::Everyone
        } else if word.eq_ignore_ascii_case("here") {
            Mention::Here
        } else if word.starts_with("did:") {
            Mention::Did(word.to_string())
        } else if word.len() <= 64 && word.chars().all(|c| c.is_alphanumeric() || c == '_' || c == '-') {
            Mention::Name(word.to_string())
        } else {
            continue;
        };

        let key = format!("{mention:?}");
        if seen.insert(key) {
            mentions.push(mention);
        }
    }

    mentions
}

/// Replace mention placeholders with formatted display text.
/// `resolver` maps DID -> display name.
pub fn render_mentions(text: &str, resolver: &dyn Fn(&str) -> Option<String>) -> String {
    let mentions = extract_mentions(text);
    let mut result = text.to_string();

    for mention in mentions.iter().rev() {
        if let Mention::Did(did) = mention {
            if let Some(name) = resolver(did) {
                let pattern = format!("@{did}");
                result = result.replace(&pattern, &format!("@{name}"));
            }
        }
    }

    result
}

/// Count how many unique users are mentioned in a message.
pub fn mention_count(text: &str) -> usize {
    extract_mentions(text).len()
}

/// Check if a specific DID is mentioned (directly or via broadcast).
pub fn is_mentioned(text: &str, did: &str) -> bool {
    extract_mentions(text).iter().any(|m| m.targets(did))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extract_did_mention() {
        let mentions = extract_mentions("Hello @did:key:z6MkhaXgBZDvotDkL5257faiztiGiC2QtKLGpbnnEGta2doK");
        assert_eq!(mentions.len(), 1);
        assert_eq!(
            mentions[0],
            Mention::Did("did:key:z6MkhaXgBZDvotDkL5257faiztiGiC2QtKLGpbnnEGta2doK".into())
        );
    }

    #[test]
    fn extract_name_mention() {
        let mentions = extract_mentions("Hey @alice, check this out");
        assert_eq!(mentions.len(), 1);
        assert_eq!(mentions[0], Mention::Name("alice".into()));
    }

    #[test]
    fn extract_everyone() {
        let mentions = extract_mentions("@everyone please review");
        assert_eq!(mentions.len(), 1);
        assert_eq!(mentions[0], Mention::Everyone);
    }

    #[test]
    fn extract_here() {
        let mentions = extract_mentions("@here quick sync?");
        assert_eq!(mentions.len(), 1);
        assert_eq!(mentions[0], Mention::Here);
    }

    #[test]
    fn extract_multiple() {
        let mentions = extract_mentions("@alice and @bob please review @everyone");
        assert_eq!(mentions.len(), 3);
        assert_eq!(mentions[0], Mention::Name("alice".into()));
        assert_eq!(mentions[1], Mention::Name("bob".into()));
        assert_eq!(mentions[2], Mention::Everyone);
    }

    #[test]
    fn no_duplicate_mentions() {
        let mentions = extract_mentions("@alice @alice @alice");
        assert_eq!(mentions.len(), 1);
    }

    #[test]
    fn no_mention_in_email() {
        let mentions = extract_mentions("email me at user@example.com");
        // 'user@example' would be caught but 'user' is alphanumeric prefix to @
        assert!(mentions.is_empty());
    }

    #[test]
    fn mention_at_start_of_text() {
        let mentions = extract_mentions("@alice hi");
        assert_eq!(mentions.len(), 1);
    }

    #[test]
    fn mention_after_newline() {
        let mentions = extract_mentions("hello\n@bob");
        assert_eq!(mentions.len(), 1);
        assert_eq!(mentions[0], Mention::Name("bob".into()));
    }

    #[test]
    fn mention_after_punctuation() {
        let mentions = extract_mentions("(@alice)");
        assert_eq!(mentions.len(), 1);
    }

    #[test]
    fn empty_text() {
        assert!(extract_mentions("").is_empty());
    }

    #[test]
    fn just_at_sign() {
        assert!(extract_mentions("@").is_empty());
    }

    #[test]
    fn no_mentions() {
        assert!(extract_mentions("nothing to see here").is_empty());
    }

    #[test]
    fn targets_did() {
        let m = Mention::Did("did:key:z123".into());
        assert!(m.targets("did:key:z123"));
        assert!(!m.targets("did:key:z456"));
    }

    #[test]
    fn everyone_targets_all() {
        assert!(Mention::Everyone.targets("did:key:z123"));
        assert!(Mention::Here.targets("did:key:z456"));
    }

    #[test]
    fn is_broadcast() {
        assert!(Mention::Everyone.is_broadcast());
        assert!(Mention::Here.is_broadcast());
        assert!(!Mention::Did("x".into()).is_broadcast());
        assert!(!Mention::Name("x".into()).is_broadcast());
    }

    #[test]
    fn render_did_mentions() {
        let text = "Hey @did:key:z123, meet @did:key:z456";
        let rendered = render_mentions(text, &|did| match did {
            "did:key:z123" => Some("alice".into()),
            "did:key:z456" => Some("bob".into()),
            _ => None,
        });
        assert_eq!(rendered, "Hey @alice, meet @bob");
    }

    #[test]
    fn render_unknown_did_unchanged() {
        let text = "Hey @did:key:zunknown";
        let rendered = render_mentions(text, &|_| None);
        assert_eq!(rendered, text);
    }

    #[test]
    fn mention_count_works() {
        assert_eq!(mention_count("@alice @bob hello @everyone"), 3);
        assert_eq!(mention_count("no mentions"), 0);
    }

    #[test]
    fn is_mentioned_direct() {
        assert!(is_mentioned(
            "Hey @did:key:z123",
            "did:key:z123"
        ));
        assert!(!is_mentioned(
            "Hey @did:key:z123",
            "did:key:z456"
        ));
    }

    #[test]
    fn is_mentioned_broadcast() {
        assert!(is_mentioned("@everyone check this", "did:key:z123"));
        assert!(is_mentioned("@here quick sync", "did:key:z456"));
    }

    #[test]
    fn name_with_underscores_and_hyphens() {
        let mentions = extract_mentions("@alice_bob @carol-dave");
        assert_eq!(mentions.len(), 2);
        assert_eq!(mentions[0], Mention::Name("alice_bob".into()));
        assert_eq!(mentions[1], Mention::Name("carol-dave".into()));
    }

    #[test]
    fn everyone_case_insensitive() {
        assert_eq!(extract_mentions("@Everyone")[0], Mention::Everyone);
        assert_eq!(extract_mentions("@EVERYONE")[0], Mention::Everyone);
    }

    #[test]
    fn here_case_insensitive() {
        assert_eq!(extract_mentions("@Here")[0], Mention::Here);
        assert_eq!(extract_mentions("@HERE")[0], Mention::Here);
    }

    #[test]
    fn mention_serializes() {
        let mention = Mention::Did("did:key:z123".into());
        let json = serde_json::to_string(&mention).unwrap();
        let restored: Mention = serde_json::from_str(&json).unwrap();
        assert_eq!(restored, mention);
    }
}
