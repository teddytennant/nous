//! CRDT (Conflict-free Replicated Data Types) for offline-first web sync.
//!
//! Standalone WASM-compatible implementations: GCounter, PNCounter,
//! LWWRegister, LWWMap. All types support JSON serialization for
//! cross-tab and cross-device state transfer.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use wasm_bindgen::prelude::*;

// ── GCounter ─────────────────────────────────────────────────────

/// Grow-only counter. Each node maintains its own count; the total
/// is the sum across all nodes. Merge takes the per-node maximum.
#[derive(Debug, Clone, Serialize, Deserialize)]
struct GCounterInner {
    counts: HashMap<String, u64>,
}

#[wasm_bindgen]
pub struct WasmGCounter {
    inner: GCounterInner,
}

#[wasm_bindgen]
impl WasmGCounter {
    /// Create a new zero-valued counter.
    #[wasm_bindgen(constructor)]
    #[allow(clippy::new_without_default)]
    pub fn new() -> Self {
        Self {
            inner: GCounterInner {
                counts: HashMap::new(),
            },
        }
    }

    /// Increment this node's count by 1.
    pub fn increment(&mut self, node_id: &str) {
        *self.inner.counts.entry(node_id.to_string()).or_insert(0) += 1;
    }

    /// Increment this node's count by a given amount.
    #[wasm_bindgen(js_name = incrementBy)]
    pub fn increment_by(&mut self, node_id: &str, amount: u64) {
        *self.inner.counts.entry(node_id.to_string()).or_insert(0) += amount;
    }

    /// The total count across all nodes.
    pub fn value(&self) -> u64 {
        self.inner.counts.values().sum()
    }

    /// Get the count for a specific node.
    #[wasm_bindgen(js_name = nodeValue)]
    pub fn node_value(&self, node_id: &str) -> u64 {
        self.inner.counts.get(node_id).copied().unwrap_or(0)
    }

    /// Number of contributing nodes.
    #[wasm_bindgen(js_name = nodeCount)]
    pub fn node_count(&self) -> usize {
        self.inner.counts.len()
    }

    /// Merge another counter into this one. Per-node maximum wins.
    pub fn merge(&mut self, other: &WasmGCounter) {
        for (node, &count) in &other.inner.counts {
            let entry = self.inner.counts.entry(node.clone()).or_insert(0);
            *entry = (*entry).max(count);
        }
    }

    /// Serialize to JSON for transport.
    #[wasm_bindgen(js_name = toJson)]
    pub fn to_json(&self) -> String {
        serde_json::to_string(&self.inner).unwrap_or_default()
    }

    /// Deserialize from JSON.
    #[wasm_bindgen(js_name = fromJson)]
    pub fn from_json(json: &str) -> Result<WasmGCounter, JsError> {
        let inner: GCounterInner =
            serde_json::from_str(json).map_err(|e| JsError::new(&format!("invalid JSON: {e}")))?;
        Ok(Self { inner })
    }
}

// ── PNCounter ────────────────────────────────────────────────────

/// Positive-Negative counter: supports increment and decrement via
/// two internal GCounters.
#[derive(Debug, Clone, Serialize, Deserialize)]
struct PNCounterInner {
    positive: HashMap<String, u64>,
    negative: HashMap<String, u64>,
}

#[wasm_bindgen]
pub struct WasmPNCounter {
    inner: PNCounterInner,
}

#[wasm_bindgen]
impl WasmPNCounter {
    /// Create a new zero-valued counter.
    #[wasm_bindgen(constructor)]
    #[allow(clippy::new_without_default)]
    pub fn new() -> Self {
        Self {
            inner: PNCounterInner {
                positive: HashMap::new(),
                negative: HashMap::new(),
            },
        }
    }

    /// Increment by 1.
    pub fn increment(&mut self, node_id: &str) {
        *self.inner.positive.entry(node_id.to_string()).or_insert(0) += 1;
    }

    /// Decrement by 1.
    pub fn decrement(&mut self, node_id: &str) {
        *self.inner.negative.entry(node_id.to_string()).or_insert(0) += 1;
    }

    /// Increment by a given amount.
    #[wasm_bindgen(js_name = incrementBy)]
    pub fn increment_by(&mut self, node_id: &str, amount: u64) {
        *self.inner.positive.entry(node_id.to_string()).or_insert(0) += amount;
    }

    /// Decrement by a given amount.
    #[wasm_bindgen(js_name = decrementBy)]
    pub fn decrement_by(&mut self, node_id: &str, amount: u64) {
        *self.inner.negative.entry(node_id.to_string()).or_insert(0) += amount;
    }

    /// The net value (total increments minus total decrements).
    pub fn value(&self) -> f64 {
        let pos: u64 = self.inner.positive.values().sum();
        let neg: u64 = self.inner.negative.values().sum();
        pos as f64 - neg as f64
    }

    /// Merge another PN counter. Per-node maximum on both sides.
    pub fn merge(&mut self, other: &WasmPNCounter) {
        for (node, &count) in &other.inner.positive {
            let entry = self.inner.positive.entry(node.clone()).or_insert(0);
            *entry = (*entry).max(count);
        }
        for (node, &count) in &other.inner.negative {
            let entry = self.inner.negative.entry(node.clone()).or_insert(0);
            *entry = (*entry).max(count);
        }
    }

    /// Serialize to JSON.
    #[wasm_bindgen(js_name = toJson)]
    pub fn to_json(&self) -> String {
        serde_json::to_string(&self.inner).unwrap_or_default()
    }

    /// Deserialize from JSON.
    #[wasm_bindgen(js_name = fromJson)]
    pub fn from_json(json: &str) -> Result<WasmPNCounter, JsError> {
        let inner: PNCounterInner =
            serde_json::from_str(json).map_err(|e| JsError::new(&format!("invalid JSON: {e}")))?;
        Ok(Self { inner })
    }
}

// ── LWWRegister ──────────────────────────────────────────────────

/// Last-Write-Wins Register for a string value. Concurrent writes
/// with equal timestamps break ties by node ID (lexicographic max).
#[derive(Debug, Clone, Serialize, Deserialize)]
struct LWWRegisterInner {
    value: Option<String>,
    timestamp: f64,
    node_id: String,
}

#[wasm_bindgen]
pub struct WasmLWWRegister {
    inner: LWWRegisterInner,
}

#[wasm_bindgen]
impl WasmLWWRegister {
    /// Create a new empty register for the given node.
    #[wasm_bindgen(constructor)]
    pub fn new(node_id: &str) -> Self {
        Self {
            inner: LWWRegisterInner {
                value: None,
                timestamp: 0.0,
                node_id: node_id.to_string(),
            },
        }
    }

    /// Set the value with a timestamp (e.g., `Date.now()` in JS).
    pub fn set(&mut self, value: &str, timestamp: f64) {
        if timestamp >= self.inner.timestamp {
            self.inner.value = Some(value.to_string());
            self.inner.timestamp = timestamp;
        }
    }

    /// Get the current value, or null if unset.
    pub fn get(&self) -> Option<String> {
        self.inner.value.clone()
    }

    /// The timestamp of the last accepted write.
    pub fn timestamp(&self) -> f64 {
        self.inner.timestamp
    }

    /// Whether the register has a value.
    #[wasm_bindgen(js_name = hasValue)]
    pub fn has_value(&self) -> bool {
        self.inner.value.is_some()
    }

    /// Merge another register. Latest timestamp wins; ties broken
    /// by node ID (lexicographic max).
    pub fn merge(&mut self, other: &WasmLWWRegister) {
        if other.inner.timestamp > self.inner.timestamp
            || (other.inner.timestamp == self.inner.timestamp
                && other.inner.node_id > self.inner.node_id)
        {
            self.inner.value = other.inner.value.clone();
            self.inner.timestamp = other.inner.timestamp;
            self.inner.node_id = other.inner.node_id.clone();
        }
    }

    /// Serialize to JSON.
    #[wasm_bindgen(js_name = toJson)]
    pub fn to_json(&self) -> String {
        serde_json::to_string(&self.inner).unwrap_or_default()
    }

    /// Deserialize from JSON.
    #[wasm_bindgen(js_name = fromJson)]
    pub fn from_json(json: &str) -> Result<WasmLWWRegister, JsError> {
        let inner: LWWRegisterInner =
            serde_json::from_str(json).map_err(|e| JsError::new(&format!("invalid JSON: {e}")))?;
        Ok(Self { inner })
    }
}

// ── LWWMap ───────────────────────────────────────────────────────

/// Last-Write-Wins Map with string keys and string values. Each key
/// has an independent LWW timestamp. Supports set, remove, and merge.
#[derive(Debug, Clone, Serialize, Deserialize)]
struct LWWMapEntry {
    value: Option<String>,
    timestamp: f64,
    node_id: String,
    deleted: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct LWWMapInner {
    entries: HashMap<String, LWWMapEntry>,
}

#[wasm_bindgen]
pub struct WasmLWWMap {
    inner: LWWMapInner,
}

#[wasm_bindgen]
impl WasmLWWMap {
    /// Create a new empty map.
    #[wasm_bindgen(constructor)]
    #[allow(clippy::new_without_default)]
    pub fn new() -> Self {
        Self {
            inner: LWWMapInner {
                entries: HashMap::new(),
            },
        }
    }

    /// Set a key-value pair with a timestamp.
    pub fn set(&mut self, key: &str, value: &str, timestamp: f64, node_id: &str) {
        let entry = self
            .inner
            .entries
            .entry(key.to_string())
            .or_insert(LWWMapEntry {
                value: None,
                timestamp: 0.0,
                node_id: String::new(),
                deleted: false,
            });
        if timestamp > entry.timestamp
            || (timestamp == entry.timestamp && node_id > entry.node_id.as_str())
        {
            entry.value = Some(value.to_string());
            entry.timestamp = timestamp;
            entry.node_id = node_id.to_string();
            entry.deleted = false;
        }
    }

    /// Remove a key with a timestamp.
    pub fn remove(&mut self, key: &str, timestamp: f64, node_id: &str) {
        let entry = self
            .inner
            .entries
            .entry(key.to_string())
            .or_insert(LWWMapEntry {
                value: None,
                timestamp: 0.0,
                node_id: String::new(),
                deleted: false,
            });
        if timestamp > entry.timestamp
            || (timestamp == entry.timestamp && node_id > entry.node_id.as_str())
        {
            entry.value = None;
            entry.timestamp = timestamp;
            entry.node_id = node_id.to_string();
            entry.deleted = true;
        }
    }

    /// Get the value for a key, or null if absent/deleted.
    pub fn get(&self, key: &str) -> Option<String> {
        self.inner
            .entries
            .get(key)
            .and_then(|e| if e.deleted { None } else { e.value.clone() })
    }

    /// Whether the key exists and is not deleted.
    #[wasm_bindgen(js_name = hasKey)]
    pub fn has_key(&self, key: &str) -> bool {
        self.inner
            .entries
            .get(key)
            .is_some_and(|e| !e.deleted && e.value.is_some())
    }

    /// Number of active (non-deleted) entries.
    pub fn len(&self) -> usize {
        self.inner
            .entries
            .values()
            .filter(|e| !e.deleted && e.value.is_some())
            .count()
    }

    /// Whether the map is empty.
    #[wasm_bindgen(js_name = isEmpty)]
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Get all active keys as a JSON array.
    pub fn keys(&self) -> String {
        let keys: Vec<&str> = self
            .inner
            .entries
            .iter()
            .filter(|(_, e)| !e.deleted && e.value.is_some())
            .map(|(k, _)| k.as_str())
            .collect();
        serde_json::to_string(&keys).unwrap_or_default()
    }

    /// Get all active entries as a JSON object `{ key: value }`.
    pub fn entries(&self) -> String {
        let map: HashMap<&str, &str> = self
            .inner
            .entries
            .iter()
            .filter_map(|(k, e)| {
                if e.deleted {
                    None
                } else {
                    e.value.as_deref().map(|v| (k.as_str(), v))
                }
            })
            .collect();
        serde_json::to_string(&map).unwrap_or_default()
    }

    /// Merge another map. Per-key LWW semantics.
    pub fn merge(&mut self, other: &WasmLWWMap) {
        for (key, other_entry) in &other.inner.entries {
            let entry = self
                .inner
                .entries
                .entry(key.clone())
                .or_insert(LWWMapEntry {
                    value: None,
                    timestamp: 0.0,
                    node_id: String::new(),
                    deleted: false,
                });
            if other_entry.timestamp > entry.timestamp
                || (other_entry.timestamp == entry.timestamp
                    && other_entry.node_id > entry.node_id)
            {
                entry.value = other_entry.value.clone();
                entry.timestamp = other_entry.timestamp;
                entry.node_id = other_entry.node_id.clone();
                entry.deleted = other_entry.deleted;
            }
        }
    }

    /// Serialize to JSON.
    #[wasm_bindgen(js_name = toJson)]
    pub fn to_json(&self) -> String {
        serde_json::to_string(&self.inner).unwrap_or_default()
    }

    /// Deserialize from JSON.
    #[wasm_bindgen(js_name = fromJson)]
    pub fn from_json(json: &str) -> Result<WasmLWWMap, JsError> {
        let inner: LWWMapInner =
            serde_json::from_str(json).map_err(|e| JsError::new(&format!("invalid JSON: {e}")))?;
        Ok(Self { inner })
    }
}

// ── Tests ────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    // --- GCounter ---

    #[test]
    fn gcounter_starts_at_zero() {
        let c = WasmGCounter::new();
        assert_eq!(c.value(), 0);
        assert_eq!(c.node_count(), 0);
    }

    #[test]
    fn gcounter_increment() {
        let mut c = WasmGCounter::new();
        c.increment("a");
        c.increment("a");
        c.increment("b");
        assert_eq!(c.value(), 3);
        assert_eq!(c.node_value("a"), 2);
        assert_eq!(c.node_value("b"), 1);
        assert_eq!(c.node_count(), 2);
    }

    #[test]
    fn gcounter_increment_by() {
        let mut c = WasmGCounter::new();
        c.increment_by("a", 100);
        c.increment_by("b", 50);
        assert_eq!(c.value(), 150);
    }

    #[test]
    fn gcounter_node_value_missing() {
        let c = WasmGCounter::new();
        assert_eq!(c.node_value("nonexistent"), 0);
    }

    #[test]
    fn gcounter_merge_takes_max() {
        let mut a = WasmGCounter::new();
        a.increment_by("n1", 10);
        a.increment_by("n2", 5);

        let mut b = WasmGCounter::new();
        b.increment_by("n1", 3);
        b.increment_by("n2", 8);
        b.increment_by("n3", 2);

        a.merge(&b);
        assert_eq!(a.value(), 20); // max(10,3) + max(5,8) + 2
    }

    #[test]
    fn gcounter_merge_commutative() {
        let mut a = WasmGCounter::new();
        a.increment_by("x", 5);
        let mut b = WasmGCounter::new();
        b.increment_by("y", 3);

        let mut ab = a.inner.clone();
        for (n, &c) in &b.inner.counts {
            let e = ab.counts.entry(n.clone()).or_insert(0);
            *e = (*e).max(c);
        }
        let mut ba = b.inner.clone();
        for (n, &c) in &a.inner.counts {
            let e = ba.counts.entry(n.clone()).or_insert(0);
            *e = (*e).max(c);
        }

        let val_ab: u64 = ab.counts.values().sum();
        let val_ba: u64 = ba.counts.values().sum();
        assert_eq!(val_ab, val_ba);
    }

    #[test]
    fn gcounter_merge_idempotent() {
        let mut a = WasmGCounter::new();
        a.increment_by("x", 5);
        let snapshot = WasmGCounter {
            inner: a.inner.clone(),
        };
        a.merge(&snapshot);
        a.merge(&snapshot);
        assert_eq!(a.value(), 5);
    }

    #[test]
    fn gcounter_json_roundtrip() {
        let mut c = WasmGCounter::new();
        c.increment_by("a", 42);
        c.increment_by("b", 7);
        let json = c.to_json();
        let restored = WasmGCounter::from_json(&json).unwrap();
        assert_eq!(restored.value(), 49);
        assert_eq!(restored.node_value("a"), 42);
    }

    #[test]
    fn gcounter_json_rejects_invalid() {
        let result = std::panic::catch_unwind(|| WasmGCounter::from_json("not json"));
        assert!(result.is_err() || result.unwrap().is_err());
    }

    // --- PNCounter ---

    #[test]
    fn pncounter_starts_at_zero() {
        let c = WasmPNCounter::new();
        assert_eq!(c.value(), 0.0);
    }

    #[test]
    fn pncounter_increment_and_decrement() {
        let mut c = WasmPNCounter::new();
        c.increment("a");
        c.increment("a");
        c.decrement("a");
        assert_eq!(c.value(), 1.0);
    }

    #[test]
    fn pncounter_increment_by_decrement_by() {
        let mut c = WasmPNCounter::new();
        c.increment_by("a", 100);
        c.decrement_by("a", 30);
        assert_eq!(c.value(), 70.0);
    }

    #[test]
    fn pncounter_negative_value() {
        let mut c = WasmPNCounter::new();
        c.decrement_by("a", 10);
        assert_eq!(c.value(), -10.0);
    }

    #[test]
    fn pncounter_multi_node() {
        let mut c = WasmPNCounter::new();
        c.increment_by("a", 10);
        c.increment_by("b", 20);
        c.decrement_by("a", 5);
        c.decrement_by("b", 3);
        assert_eq!(c.value(), 22.0); // 30 - 8
    }

    #[test]
    fn pncounter_merge() {
        let mut a = WasmPNCounter::new();
        a.increment_by("n1", 10);
        a.decrement_by("n1", 2);

        let mut b = WasmPNCounter::new();
        b.increment_by("n2", 5);
        b.decrement_by("n2", 1);

        a.merge(&b);
        assert_eq!(a.value(), 12.0); // 10+5 - 2-1
    }

    #[test]
    fn pncounter_merge_commutative() {
        let mut a = WasmPNCounter::new();
        a.increment_by("x", 5);
        a.decrement("x");

        let mut b = WasmPNCounter::new();
        b.increment_by("y", 3);

        let mut ab = WasmPNCounter::new();
        ab.inner.positive = a.inner.positive.clone();
        ab.inner.negative = a.inner.negative.clone();
        ab.merge(&b);

        let mut ba = WasmPNCounter::new();
        ba.inner.positive = b.inner.positive.clone();
        ba.inner.negative = b.inner.negative.clone();
        ba.merge(&a);

        assert_eq!(ab.value(), ba.value());
    }

    #[test]
    fn pncounter_merge_idempotent() {
        let mut a = WasmPNCounter::new();
        a.increment_by("x", 5);
        a.decrement_by("x", 2);

        let snapshot = WasmPNCounter {
            inner: a.inner.clone(),
        };
        a.merge(&snapshot);
        a.merge(&snapshot);
        assert_eq!(a.value(), 3.0);
    }

    #[test]
    fn pncounter_json_roundtrip() {
        let mut c = WasmPNCounter::new();
        c.increment_by("a", 100);
        c.decrement_by("b", 30);
        let json = c.to_json();
        let restored = WasmPNCounter::from_json(&json).unwrap();
        assert_eq!(restored.value(), 70.0);
    }

    #[test]
    fn pncounter_json_rejects_invalid() {
        let result = std::panic::catch_unwind(|| WasmPNCounter::from_json("{bad}"));
        assert!(result.is_err() || result.unwrap().is_err());
    }

    // --- LWWRegister ---

    #[test]
    fn lww_register_starts_empty() {
        let r = WasmLWWRegister::new("node-a");
        assert!(r.get().is_none());
        assert!(!r.has_value());
        assert_eq!(r.timestamp(), 0.0);
    }

    #[test]
    fn lww_register_set_and_get() {
        let mut r = WasmLWWRegister::new("node-a");
        r.set("hello", 1.0);
        assert_eq!(r.get(), Some("hello".to_string()));
        assert!(r.has_value());
        assert_eq!(r.timestamp(), 1.0);
    }

    #[test]
    fn lww_register_last_write_wins() {
        let mut r = WasmLWWRegister::new("node-a");
        r.set("first", 1.0);
        r.set("second", 2.0);
        assert_eq!(r.get(), Some("second".to_string()));
    }

    #[test]
    fn lww_register_ignores_old_writes() {
        let mut r = WasmLWWRegister::new("node-a");
        r.set("new", 10.0);
        r.set("old", 5.0);
        assert_eq!(r.get(), Some("new".to_string()));
    }

    #[test]
    fn lww_register_equal_timestamp_accepts() {
        let mut r = WasmLWWRegister::new("node-a");
        r.set("first", 1.0);
        r.set("second", 1.0);
        // Equal timestamp is accepted (>=)
        assert_eq!(r.get(), Some("second".to_string()));
    }

    #[test]
    fn lww_register_merge_takes_latest() {
        let mut a = WasmLWWRegister::new("node-a");
        a.set("old", 1.0);

        let mut b = WasmLWWRegister::new("node-b");
        b.set("new", 2.0);

        a.merge(&b);
        assert_eq!(a.get(), Some("new".to_string()));
    }

    #[test]
    fn lww_register_merge_tiebreak_by_node_id() {
        let mut a = WasmLWWRegister::new("node-a");
        a.set("a-value", 1.0);

        let mut b = WasmLWWRegister::new("node-b");
        b.set("b-value", 1.0);

        a.merge(&b);
        // node-b > node-a, so b wins
        assert_eq!(a.get(), Some("b-value".to_string()));
    }

    #[test]
    fn lww_register_merge_no_overwrite_if_older() {
        let mut a = WasmLWWRegister::new("node-a");
        a.set("newer", 10.0);

        let mut b = WasmLWWRegister::new("node-b");
        b.set("older", 5.0);

        a.merge(&b);
        assert_eq!(a.get(), Some("newer".to_string()));
    }

    #[test]
    fn lww_register_json_roundtrip() {
        let mut r = WasmLWWRegister::new("node-a");
        r.set("persisted", 42.0);
        let json = r.to_json();
        let restored = WasmLWWRegister::from_json(&json).unwrap();
        assert_eq!(restored.get(), Some("persisted".to_string()));
        assert_eq!(restored.timestamp(), 42.0);
    }

    #[test]
    fn lww_register_json_roundtrip_empty() {
        let r = WasmLWWRegister::new("n");
        let json = r.to_json();
        let restored = WasmLWWRegister::from_json(&json).unwrap();
        assert!(!restored.has_value());
    }

    #[test]
    fn lww_register_json_rejects_invalid() {
        let result = std::panic::catch_unwind(|| WasmLWWRegister::from_json("nope"));
        assert!(result.is_err() || result.unwrap().is_err());
    }

    // --- LWWMap ---

    #[test]
    fn lwwmap_starts_empty() {
        let m = WasmLWWMap::new();
        assert!(m.is_empty());
        assert_eq!(m.len(), 0);
    }

    #[test]
    fn lwwmap_set_and_get() {
        let mut m = WasmLWWMap::new();
        m.set("key", "value", 1.0, "node-a");
        assert_eq!(m.get("key"), Some("value".to_string()));
        assert!(m.has_key("key"));
        assert_eq!(m.len(), 1);
    }

    #[test]
    fn lwwmap_latest_wins() {
        let mut m = WasmLWWMap::new();
        m.set("k", "old", 1.0, "n");
        m.set("k", "new", 2.0, "n");
        assert_eq!(m.get("k"), Some("new".to_string()));
    }

    #[test]
    fn lwwmap_ignores_old_writes() {
        let mut m = WasmLWWMap::new();
        m.set("k", "new", 10.0, "n");
        m.set("k", "old", 5.0, "n");
        assert_eq!(m.get("k"), Some("new".to_string()));
    }

    #[test]
    fn lwwmap_remove() {
        let mut m = WasmLWWMap::new();
        m.set("k", "v", 1.0, "n");
        m.remove("k", 2.0, "n");
        assert!(m.get("k").is_none());
        assert!(!m.has_key("k"));
        assert!(m.is_empty());
    }

    #[test]
    fn lwwmap_remove_then_set() {
        let mut m = WasmLWWMap::new();
        m.set("k", "first", 1.0, "n");
        m.remove("k", 2.0, "n");
        m.set("k", "second", 3.0, "n");
        assert_eq!(m.get("k"), Some("second".to_string()));
    }

    #[test]
    fn lwwmap_old_remove_ignored() {
        let mut m = WasmLWWMap::new();
        m.set("k", "val", 10.0, "n");
        m.remove("k", 5.0, "n");
        assert_eq!(m.get("k"), Some("val".to_string()));
    }

    #[test]
    fn lwwmap_get_missing_key() {
        let m = WasmLWWMap::new();
        assert!(m.get("nonexistent").is_none());
        assert!(!m.has_key("nonexistent"));
    }

    #[test]
    fn lwwmap_multiple_keys() {
        let mut m = WasmLWWMap::new();
        m.set("a", "1", 1.0, "n");
        m.set("b", "2", 1.0, "n");
        m.set("c", "3", 1.0, "n");
        assert_eq!(m.len(), 3);
        assert!(!m.is_empty());
    }

    #[test]
    fn lwwmap_keys_json() {
        let mut m = WasmLWWMap::new();
        m.set("alpha", "1", 1.0, "n");
        m.set("beta", "2", 1.0, "n");
        m.remove("beta", 2.0, "n");
        let keys_json = m.keys();
        let keys: Vec<String> = serde_json::from_str(&keys_json).unwrap();
        assert_eq!(keys, vec!["alpha"]);
    }

    #[test]
    fn lwwmap_entries_json() {
        let mut m = WasmLWWMap::new();
        m.set("name", "nous", 1.0, "n");
        m.set("version", "1.0", 1.0, "n");
        let entries_json = m.entries();
        let entries: HashMap<String, String> = serde_json::from_str(&entries_json).unwrap();
        assert_eq!(entries.get("name").unwrap(), "nous");
        assert_eq!(entries.get("version").unwrap(), "1.0");
    }

    #[test]
    fn lwwmap_merge() {
        let mut a = WasmLWWMap::new();
        a.set("k1", "a-val", 1.0, "node-a");
        a.set("shared", "old", 1.0, "node-a");

        let mut b = WasmLWWMap::new();
        b.set("k2", "b-val", 1.0, "node-b");
        b.set("shared", "new", 2.0, "node-b");

        a.merge(&b);
        assert_eq!(a.get("k1"), Some("a-val".to_string()));
        assert_eq!(a.get("k2"), Some("b-val".to_string()));
        assert_eq!(a.get("shared"), Some("new".to_string()));
    }

    #[test]
    fn lwwmap_merge_commutative() {
        let mut a = WasmLWWMap::new();
        a.set("x", "1", 1.0, "a");
        let mut b = WasmLWWMap::new();
        b.set("y", "2", 1.0, "b");

        let mut ab = WasmLWWMap {
            inner: a.inner.clone(),
        };
        ab.merge(&b);
        let mut ba = WasmLWWMap {
            inner: b.inner.clone(),
        };
        ba.merge(&a);

        assert_eq!(ab.get("x"), ba.get("x"));
        assert_eq!(ab.get("y"), ba.get("y"));
    }

    #[test]
    fn lwwmap_merge_with_removal() {
        let mut a = WasmLWWMap::new();
        a.set("k", "v", 1.0, "node-a");

        let mut b = WasmLWWMap::new();
        b.set("k", "v", 1.0, "node-b");
        b.remove("k", 2.0, "node-b");

        a.merge(&b);
        assert!(a.get("k").is_none());
    }

    #[test]
    fn lwwmap_merge_tiebreak_by_node_id() {
        let mut a = WasmLWWMap::new();
        a.set("k", "a-wins", 1.0, "node-a");
        let mut b = WasmLWWMap::new();
        b.set("k", "b-wins", 1.0, "node-b");
        a.merge(&b);
        assert_eq!(a.get("k"), Some("b-wins".to_string()));
    }

    #[test]
    fn lwwmap_json_roundtrip() {
        let mut m = WasmLWWMap::new();
        m.set("name", "nous", 1.0, "n");
        m.set("deleted", "val", 1.0, "n");
        m.remove("deleted", 2.0, "n");
        let json = m.to_json();
        let restored = WasmLWWMap::from_json(&json).unwrap();
        assert_eq!(restored.get("name"), Some("nous".to_string()));
        assert!(restored.get("deleted").is_none());
        assert_eq!(restored.len(), 1);
    }

    #[test]
    fn lwwmap_json_rejects_invalid() {
        let result = std::panic::catch_unwind(|| WasmLWWMap::from_json("garbage"));
        assert!(result.is_err() || result.unwrap().is_err());
    }
}
