use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Embedding {
    pub vector: Vec<f32>,
    pub model: String,
    pub dimensions: usize,
}

impl Embedding {
    pub fn new(vector: Vec<f32>, model: impl Into<String>) -> Self {
        let dimensions = vector.len();
        Self {
            vector,
            model: model.into(),
            dimensions,
        }
    }

    pub fn cosine_similarity(&self, other: &Embedding) -> f32 {
        if self.dimensions != other.dimensions {
            return 0.0;
        }

        let dot: f32 = self
            .vector
            .iter()
            .zip(other.vector.iter())
            .map(|(a, b)| a * b)
            .sum();

        let mag_a: f32 = self.vector.iter().map(|x| x * x).sum::<f32>().sqrt();
        let mag_b: f32 = other.vector.iter().map(|x| x * x).sum::<f32>().sqrt();

        if mag_a == 0.0 || mag_b == 0.0 {
            return 0.0;
        }

        dot / (mag_a * mag_b)
    }

    pub fn euclidean_distance(&self, other: &Embedding) -> f32 {
        if self.dimensions != other.dimensions {
            return f32::MAX;
        }

        self.vector
            .iter()
            .zip(other.vector.iter())
            .map(|(a, b)| (a - b).powi(2))
            .sum::<f32>()
            .sqrt()
    }

    pub fn normalize(&mut self) {
        let magnitude: f32 = self.vector.iter().map(|x| x * x).sum::<f32>().sqrt();
        if magnitude > 0.0 {
            for v in &mut self.vector {
                *v /= magnitude;
            }
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmbeddingIndex {
    entries: Vec<IndexEntry>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct IndexEntry {
    id: String,
    embedding: Embedding,
    metadata: String,
}

impl EmbeddingIndex {
    pub fn new() -> Self {
        Self {
            entries: Vec::new(),
        }
    }

    pub fn insert(
        &mut self,
        id: impl Into<String>,
        embedding: Embedding,
        metadata: impl Into<String>,
    ) {
        self.entries.push(IndexEntry {
            id: id.into(),
            embedding,
            metadata: metadata.into(),
        });
    }

    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    pub fn search(&self, query: &Embedding, limit: usize) -> Vec<SearchResult> {
        let mut results: Vec<SearchResult> = self
            .entries
            .iter()
            .map(|entry| SearchResult {
                id: entry.id.clone(),
                score: entry.embedding.cosine_similarity(query),
                metadata: entry.metadata.clone(),
            })
            .collect();

        results.sort_by(|a, b| {
            b.score
                .partial_cmp(&a.score)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        results.truncate(limit);
        results
    }

    pub fn remove(&mut self, id: &str) -> bool {
        let before = self.entries.len();
        self.entries.retain(|e| e.id != id);
        self.entries.len() < before
    }
}

impl Default for EmbeddingIndex {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchResult {
    pub id: String,
    pub score: f32,
    pub metadata: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn vec_embedding(v: Vec<f32>) -> Embedding {
        Embedding::new(v, "test")
    }

    #[test]
    fn cosine_similarity_identical() {
        let a = vec_embedding(vec![1.0, 0.0, 0.0]);
        let b = vec_embedding(vec![1.0, 0.0, 0.0]);
        let sim = a.cosine_similarity(&b);
        assert!((sim - 1.0).abs() < 1e-6);
    }

    #[test]
    fn cosine_similarity_orthogonal() {
        let a = vec_embedding(vec![1.0, 0.0]);
        let b = vec_embedding(vec![0.0, 1.0]);
        let sim = a.cosine_similarity(&b);
        assert!(sim.abs() < 1e-6);
    }

    #[test]
    fn cosine_similarity_opposite() {
        let a = vec_embedding(vec![1.0, 0.0]);
        let b = vec_embedding(vec![-1.0, 0.0]);
        let sim = a.cosine_similarity(&b);
        assert!((sim - (-1.0)).abs() < 1e-6);
    }

    #[test]
    fn cosine_similarity_different_dimensions() {
        let a = vec_embedding(vec![1.0, 0.0]);
        let b = vec_embedding(vec![1.0, 0.0, 0.0]);
        assert_eq!(a.cosine_similarity(&b), 0.0);
    }

    #[test]
    fn euclidean_distance() {
        let a = vec_embedding(vec![0.0, 0.0]);
        let b = vec_embedding(vec![3.0, 4.0]);
        let dist = a.euclidean_distance(&b);
        assert!((dist - 5.0).abs() < 1e-6);
    }

    #[test]
    fn normalize() {
        let mut e = vec_embedding(vec![3.0, 4.0]);
        e.normalize();
        let mag: f32 = e.vector.iter().map(|x| x * x).sum::<f32>().sqrt();
        assert!((mag - 1.0).abs() < 1e-6);
    }

    #[test]
    fn index_search() {
        let mut index = EmbeddingIndex::new();
        index.insert("doc1", vec_embedding(vec![1.0, 0.0, 0.0]), "about cats");
        index.insert("doc2", vec_embedding(vec![0.0, 1.0, 0.0]), "about dogs");
        index.insert("doc3", vec_embedding(vec![0.9, 0.1, 0.0]), "about kittens");

        let query = vec_embedding(vec![1.0, 0.0, 0.0]);
        let results = index.search(&query, 2);

        assert_eq!(results.len(), 2);
        assert_eq!(results[0].id, "doc1");
        assert_eq!(results[1].id, "doc3");
    }

    #[test]
    fn index_remove() {
        let mut index = EmbeddingIndex::new();
        index.insert("doc1", vec_embedding(vec![1.0, 0.0]), "test");
        assert_eq!(index.len(), 1);

        assert!(index.remove("doc1"));
        assert!(index.is_empty());
        assert!(!index.remove("doc1"));
    }

    #[test]
    fn embedding_serializes() {
        let e = vec_embedding(vec![1.0, 2.0, 3.0]);
        let json = serde_json::to_string(&e).unwrap();
        let restored: Embedding = serde_json::from_str(&json).unwrap();
        assert_eq!(restored.dimensions, 3);
    }

    #[test]
    fn zero_vector_cosine() {
        let a = vec_embedding(vec![0.0, 0.0]);
        let b = vec_embedding(vec![1.0, 0.0]);
        assert_eq!(a.cosine_similarity(&b), 0.0);
    }
}
