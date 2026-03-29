use std::collections::{HashMap, HashSet};

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FollowGraph {
    following: HashMap<String, HashSet<String>>,
    followers: HashMap<String, HashSet<String>>,
}

impl FollowGraph {
    pub fn new() -> Self {
        Self {
            following: HashMap::new(),
            followers: HashMap::new(),
        }
    }

    pub fn follow(&mut self, follower: &str, target: &str) -> bool {
        if follower == target {
            return false;
        }
        let added = self
            .following
            .entry(follower.to_string())
            .or_default()
            .insert(target.to_string());

        if added {
            self.followers
                .entry(target.to_string())
                .or_default()
                .insert(follower.to_string());
        }
        added
    }

    pub fn unfollow(&mut self, follower: &str, target: &str) -> bool {
        let removed = self
            .following
            .get_mut(follower)
            .map(|set| set.remove(target))
            .unwrap_or(false);

        if removed && let Some(set) = self.followers.get_mut(target) {
            set.remove(follower);
        }
        removed
    }

    pub fn is_following(&self, follower: &str, target: &str) -> bool {
        self.following
            .get(follower)
            .map(|set| set.contains(target))
            .unwrap_or(false)
    }

    pub fn following_of(&self, user: &str) -> Vec<&str> {
        self.following
            .get(user)
            .map(|set| set.iter().map(|s| s.as_str()).collect())
            .unwrap_or_default()
    }

    pub fn followers_of(&self, user: &str) -> Vec<&str> {
        self.followers
            .get(user)
            .map(|set| set.iter().map(|s| s.as_str()).collect())
            .unwrap_or_default()
    }

    pub fn following_count(&self, user: &str) -> usize {
        self.following.get(user).map(|s| s.len()).unwrap_or(0)
    }

    pub fn followers_count(&self, user: &str) -> usize {
        self.followers.get(user).map(|s| s.len()).unwrap_or(0)
    }

    pub fn mutual_follows(&self, user_a: &str, user_b: &str) -> bool {
        self.is_following(user_a, user_b) && self.is_following(user_b, user_a)
    }

    pub fn suggested_follows(&self, user: &str, limit: usize) -> Vec<String> {
        let my_following = self.following_of(user);
        let my_following_set: HashSet<&str> = my_following.iter().copied().collect();

        let mut candidates: HashMap<&str, usize> = HashMap::new();

        for followed in &my_following {
            for their_following in self.following_of(followed) {
                if their_following != user && !my_following_set.contains(their_following) {
                    *candidates.entry(their_following).or_insert(0) += 1;
                }
            }
        }

        let mut sorted: Vec<_> = candidates.into_iter().collect();
        sorted.sort_by(|a, b| b.1.cmp(&a.1));
        sorted
            .into_iter()
            .take(limit)
            .map(|(k, _)| k.to_string())
            .collect()
    }
}

impl Default for FollowGraph {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn follow_and_unfollow() {
        let mut graph = FollowGraph::new();
        assert!(graph.follow("alice", "bob"));
        assert!(graph.is_following("alice", "bob"));
        assert!(!graph.is_following("bob", "alice"));

        assert!(graph.unfollow("alice", "bob"));
        assert!(!graph.is_following("alice", "bob"));
    }

    #[test]
    fn cannot_follow_self() {
        let mut graph = FollowGraph::new();
        assert!(!graph.follow("alice", "alice"));
    }

    #[test]
    fn duplicate_follow_returns_false() {
        let mut graph = FollowGraph::new();
        assert!(graph.follow("alice", "bob"));
        assert!(!graph.follow("alice", "bob"));
    }

    #[test]
    fn unfollow_nonexistent_returns_false() {
        let mut graph = FollowGraph::new();
        assert!(!graph.unfollow("alice", "bob"));
    }

    #[test]
    fn followers_and_following() {
        let mut graph = FollowGraph::new();
        graph.follow("alice", "carol");
        graph.follow("bob", "carol");
        graph.follow("alice", "bob");

        assert_eq!(graph.following_count("alice"), 2);
        assert_eq!(graph.followers_count("carol"), 2);
        assert_eq!(graph.followers_count("bob"), 1);
    }

    #[test]
    fn mutual_follows() {
        let mut graph = FollowGraph::new();
        graph.follow("alice", "bob");
        assert!(!graph.mutual_follows("alice", "bob"));

        graph.follow("bob", "alice");
        assert!(graph.mutual_follows("alice", "bob"));
    }

    #[test]
    fn suggested_follows() {
        let mut graph = FollowGraph::new();
        // alice follows bob and carol
        graph.follow("alice", "bob");
        graph.follow("alice", "carol");
        // bob follows dave
        graph.follow("bob", "dave");
        // carol follows dave and eve
        graph.follow("carol", "dave");
        graph.follow("carol", "eve");

        let suggestions = graph.suggested_follows("alice", 10);
        // dave should be top suggestion (followed by both bob and carol)
        assert!(!suggestions.is_empty());
        assert_eq!(suggestions[0], "dave");
        assert!(suggestions.contains(&"eve".to_string()));
    }

    #[test]
    fn suggested_follows_excludes_already_following() {
        let mut graph = FollowGraph::new();
        graph.follow("alice", "bob");
        graph.follow("bob", "carol");
        graph.follow("alice", "carol");

        let suggestions = graph.suggested_follows("alice", 10);
        assert!(!suggestions.contains(&"carol".to_string()));
    }

    #[test]
    fn serializes() {
        let mut graph = FollowGraph::new();
        graph.follow("alice", "bob");
        let json = serde_json::to_string(&graph).unwrap();
        let restored: FollowGraph = serde_json::from_str(&json).unwrap();
        assert!(restored.is_following("alice", "bob"));
    }

    #[test]
    fn empty_graph_stats() {
        let graph = FollowGraph::new();
        assert_eq!(graph.following_count("nobody"), 0);
        assert_eq!(graph.followers_count("nobody"), 0);
        assert!(graph.following_of("nobody").is_empty());
        assert!(graph.followers_of("nobody").is_empty());
    }
}
