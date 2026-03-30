//! HNSW (Hierarchical Navigable Small World) approximate nearest neighbor index.
//!
//! Provides O(log n) approximate nearest neighbor search for embedding vectors.
//! Core algorithm: multi-layer graph where higher layers have fewer nodes for
//! coarse navigation, lower layers have more nodes for fine-grained search.
//!
//! Reference: Malkov & Yashunin, "Efficient and robust approximate nearest
//! neighbor search using Hierarchical Navigable Small World graphs" (2018).

use std::collections::{BinaryHeap, HashMap, HashSet};

use rand::Rng;
use serde::{Deserialize, Serialize};

use crate::embedding::Embedding;

/// Configuration for the HNSW index.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HnswConfig {
    /// Max connections per node at layer 0.
    pub m: usize,
    /// Max connections per node at layers > 0.
    pub m_max: usize,
    /// Size of dynamic candidate list during construction.
    pub ef_construction: usize,
    /// Size of dynamic candidate list during search.
    pub ef_search: usize,
    /// Level multiplier (1/ln(M)).
    pub ml: f64,
}

impl Default for HnswConfig {
    fn default() -> Self {
        let m = 16;
        Self {
            m,
            m_max: m * 2,
            ef_construction: 200,
            ef_search: 50,
            ml: 1.0 / (m as f64).ln(),
        }
    }
}

impl HnswConfig {
    pub fn with_m(m: usize) -> Self {
        Self {
            m,
            m_max: m * 2,
            ml: 1.0 / (m as f64).ln(),
            ..Default::default()
        }
    }
}

/// A node in the HNSW graph.
#[derive(Debug, Clone, Serialize, Deserialize)]
struct HnswNode {
    id: String,
    vector: Vec<f32>,
    metadata: String,
    /// Neighbors per layer: layer -> [neighbor_indices].
    neighbors: Vec<Vec<usize>>,
    level: usize,
}

/// HNSW index for approximate nearest neighbor search.
#[derive(Debug, Serialize, Deserialize)]
pub struct HnswIndex {
    config: HnswConfig,
    nodes: Vec<HnswNode>,
    id_to_index: HashMap<String, usize>,
    entry_point: Option<usize>,
    max_level: usize,
    dimensions: usize,
}

/// Result of an HNSW search.
#[derive(Debug, Clone)]
pub struct HnswResult {
    pub id: String,
    pub score: f32,
    pub metadata: String,
}

// Min-heap entry for priority queue (sorted by distance, ascending).
#[derive(Debug, Clone)]
struct Candidate {
    distance: f32,
    index: usize,
}

impl PartialEq for Candidate {
    fn eq(&self, other: &Self) -> bool {
        self.distance == other.distance
    }
}

impl Eq for Candidate {}

impl PartialOrd for Candidate {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Candidate {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        // Reverse order for max-heap (BinaryHeap) — we want min-distance first
        // when popping, but BinaryHeap is max-heap, so we reverse for "nearest" semantics.
        other
            .distance
            .partial_cmp(&self.distance)
            .unwrap_or(std::cmp::Ordering::Equal)
    }
}

// Max-heap entry (largest distance first — for eviction).
#[derive(Debug, Clone)]
struct FarCandidate {
    distance: f32,
    index: usize,
}

impl PartialEq for FarCandidate {
    fn eq(&self, other: &Self) -> bool {
        self.distance == other.distance
    }
}

impl Eq for FarCandidate {}

impl PartialOrd for FarCandidate {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for FarCandidate {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.distance
            .partial_cmp(&other.distance)
            .unwrap_or(std::cmp::Ordering::Equal)
    }
}

impl HnswIndex {
    /// Create a new empty HNSW index.
    pub fn new(dimensions: usize, config: HnswConfig) -> Self {
        Self {
            config,
            nodes: Vec::new(),
            id_to_index: HashMap::new(),
            entry_point: None,
            max_level: 0,
            dimensions,
        }
    }

    /// Create with default configuration.
    pub fn with_defaults(dimensions: usize) -> Self {
        Self::new(dimensions, HnswConfig::default())
    }

    /// Insert a vector into the index.
    pub fn insert(
        &mut self,
        id: impl Into<String>,
        embedding: &Embedding,
        metadata: impl Into<String>,
    ) {
        let id = id.into();
        if embedding.dimensions != self.dimensions {
            return;
        }

        // Remove existing entry with same id
        if self.id_to_index.contains_key(&id) {
            self.remove(&id);
        }

        let level = self.random_level();
        let node_index = self.nodes.len();

        let mut node = HnswNode {
            id: id.clone(),
            vector: embedding.vector.clone(),
            metadata: metadata.into(),
            neighbors: vec![Vec::new(); level + 1],
            level,
        };

        if self.id_to_index.is_empty() {
            // First (or only remaining) node
            self.nodes.push(node);
            self.id_to_index.insert(id, node_index);
            self.entry_point = Some(node_index);
            self.max_level = level;
            return;
        }

        let mut ep = match self.entry_point {
            Some(ep) if !self.nodes[ep].vector.is_empty() => ep,
            _ => {
                // Entry point is invalid, find any valid node
                match self.id_to_index.values().next().copied() {
                    Some(idx) => idx,
                    None => {
                        self.nodes.push(node);
                        self.id_to_index.insert(id, node_index);
                        self.entry_point = Some(node_index);
                        self.max_level = level;
                        return;
                    }
                }
            }
        };

        // Phase 1: Greedily traverse from top to insertion level
        let query = &embedding.vector;
        for lc in (level + 1..=self.max_level).rev() {
            ep = self.greedy_closest(query, ep, lc);
        }

        // Phase 2: Insert at each layer from min(level, max_level) down to 0
        let top = level.min(self.max_level);
        for lc in (0..=top).rev() {
            let neighbors = self.search_layer(query, ep, self.config.ef_construction, lc);

            // Select M best neighbors
            let m = if lc == 0 {
                self.config.m_max
            } else {
                self.config.m
            };
            let selected: Vec<usize> = neighbors.iter().take(m).map(|c| c.index).collect();

            node.neighbors[lc] = selected.clone();

            // Update ep for next layer
            if let Some(first) = neighbors.first() {
                ep = first.index;
            }
        }

        self.nodes.push(node);
        self.id_to_index.insert(id, node_index);

        // Add reverse connections
        let top = level.min(self.max_level);
        for lc in 0..=top {
            let m = if lc == 0 {
                self.config.m_max
            } else {
                self.config.m
            };
            let neighbors_at_lc: Vec<usize> = self.nodes[node_index].neighbors[lc].clone();
            for &neighbor_idx in &neighbors_at_lc {
                if neighbor_idx < self.nodes.len() && lc < self.nodes[neighbor_idx].neighbors.len()
                {
                    let already = self.nodes[neighbor_idx].neighbors[lc].contains(&node_index);
                    if !already {
                        self.nodes[neighbor_idx].neighbors[lc].push(node_index);
                        // Prune if too many connections
                        if self.nodes[neighbor_idx].neighbors[lc].len() > m {
                            self.prune_connections(neighbor_idx, lc, m);
                        }
                    }
                }
            }
        }

        // Update entry point if new node has higher level
        if level > self.max_level {
            self.max_level = level;
            self.entry_point = Some(node_index);
        }
    }

    /// Search for the k nearest neighbors.
    pub fn search(&self, query: &Embedding, k: usize) -> Vec<HnswResult> {
        if self.nodes.is_empty() || query.dimensions != self.dimensions {
            return Vec::new();
        }

        let mut ep = self.entry_point.unwrap();
        let q = &query.vector;

        // Greedy traversal from top layer down to layer 1
        for lc in (1..=self.max_level).rev() {
            ep = self.greedy_closest(q, ep, lc);
        }

        // Search layer 0 with ef_search candidates
        let ef = self.config.ef_search.max(k);
        let candidates = self.search_layer(q, ep, ef, 0);

        candidates
            .into_iter()
            .take(k)
            .map(|c| {
                let node = &self.nodes[c.index];
                HnswResult {
                    id: node.id.clone(),
                    score: 1.0 - c.distance, // Convert distance to similarity
                    metadata: node.metadata.clone(),
                }
            })
            .collect()
    }

    /// Remove a vector by id.
    pub fn remove(&mut self, id: &str) -> bool {
        let Some(&index) = self.id_to_index.get(id) else {
            return false;
        };

        // Remove from all neighbor lists
        for node in &mut self.nodes {
            for layer in &mut node.neighbors {
                layer.retain(|&n| n != index);
            }
        }

        // Mark as deleted (we don't actually remove to preserve indices)
        self.nodes[index].vector.clear();
        self.nodes[index].neighbors.clear();
        self.id_to_index.remove(id);

        // Update entry point if needed
        if self.entry_point == Some(index) {
            self.entry_point = self.id_to_index.values().copied().next();
        }

        true
    }

    /// Number of vectors in the index.
    pub fn len(&self) -> usize {
        self.id_to_index.len()
    }

    pub fn is_empty(&self) -> bool {
        self.id_to_index.is_empty()
    }

    /// Current max level.
    pub fn max_level(&self) -> usize {
        self.max_level
    }

    /// Index configuration.
    pub fn config(&self) -> &HnswConfig {
        &self.config
    }

    // ── Internal ───────────────────────────────────────────────

    fn random_level(&self) -> usize {
        let mut rng = rand::thread_rng();
        let uniform: f64 = rng.r#gen();
        (-uniform.ln() * self.config.ml).floor() as usize
    }

    fn distance(a: &[f32], b: &[f32]) -> f32 {
        // Cosine distance = 1 - cosine_similarity
        let dot: f32 = a.iter().zip(b.iter()).map(|(x, y)| x * y).sum();
        let mag_a: f32 = a.iter().map(|x| x * x).sum::<f32>().sqrt();
        let mag_b: f32 = b.iter().map(|x| x * x).sum::<f32>().sqrt();

        if mag_a == 0.0 || mag_b == 0.0 {
            return 1.0;
        }

        1.0 - (dot / (mag_a * mag_b))
    }

    fn greedy_closest(&self, query: &[f32], mut ep: usize, layer: usize) -> usize {
        let mut ep_dist = Self::distance(query, &self.nodes[ep].vector);

        loop {
            let mut changed = false;
            let neighbors = if layer < self.nodes[ep].neighbors.len() {
                &self.nodes[ep].neighbors[layer]
            } else {
                break;
            };

            for &neighbor in neighbors {
                if neighbor >= self.nodes.len() || self.nodes[neighbor].vector.is_empty() {
                    continue;
                }
                let dist = Self::distance(query, &self.nodes[neighbor].vector);
                if dist < ep_dist {
                    ep = neighbor;
                    ep_dist = dist;
                    changed = true;
                }
            }

            if !changed {
                break;
            }
        }

        ep
    }

    fn search_layer(&self, query: &[f32], ep: usize, ef: usize, layer: usize) -> Vec<Candidate> {
        let mut visited = HashSet::new();
        visited.insert(ep);

        let ep_dist = Self::distance(query, &self.nodes[ep].vector);

        // candidates = min-heap (closest first)
        let mut candidates = BinaryHeap::new();
        candidates.push(Candidate {
            distance: ep_dist,
            index: ep,
        });

        // result = max-heap (farthest first, for eviction)
        let mut result = BinaryHeap::new();
        result.push(FarCandidate {
            distance: ep_dist,
            index: ep,
        });

        while let Some(closest) = candidates.pop() {
            let farthest_dist = result.peek().map(|r| r.distance).unwrap_or(f32::MAX);

            if closest.distance > farthest_dist {
                break; // All remaining candidates are farther than our worst result
            }

            let neighbors = if layer < self.nodes[closest.index].neighbors.len() {
                &self.nodes[closest.index].neighbors[layer]
            } else {
                continue;
            };

            for &neighbor in neighbors {
                if !visited.insert(neighbor) {
                    continue;
                }
                if neighbor >= self.nodes.len() || self.nodes[neighbor].vector.is_empty() {
                    continue;
                }

                let dist = Self::distance(query, &self.nodes[neighbor].vector);
                let farthest_dist = result.peek().map(|r| r.distance).unwrap_or(f32::MAX);

                if dist < farthest_dist || result.len() < ef {
                    candidates.push(Candidate {
                        distance: dist,
                        index: neighbor,
                    });
                    result.push(FarCandidate {
                        distance: dist,
                        index: neighbor,
                    });

                    if result.len() > ef {
                        result.pop(); // Remove farthest
                    }
                }
            }
        }

        // Convert to sorted vec (closest first)
        let mut results: Vec<Candidate> = result
            .into_iter()
            .map(|fc| Candidate {
                distance: fc.distance,
                index: fc.index,
            })
            .collect();
        results.sort_by(|a, b| {
            a.distance
                .partial_cmp(&b.distance)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        results
    }

    fn prune_connections(&mut self, node_idx: usize, layer: usize, max_connections: usize) {
        let node_vec = self.nodes[node_idx].vector.clone();
        let mut scored: Vec<(f32, usize)> = self.nodes[node_idx].neighbors[layer]
            .iter()
            .filter(|&&n| n < self.nodes.len() && !self.nodes[n].vector.is_empty())
            .map(|&n| (Self::distance(&node_vec, &self.nodes[n].vector), n))
            .collect();

        scored.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap_or(std::cmp::Ordering::Equal));
        scored.truncate(max_connections);

        self.nodes[node_idx].neighbors[layer] = scored.into_iter().map(|(_, idx)| idx).collect();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::embedding::Embedding;

    fn emb(v: Vec<f32>) -> Embedding {
        Embedding::new(v, "test")
    }

    // ── Config ─────────────────────────────────────────────────

    #[test]
    fn default_config() {
        let config = HnswConfig::default();
        assert_eq!(config.m, 16);
        assert_eq!(config.m_max, 32);
        assert!(config.ml > 0.0);
    }

    #[test]
    fn custom_config() {
        let config = HnswConfig::with_m(8);
        assert_eq!(config.m, 8);
        assert_eq!(config.m_max, 16);
    }

    // ── Basic operations ───────────────────────────────────────

    #[test]
    fn insert_and_search() {
        let mut index = HnswIndex::with_defaults(3);
        index.insert("a", &emb(vec![1.0, 0.0, 0.0]), "doc a");
        index.insert("b", &emb(vec![0.0, 1.0, 0.0]), "doc b");
        index.insert("c", &emb(vec![0.9, 0.1, 0.0]), "doc c");

        let results = index.search(&emb(vec![1.0, 0.0, 0.0]), 2);
        assert_eq!(results.len(), 2);
        assert_eq!(results[0].id, "a");
        assert!(results[0].score > results[1].score);
    }

    #[test]
    fn empty_index_returns_empty() {
        let index = HnswIndex::with_defaults(3);
        let results = index.search(&emb(vec![1.0, 0.0, 0.0]), 5);
        assert!(results.is_empty());
    }

    #[test]
    fn single_element() {
        let mut index = HnswIndex::with_defaults(3);
        index.insert("only", &emb(vec![1.0, 2.0, 3.0]), "single");

        let results = index.search(&emb(vec![1.0, 2.0, 3.0]), 1);
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].id, "only");
        assert!((results[0].score - 1.0).abs() < 1e-5);
    }

    #[test]
    fn insert_same_id_replaces() {
        let mut index = HnswIndex::with_defaults(3);
        index.insert("x", &emb(vec![1.0, 0.0, 0.0]), "first");
        index.insert("x", &emb(vec![0.0, 1.0, 0.0]), "second");

        assert_eq!(index.len(), 1);
        let results = index.search(&emb(vec![0.0, 1.0, 0.0]), 1);
        assert_eq!(results[0].id, "x");
        assert_eq!(results[0].metadata, "second");
    }

    #[test]
    fn remove_element() {
        let mut index = HnswIndex::with_defaults(3);
        index.insert("a", &emb(vec![1.0, 0.0, 0.0]), "");
        index.insert("b", &emb(vec![0.0, 1.0, 0.0]), "");

        assert!(index.remove("a"));
        assert_eq!(index.len(), 1);
        assert!(!index.remove("a")); // Already removed
    }

    #[test]
    fn wrong_dimensions_ignored() {
        let mut index = HnswIndex::with_defaults(3);
        index.insert("a", &emb(vec![1.0, 0.0]), "wrong dims");
        assert_eq!(index.len(), 0);
    }

    // ── Search quality ─────────────────────────────────────────

    #[test]
    fn search_finds_nearest_cluster() {
        let mut index = HnswIndex::new(
            3,
            HnswConfig {
                ef_search: 100,
                ef_construction: 100,
                ..HnswConfig::with_m(4)
            },
        );

        // Cluster A: near [1, 0, 0]
        index.insert("a1", &emb(vec![1.0, 0.0, 0.0]), "cluster-a");
        index.insert("a2", &emb(vec![0.95, 0.05, 0.0]), "cluster-a");
        index.insert("a3", &emb(vec![0.9, 0.1, 0.0]), "cluster-a");

        // Cluster B: near [0, 1, 0]
        index.insert("b1", &emb(vec![0.0, 1.0, 0.0]), "cluster-b");
        index.insert("b2", &emb(vec![0.05, 0.95, 0.0]), "cluster-b");
        index.insert("b3", &emb(vec![0.1, 0.9, 0.0]), "cluster-b");

        let results = index.search(&emb(vec![0.98, 0.02, 0.0]), 3);
        assert_eq!(results.len(), 3);
        // All results should be from cluster A
        for r in &results {
            assert_eq!(r.metadata, "cluster-a");
        }
    }

    #[test]
    fn search_k_larger_than_index() {
        let mut index = HnswIndex::with_defaults(2);
        index.insert("a", &emb(vec![1.0, 0.0]), "");
        index.insert("b", &emb(vec![0.0, 1.0]), "");

        let results = index.search(&emb(vec![1.0, 0.0]), 10);
        assert_eq!(results.len(), 2); // Only 2 elements
    }

    #[test]
    fn search_scores_decrease() {
        let mut index = HnswIndex::new(
            3,
            HnswConfig {
                ef_search: 50,
                ..HnswConfig::with_m(4)
            },
        );

        for i in 0..20 {
            let angle = (i as f32) * 0.1;
            let v = vec![angle.cos(), angle.sin(), 0.0];
            index.insert(&format!("v{i}"), &emb(v), "");
        }

        let results = index.search(&emb(vec![1.0, 0.0, 0.0]), 5);
        for window in results.windows(2) {
            assert!(window[0].score >= window[1].score);
        }
    }

    // ── Cosine distance ────────────────────────────────────────

    #[test]
    fn distance_identical_is_zero() {
        let d = HnswIndex::distance(&[1.0, 0.0, 0.0], &[1.0, 0.0, 0.0]);
        assert!(d.abs() < 1e-6);
    }

    #[test]
    fn distance_orthogonal_is_one() {
        let d = HnswIndex::distance(&[1.0, 0.0], &[0.0, 1.0]);
        assert!((d - 1.0).abs() < 1e-6);
    }

    #[test]
    fn distance_opposite_is_two() {
        let d = HnswIndex::distance(&[1.0, 0.0], &[-1.0, 0.0]);
        assert!((d - 2.0).abs() < 1e-6);
    }

    // ── Scaling ────────────────────────────────────────────────

    #[test]
    fn handles_moderate_dataset() {
        let mut index = HnswIndex::new(
            8,
            HnswConfig {
                ef_search: 50,
                ef_construction: 100,
                ..HnswConfig::with_m(8)
            },
        );

        let mut rng = rand::thread_rng();
        for i in 0..200 {
            let v: Vec<f32> = (0..8).map(|_| rng.r#gen::<f32>()).collect();
            index.insert(&format!("doc{i}"), &emb(v), &format!("meta{i}"));
        }

        assert_eq!(index.len(), 200);

        // Search should return exactly k results
        let query: Vec<f32> = (0..8).map(|_| rng.r#gen::<f32>()).collect();
        let results = index.search(&emb(query), 10);
        assert_eq!(results.len(), 10);
    }

    #[test]
    fn recall_at_10_is_reasonable() {
        // Verify HNSW recall quality against brute-force
        let dims = 8;
        let n = 100;
        let k = 5;

        let config = HnswConfig {
            ef_search: 100,
            ef_construction: 200,
            ..HnswConfig::with_m(8)
        };
        let mut index = HnswIndex::new(dims, config);
        let mut brute = crate::embedding::EmbeddingIndex::new();

        let mut rng = rand::thread_rng();
        let vectors: Vec<Vec<f32>> = (0..n)
            .map(|_| (0..dims).map(|_| rng.r#gen::<f32>()).collect())
            .collect();

        for (i, v) in vectors.iter().enumerate() {
            let e = emb(v.clone());
            index.insert(&format!("d{i}"), &e, "");
            brute.insert(format!("d{i}"), e, "");
        }

        // Run 10 random queries and check recall
        let mut total_recall = 0.0;
        for _ in 0..10 {
            let q: Vec<f32> = (0..dims).map(|_| rng.r#gen::<f32>()).collect();
            let qe = emb(q);

            let hnsw_ids: HashSet<String> =
                index.search(&qe, k).into_iter().map(|r| r.id).collect();
            let brute_ids: HashSet<String> =
                brute.search(&qe, k).into_iter().map(|r| r.id).collect();

            let intersection = hnsw_ids.intersection(&brute_ids).count();
            total_recall += intersection as f64 / k as f64;
        }

        let avg_recall = total_recall / 10.0;
        assert!(
            avg_recall >= 0.6,
            "recall@{k} should be >= 60%, got {:.1}%",
            avg_recall * 100.0
        );
    }

    // ── Serialization ──────────────────────────────────────────

    #[test]
    fn index_serializes() {
        let mut index = HnswIndex::with_defaults(3);
        index.insert("a", &emb(vec![1.0, 0.0, 0.0]), "doc a");
        index.insert("b", &emb(vec![0.0, 1.0, 0.0]), "doc b");

        let json = serde_json::to_string(&index).unwrap();
        let restored: HnswIndex = serde_json::from_str(&json).unwrap();

        assert_eq!(restored.len(), 2);
        let results = restored.search(&emb(vec![1.0, 0.0, 0.0]), 1);
        assert_eq!(results[0].id, "a");
    }

    #[test]
    fn config_serializes() {
        let config = HnswConfig::default();
        let json = serde_json::to_string(&config).unwrap();
        let restored: HnswConfig = serde_json::from_str(&json).unwrap();
        assert_eq!(restored.m, 16);
    }
}
