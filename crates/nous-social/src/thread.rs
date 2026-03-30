//! Conversation threading: resolves flat event lists into tree structures.
//!
//! Posts reference their parent via `e` tags. This module builds thread trees,
//! computes depths, and provides traversal utilities.

use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use crate::event::{EventKind, SignedEvent};

/// A node in a conversation thread tree.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ThreadNode {
    pub event_id: String,
    pub author: String,
    pub content: String,
    pub parent_id: Option<String>,
    pub depth: usize,
    pub reply_count: usize,
    pub children: Vec<String>,
}

/// A resolved conversation thread.
#[derive(Debug)]
pub struct Thread {
    pub root_id: String,
    nodes: HashMap<String, ThreadNode>,
    order: Vec<String>,
}

impl Thread {
    /// Build a thread from a collection of events.
    /// The root is the event with no parent (or the oldest event if all are replies).
    pub fn from_events(events: &[SignedEvent]) -> Option<Self> {
        if events.is_empty() {
            return None;
        }

        let event_ids: std::collections::HashSet<&str> =
            events.iter().map(|e| e.id.as_str()).collect();

        let mut nodes: HashMap<String, ThreadNode> = HashMap::new();

        // First pass: create nodes
        for event in events {
            if event.kind != EventKind::TextNote {
                continue;
            }
            let parent = event
                .referenced_events()
                .into_iter()
                .find(|id| event_ids.contains(id))
                .map(|s| s.to_string());

            nodes.insert(
                event.id.clone(),
                ThreadNode {
                    event_id: event.id.clone(),
                    author: event.pubkey.clone(),
                    content: event.content.clone(),
                    parent_id: parent,
                    depth: 0,
                    reply_count: 0,
                    children: Vec::new(),
                },
            );
        }

        // Second pass: link children
        let ids: Vec<String> = nodes.keys().cloned().collect();
        for id in &ids {
            if let Some(parent_id) = nodes.get(id).and_then(|n| n.parent_id.clone())
                && let Some(parent) = nodes.get_mut(&parent_id)
            {
                parent.children.push(id.clone());
                parent.reply_count += 1;
            }
        }

        // Find root (no parent within the set)
        let root_id = nodes
            .values()
            .filter(|n| n.parent_id.is_none())
            .min_by_key(|n| {
                events
                    .iter()
                    .find(|e| e.id == n.event_id)
                    .map(|e| e.created_at)
            })
            .map(|n| n.event_id.clone())?;

        // Compute depths via BFS
        let mut queue = std::collections::VecDeque::new();
        queue.push_back((root_id.clone(), 0usize));
        while let Some((id, depth)) = queue.pop_front() {
            if let Some(node) = nodes.get_mut(&id) {
                node.depth = depth;
                for child_id in node.children.clone() {
                    queue.push_back((child_id, depth + 1));
                }
            }
        }

        // Build DFS order for display
        let mut order = Vec::new();
        let mut stack = vec![root_id.clone()];
        while let Some(id) = stack.pop() {
            order.push(id.clone());
            if let Some(node) = nodes.get(&id) {
                // Push children in reverse so first child is processed first
                for child_id in node.children.iter().rev() {
                    stack.push(child_id.clone());
                }
            }
        }

        Some(Self {
            root_id,
            nodes,
            order,
        })
    }

    /// Get a node by event ID.
    pub fn get(&self, event_id: &str) -> Option<&ThreadNode> {
        self.nodes.get(event_id)
    }

    /// The root node.
    pub fn root(&self) -> Option<&ThreadNode> {
        self.nodes.get(&self.root_id)
    }

    /// Iterate nodes in display order (DFS).
    pub fn iter_display_order(&self) -> impl Iterator<Item = &ThreadNode> {
        self.order.iter().filter_map(|id| self.nodes.get(id))
    }

    /// Total number of nodes.
    pub fn len(&self) -> usize {
        self.nodes.len()
    }

    /// Whether the thread is empty.
    pub fn is_empty(&self) -> bool {
        self.nodes.is_empty()
    }

    /// Maximum depth in the thread.
    pub fn max_depth(&self) -> usize {
        self.nodes.values().map(|n| n.depth).max().unwrap_or(0)
    }

    /// All direct replies to a given event.
    pub fn replies_to(&self, event_id: &str) -> Vec<&ThreadNode> {
        self.nodes
            .get(event_id)
            .map(|n| {
                n.children
                    .iter()
                    .filter_map(|id| self.nodes.get(id))
                    .collect()
            })
            .unwrap_or_default()
    }

    /// All unique authors in the thread.
    pub fn participants(&self) -> Vec<&str> {
        let mut authors: Vec<&str> = self.nodes.values().map(|n| n.author.as_str()).collect();
        authors.sort();
        authors.dedup();
        authors
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::event::{EventKind, SignedEvent, Tag};

    fn make_event(id: &str, author: &str, content: &str, reply_to: Option<&str>) -> SignedEvent {
        let tags = match reply_to {
            Some(parent) => vec![Tag::event(parent)],
            None => vec![],
        };
        let mut event = SignedEvent::new(author, EventKind::TextNote, content, tags);
        event.id = id.to_string();
        event
    }

    #[test]
    fn empty_events_returns_none() {
        assert!(Thread::from_events(&[]).is_none());
    }

    #[test]
    fn single_post_thread() {
        let events = vec![make_event("root", "alice", "Hello", None)];
        let thread = Thread::from_events(&events).unwrap();
        assert_eq!(thread.root_id, "root");
        assert_eq!(thread.len(), 1);
        assert_eq!(thread.max_depth(), 0);
    }

    #[test]
    fn linear_thread() {
        let events = vec![
            make_event("a", "alice", "Start", None),
            make_event("b", "bob", "Reply 1", Some("a")),
            make_event("c", "carol", "Reply 2", Some("b")),
        ];
        let thread = Thread::from_events(&events).unwrap();

        assert_eq!(thread.root_id, "a");
        assert_eq!(thread.len(), 3);
        assert_eq!(thread.max_depth(), 2);

        assert_eq!(thread.get("a").unwrap().depth, 0);
        assert_eq!(thread.get("b").unwrap().depth, 1);
        assert_eq!(thread.get("c").unwrap().depth, 2);
    }

    #[test]
    fn branching_thread() {
        let events = vec![
            make_event("root", "alice", "Topic", None),
            make_event("r1", "bob", "Branch 1", Some("root")),
            make_event("r2", "carol", "Branch 2", Some("root")),
            make_event("r1a", "dave", "Reply to Branch 1", Some("r1")),
        ];
        let thread = Thread::from_events(&events).unwrap();

        assert_eq!(thread.len(), 4);
        assert_eq!(thread.max_depth(), 2);
        assert_eq!(thread.replies_to("root").len(), 2);
        assert_eq!(thread.replies_to("r1").len(), 1);
        assert_eq!(thread.replies_to("r2").len(), 0);
    }

    #[test]
    fn root_reply_count() {
        let events = vec![
            make_event("root", "alice", "OP", None),
            make_event("r1", "bob", "Reply 1", Some("root")),
            make_event("r2", "carol", "Reply 2", Some("root")),
        ];
        let thread = Thread::from_events(&events).unwrap();
        assert_eq!(thread.root().unwrap().reply_count, 2);
    }

    #[test]
    fn display_order_is_dfs() {
        let events = vec![
            make_event("root", "alice", "Root", None),
            make_event("a", "bob", "A", Some("root")),
            make_event("b", "carol", "B", Some("root")),
            make_event("a1", "dave", "A1", Some("a")),
        ];
        let thread = Thread::from_events(&events).unwrap();
        let order: Vec<&str> = thread
            .iter_display_order()
            .map(|n| n.event_id.as_str())
            .collect();
        assert_eq!(order[0], "root");
        // DFS: root → a → a1 → b
        assert_eq!(order.len(), 4);
        // a1 should come before b (depth-first into a's children)
        let a1_pos = order.iter().position(|&id| id == "a1").unwrap();
        let b_pos = order.iter().position(|&id| id == "b").unwrap();
        assert!(a1_pos < b_pos);
    }

    #[test]
    fn participants_are_unique() {
        let events = vec![
            make_event("a", "alice", "1", None),
            make_event("b", "alice", "2", Some("a")),
            make_event("c", "bob", "3", Some("a")),
        ];
        let thread = Thread::from_events(&events).unwrap();
        let parts = thread.participants();
        assert_eq!(parts.len(), 2);
        assert!(parts.contains(&"alice"));
        assert!(parts.contains(&"bob"));
    }

    #[test]
    fn non_text_events_excluded() {
        let events = vec![make_event("root", "alice", "Hello", None), {
            let mut e = SignedEvent::new("bob", EventKind::Reaction, "+", vec![Tag::event("root")]);
            e.id = "reaction".into();
            e
        }];
        let thread = Thread::from_events(&events).unwrap();
        assert_eq!(thread.len(), 1); // only the text note
    }
}
