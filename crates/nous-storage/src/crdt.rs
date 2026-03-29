use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GCounter {
    counts: HashMap<String, u64>,
}

impl GCounter {
    pub fn new() -> Self {
        Self {
            counts: HashMap::new(),
        }
    }

    pub fn increment(&mut self, node_id: &str) {
        *self.counts.entry(node_id.to_string()).or_insert(0) += 1;
    }

    pub fn increment_by(&mut self, node_id: &str, amount: u64) {
        *self.counts.entry(node_id.to_string()).or_insert(0) += amount;
    }

    pub fn value(&self) -> u64 {
        self.counts.values().sum()
    }

    pub fn merge(&mut self, other: &GCounter) {
        for (node, &count) in &other.counts {
            let entry = self.counts.entry(node.clone()).or_insert(0);
            *entry = (*entry).max(count);
        }
    }
}

impl Default for GCounter {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LWWRegister<T: Clone> {
    value: Option<T>,
    timestamp: u64,
    node_id: String,
}

impl<T: Clone> LWWRegister<T> {
    pub fn new(node_id: impl Into<String>) -> Self {
        Self {
            value: None,
            timestamp: 0,
            node_id: node_id.into(),
        }
    }

    pub fn set(&mut self, value: T, timestamp: u64) {
        if timestamp >= self.timestamp {
            self.value = Some(value);
            self.timestamp = timestamp;
        }
    }

    pub fn get(&self) -> Option<&T> {
        self.value.as_ref()
    }

    pub fn timestamp(&self) -> u64 {
        self.timestamp
    }

    pub fn merge(&mut self, other: &LWWRegister<T>) {
        if other.timestamp > self.timestamp
            || (other.timestamp == self.timestamp && other.node_id > self.node_id)
        {
            self.value = other.value.clone();
            self.timestamp = other.timestamp;
            self.node_id = other.node_id.clone();
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ORSet<T: Clone + Eq + std::hash::Hash> {
    entries: HashMap<String, HashSet<String>>,
    tombstones: HashMap<String, HashSet<String>>,
    _phantom: std::marker::PhantomData<T>,
    values: HashMap<String, T>,
}

impl<T: Clone + Eq + std::hash::Hash + Serialize> ORSet<T> {
    pub fn new() -> Self {
        Self {
            entries: HashMap::new(),
            tombstones: HashMap::new(),
            _phantom: std::marker::PhantomData,
            values: HashMap::new(),
        }
    }

    pub fn add(&mut self, value: T, node_id: &str) {
        let tag = format!("{}:{}", node_id, uuid::Uuid::new_v4());
        let key = serde_json::to_string(&value).unwrap_or_default();

        self.entries.entry(key.clone()).or_default().insert(tag);
        self.values.insert(key, value);
    }

    pub fn remove(&mut self, value: &T) {
        let key = serde_json::to_string(value).unwrap_or_default();
        if let Some(tags) = self.entries.remove(&key) {
            self.tombstones.entry(key.clone()).or_default().extend(tags);
            self.values.remove(&key);
        }
    }

    pub fn contains(&self, value: &T) -> bool {
        let key = serde_json::to_string(value).unwrap_or_default();
        self.entries.contains_key(&key)
    }

    pub fn elements(&self) -> Vec<&T> {
        self.values.values().collect()
    }

    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    pub fn merge(&mut self, other: &ORSet<T>) {
        for (key, tags) in &other.entries {
            let entry = self.entries.entry(key.clone()).or_default();
            entry.extend(tags.iter().cloned());

            if let Some(value) = other.values.get(key) {
                self.values.insert(key.clone(), value.clone());
            }
        }

        for (key, tags) in &other.tombstones {
            let tomb = self.tombstones.entry(key.clone()).or_default();
            tomb.extend(tags.iter().cloned());
        }

        // Remove tombstoned tags from entries
        for (key, tomb_tags) in &self.tombstones {
            if let Some(entry_tags) = self.entries.get_mut(key) {
                for tag in tomb_tags {
                    entry_tags.remove(tag);
                }
                if entry_tags.is_empty() {
                    self.entries.remove(key);
                    self.values.remove(key);
                }
            }
        }
    }
}

impl<T: Clone + Eq + std::hash::Hash + Serialize> Default for ORSet<T> {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // --- GCounter tests ---

    #[test]
    fn gcounter_starts_at_zero() {
        let c = GCounter::new();
        assert_eq!(c.value(), 0);
    }

    #[test]
    fn gcounter_increment() {
        let mut c = GCounter::new();
        c.increment("node-a");
        c.increment("node-a");
        c.increment("node-b");
        assert_eq!(c.value(), 3);
    }

    #[test]
    fn gcounter_increment_by() {
        let mut c = GCounter::new();
        c.increment_by("node-a", 10);
        c.increment_by("node-b", 5);
        assert_eq!(c.value(), 15);
    }

    #[test]
    fn gcounter_merge_takes_max() {
        let mut a = GCounter::new();
        a.increment_by("node-1", 10);
        a.increment_by("node-2", 5);

        let mut b = GCounter::new();
        b.increment_by("node-1", 3);
        b.increment_by("node-2", 8);
        b.increment_by("node-3", 2);

        a.merge(&b);

        assert_eq!(a.value(), 20); // max(10,3) + max(5,8) + 2
    }

    #[test]
    fn gcounter_merge_is_commutative() {
        let mut a = GCounter::new();
        a.increment_by("x", 5);

        let mut b = GCounter::new();
        b.increment_by("y", 3);

        let mut ab = a.clone();
        ab.merge(&b);

        let mut ba = b.clone();
        ba.merge(&a);

        assert_eq!(ab.value(), ba.value());
    }

    #[test]
    fn gcounter_merge_is_idempotent() {
        let mut a = GCounter::new();
        a.increment_by("x", 5);

        let mut b = a.clone();
        b.merge(&a);
        b.merge(&a);

        assert_eq!(b.value(), 5);
    }

    // --- LWWRegister tests ---

    #[test]
    fn lww_register_starts_empty() {
        let r: LWWRegister<String> = LWWRegister::new("node-a");
        assert!(r.get().is_none());
    }

    #[test]
    fn lww_register_set_and_get() {
        let mut r = LWWRegister::new("node-a");
        r.set("hello".to_string(), 1);
        assert_eq!(r.get(), Some(&"hello".to_string()));
    }

    #[test]
    fn lww_register_last_write_wins() {
        let mut r = LWWRegister::new("node-a");
        r.set("first".to_string(), 1);
        r.set("second".to_string(), 2);
        assert_eq!(r.get(), Some(&"second".to_string()));
    }

    #[test]
    fn lww_register_ignores_old_writes() {
        let mut r = LWWRegister::new("node-a");
        r.set("new".to_string(), 10);
        r.set("old".to_string(), 5);
        assert_eq!(r.get(), Some(&"new".to_string()));
    }

    #[test]
    fn lww_register_merge_takes_latest() {
        let mut a = LWWRegister::new("node-a");
        a.set("old".to_string(), 1);

        let mut b = LWWRegister::new("node-b");
        b.set("new".to_string(), 2);

        a.merge(&b);
        assert_eq!(a.get(), Some(&"new".to_string()));
    }

    // --- ORSet tests ---

    #[test]
    fn orset_starts_empty() {
        let s: ORSet<String> = ORSet::new();
        assert!(s.is_empty());
        assert_eq!(s.len(), 0);
    }

    #[test]
    fn orset_add_and_contains() {
        let mut s = ORSet::new();
        s.add("hello".to_string(), "node-a");
        assert!(s.contains(&"hello".to_string()));
        assert!(!s.contains(&"world".to_string()));
    }

    #[test]
    fn orset_remove() {
        let mut s = ORSet::new();
        s.add("item".to_string(), "node-a");
        assert!(s.contains(&"item".to_string()));

        s.remove(&"item".to_string());
        assert!(!s.contains(&"item".to_string()));
        assert!(s.is_empty());
    }

    #[test]
    fn orset_add_after_remove() {
        let mut s = ORSet::new();
        s.add("item".to_string(), "node-a");
        s.remove(&"item".to_string());
        s.add("item".to_string(), "node-a");
        assert!(s.contains(&"item".to_string()));
    }

    #[test]
    fn orset_merge_concurrent_add() {
        let mut a = ORSet::new();
        a.add("from-a".to_string(), "node-a");

        let mut b = ORSet::new();
        b.add("from-b".to_string(), "node-b");

        a.merge(&b);
        assert!(a.contains(&"from-a".to_string()));
        assert!(a.contains(&"from-b".to_string()));
        assert_eq!(a.len(), 2);
    }

    #[test]
    fn orset_merge_concurrent_add_remove() {
        let mut base = ORSet::new();
        base.add("shared".to_string(), "node-a");

        let mut a = base.clone();
        a.remove(&"shared".to_string()); // a removes

        let mut b = base.clone();
        b.add("shared".to_string(), "node-b"); // b re-adds concurrently

        a.merge(&b);
        // add wins over concurrent remove in OR-Set
        assert!(a.contains(&"shared".to_string()));
    }

    #[test]
    fn orset_elements() {
        let mut s = ORSet::new();
        s.add(1, "node-a");
        s.add(2, "node-a");
        s.add(3, "node-b");

        let mut elements: Vec<i32> = s.elements().into_iter().copied().collect();
        elements.sort();
        assert_eq!(elements, vec![1, 2, 3]);
    }
}
