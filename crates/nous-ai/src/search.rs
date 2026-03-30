//! BM25 keyword search with inverted index.
//!
//! Provides term-frequency / inverse-document-frequency scoring using the
//! Okapi BM25 algorithm. Complements the embedding-based semantic search in
//! [`crate::embedding`] — use both together for hybrid retrieval.

use std::collections::{HashMap, HashSet};

use serde::{Deserialize, Serialize};

/// Parameters for BM25 scoring.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Bm25Config {
    /// Term frequency saturation parameter (typically 1.2–2.0).
    pub k1: f32,
    /// Document length normalization (0 = no normalization, 1 = full).
    pub b: f32,
}

impl Default for Bm25Config {
    fn default() -> Self {
        Self { k1: 1.5, b: 0.75 }
    }
}

/// A tokenized document stored in the index.
#[derive(Debug, Clone, Serialize, Deserialize)]
struct IndexedDocument {
    id: String,
    token_count: usize,
    term_freqs: HashMap<String, u32>,
    metadata: String,
}

/// Inverted index mapping terms to document IDs.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
struct PostingList {
    /// Document IDs that contain this term.
    doc_ids: Vec<String>,
}

/// A result from keyword search.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KeywordResult {
    pub id: String,
    pub score: f32,
    pub metadata: String,
    /// Which query terms matched and how many times in this document.
    pub matched_terms: Vec<(String, u32)>,
}

/// BM25 keyword search engine with an inverted index.
///
/// Supports incremental indexing, removal, and multi-term queries scored
/// with the Okapi BM25 ranking function.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchEngine {
    config: Bm25Config,
    documents: HashMap<String, IndexedDocument>,
    postings: HashMap<String, PostingList>,
    total_tokens: u64,
    stopwords: HashSet<String>,
}

impl SearchEngine {
    pub fn new(config: Bm25Config) -> Self {
        Self {
            config,
            documents: HashMap::new(),
            postings: HashMap::new(),
            total_tokens: 0,
            stopwords: default_stopwords(),
        }
    }

    pub fn document_count(&self) -> usize {
        self.documents.len()
    }

    pub fn term_count(&self) -> usize {
        self.postings.len()
    }

    /// Average document length in tokens.
    fn avg_doc_len(&self) -> f32 {
        if self.documents.is_empty() {
            return 0.0;
        }
        self.total_tokens as f32 / self.documents.len() as f32
    }

    /// Index a document. Tokenizes the text and updates the inverted index.
    pub fn index(&mut self, id: impl Into<String>, text: &str, metadata: impl Into<String>) {
        let id = id.into();

        // Remove previous version if re-indexing.
        self.remove(&id);

        let tokens = self.tokenize(text);
        let token_count = tokens.len();

        let mut term_freqs: HashMap<String, u32> = HashMap::new();
        for token in &tokens {
            *term_freqs.entry(token.clone()).or_insert(0) += 1;
        }

        // Update postings.
        for term in term_freqs.keys() {
            self.postings
                .entry(term.clone())
                .or_default()
                .doc_ids
                .push(id.clone());
        }

        self.total_tokens += token_count as u64;

        self.documents.insert(
            id.clone(),
            IndexedDocument {
                id,
                token_count,
                term_freqs,
                metadata: metadata.into(),
            },
        );
    }

    /// Remove a document from the index.
    pub fn remove(&mut self, id: &str) -> bool {
        let Some(doc) = self.documents.remove(id) else {
            return false;
        };

        self.total_tokens = self.total_tokens.saturating_sub(doc.token_count as u64);

        for term in doc.term_freqs.keys() {
            if let Some(posting) = self.postings.get_mut(term) {
                posting.doc_ids.retain(|d| d != id);
                if posting.doc_ids.is_empty() {
                    self.postings.remove(term);
                }
            }
        }

        true
    }

    /// Search using BM25 scoring. Returns results sorted by descending score.
    pub fn search(&self, query: &str, limit: usize) -> Vec<KeywordResult> {
        let query_terms = self.tokenize(query);
        if query_terms.is_empty() {
            return Vec::new();
        }

        let n = self.documents.len() as f32;
        let avgdl = self.avg_doc_len();

        // Collect candidate documents (any doc containing at least one query term).
        let mut candidates: HashSet<&str> = HashSet::new();
        for term in &query_terms {
            if let Some(posting) = self.postings.get(term) {
                for doc_id in &posting.doc_ids {
                    candidates.insert(doc_id);
                }
            }
        }

        let mut results: Vec<KeywordResult> = candidates
            .into_iter()
            .filter_map(|doc_id| {
                let doc = self.documents.get(doc_id)?;
                let (score, matched) = self.score_document(doc, &query_terms, n, avgdl);
                if score <= 0.0 {
                    return None;
                }
                Some(KeywordResult {
                    id: doc.id.clone(),
                    score,
                    metadata: doc.metadata.clone(),
                    matched_terms: matched,
                })
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

    /// Score a single document against query terms using BM25.
    fn score_document(
        &self,
        doc: &IndexedDocument,
        query_terms: &[String],
        n: f32,
        avgdl: f32,
    ) -> (f32, Vec<(String, u32)>) {
        let mut total_score = 0.0f32;
        let mut matched = Vec::new();
        let dl = doc.token_count as f32;

        for term in query_terms {
            let tf = doc.term_freqs.get(term).copied().unwrap_or(0);
            if tf == 0 {
                continue;
            }

            // Number of documents containing this term.
            let df = self
                .postings
                .get(term)
                .map(|p| p.doc_ids.len() as f32)
                .unwrap_or(0.0);

            // IDF component: log((N - df + 0.5) / (df + 0.5) + 1)
            let idf = ((n - df + 0.5) / (df + 0.5) + 1.0).ln();

            // TF component with saturation and length normalization.
            let tf_f = tf as f32;
            let tf_norm = (tf_f * (self.config.k1 + 1.0))
                / (tf_f + self.config.k1 * (1.0 - self.config.b + self.config.b * dl / avgdl));

            total_score += idf * tf_norm;
            matched.push((term.clone(), tf));
        }

        (total_score, matched)
    }

    /// Tokenize text: lowercase, split on non-alphanumeric, filter stopwords
    /// and short tokens.
    fn tokenize(&self, text: &str) -> Vec<String> {
        text.to_lowercase()
            .split(|c: char| !c.is_alphanumeric())
            .filter(|t| t.len() >= 2)
            .filter(|t| !self.stopwords.contains(*t))
            .map(String::from)
            .collect()
    }

    /// Get the number of documents containing a specific term.
    pub fn document_frequency(&self, term: &str) -> usize {
        self.postings
            .get(term)
            .map(|p| p.doc_ids.len())
            .unwrap_or(0)
    }

    /// Get all unique terms in the index.
    pub fn vocabulary_size(&self) -> usize {
        self.postings.len()
    }
}

impl Default for SearchEngine {
    fn default() -> Self {
        Self::new(Bm25Config::default())
    }
}

fn default_stopwords() -> HashSet<String> {
    [
        "a", "an", "the", "is", "it", "in", "on", "of", "to", "and", "or", "for", "with", "at",
        "by", "as", "be", "was", "are", "has", "had", "do", "did", "but", "not", "this", "that",
        "from", "they", "we", "he", "she", "its", "my", "his", "her", "our", "your", "will",
        "would", "can", "could", "if", "so", "no", "up", "out", "all", "been", "have", "were",
        "what", "when", "who", "which", "their", "than", "them", "then", "into", "each", "also",
    ]
    .iter()
    .map(|s| s.to_string())
    .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_engine() -> SearchEngine {
        SearchEngine::default()
    }

    #[test]
    fn empty_engine() {
        let engine = make_engine();
        assert_eq!(engine.document_count(), 0);
        assert_eq!(engine.term_count(), 0);
    }

    #[test]
    fn index_single_document() {
        let mut engine = make_engine();
        engine.index("doc1", "hello world", "metadata");
        assert_eq!(engine.document_count(), 1);
        assert!(engine.term_count() > 0);
    }

    #[test]
    fn index_multiple_documents() {
        let mut engine = make_engine();
        engine.index("d1", "rust programming language", "");
        engine.index("d2", "python programming scripting", "");
        engine.index("d3", "javascript web development", "");
        assert_eq!(engine.document_count(), 3);
    }

    #[test]
    fn search_finds_relevant_document() {
        let mut engine = make_engine();
        engine.index("d1", "governance proposal voting quadratic", "gov");
        engine.index("d2", "messaging encryption protocol", "msg");
        engine.index("d3", "marketplace listing escrow", "mkt");

        let results = engine.search("governance voting", 10);
        assert!(!results.is_empty());
        assert_eq!(results[0].id, "d1");
    }

    #[test]
    fn search_ranks_by_relevance() {
        let mut engine = make_engine();
        engine.index("d1", "rust rust rust rust rust systems programming", "");
        engine.index("d2", "rust programming general overview text content", "");
        engine.index("d3", "python scripting language", "");

        let results = engine.search("rust", 10);
        assert!(results.len() >= 2);
        // d1 has far more "rust" occurrences — BM25 should rank it higher.
        assert_eq!(results[0].id, "d1");
    }

    #[test]
    fn search_empty_query() {
        let mut engine = make_engine();
        engine.index("d1", "some document text", "");
        let results = engine.search("", 10);
        assert!(results.is_empty());
    }

    #[test]
    fn search_stopwords_only() {
        let mut engine = make_engine();
        engine.index("d1", "some document text", "");
        let results = engine.search("the is and", 10);
        assert!(results.is_empty());
    }

    #[test]
    fn search_no_match() {
        let mut engine = make_engine();
        engine.index("d1", "governance proposal", "");
        let results = engine.search("cryptocurrency blockchain", 10);
        assert!(results.is_empty());
    }

    #[test]
    fn search_respects_limit() {
        let mut engine = make_engine();
        for i in 0..20 {
            engine.index(format!("d{i}"), &format!("common term document {i}"), "");
        }
        let results = engine.search("common term", 5);
        assert_eq!(results.len(), 5);
    }

    #[test]
    fn remove_document() {
        let mut engine = make_engine();
        engine.index("d1", "hello world", "");
        assert_eq!(engine.document_count(), 1);

        assert!(engine.remove("d1"));
        assert_eq!(engine.document_count(), 0);

        let results = engine.search("hello", 10);
        assert!(results.is_empty());
    }

    #[test]
    fn remove_nonexistent() {
        let mut engine = make_engine();
        assert!(!engine.remove("ghost"));
    }

    #[test]
    fn reindex_document() {
        let mut engine = make_engine();
        engine.index("d1", "old content about cats", "v1");
        engine.index("d1", "new content about dogs", "v2");

        assert_eq!(engine.document_count(), 1);

        let results = engine.search("dogs", 10);
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].metadata, "v2");

        let results = engine.search("cats", 10);
        assert!(results.is_empty());
    }

    #[test]
    fn document_frequency() {
        let mut engine = make_engine();
        engine.index("d1", "rust programming", "");
        engine.index("d2", "rust systems", "");
        engine.index("d3", "python scripting", "");

        assert_eq!(engine.document_frequency("rust"), 2);
        assert_eq!(engine.document_frequency("python"), 1);
        assert_eq!(engine.document_frequency("unknown"), 0);
    }

    #[test]
    fn matched_terms_reported() {
        let mut engine = make_engine();
        engine.index("d1", "governance voting proposal system", "");

        let results = engine.search("governance voting", 10);
        assert_eq!(results.len(), 1);
        assert!(!results[0].matched_terms.is_empty());

        let terms: Vec<&str> = results[0]
            .matched_terms
            .iter()
            .map(|(t, _)| t.as_str())
            .collect();
        assert!(terms.contains(&"governance"));
        assert!(terms.contains(&"voting"));
    }

    #[test]
    fn bm25_idf_boosts_rare_terms() {
        let mut engine = make_engine();
        // "common" appears in all docs, "rare" in only one.
        engine.index("d1", "common common common rare", "");
        engine.index("d2", "common document text", "");
        engine.index("d3", "common another document", "");

        let results = engine.search("rare", 10);
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].id, "d1");
        assert!(results[0].score > 0.0);
    }

    #[test]
    fn custom_bm25_config() {
        let config = Bm25Config { k1: 2.0, b: 0.5 };
        let mut engine = SearchEngine::new(config);
        engine.index("d1", "test document content", "");
        let results = engine.search("test", 10);
        assert_eq!(results.len(), 1);
    }

    #[test]
    fn tokenization_handles_punctuation() {
        let mut engine = make_engine();
        engine.index("d1", "Hello, World! This is a test-case.", "");

        let results = engine.search("hello", 10);
        assert_eq!(results.len(), 1);
    }

    #[test]
    fn tokenization_case_insensitive() {
        let mut engine = make_engine();
        engine.index("d1", "Governance VOTING Proposal", "");

        let results = engine.search("governance", 10);
        assert_eq!(results.len(), 1);
    }

    #[test]
    fn vocabulary_size() {
        let mut engine = make_engine();
        engine.index("d1", "alpha beta gamma", "");
        engine.index("d2", "beta gamma delta", "");
        assert_eq!(engine.vocabulary_size(), 4);
    }

    #[test]
    fn search_serializes() {
        let mut engine = make_engine();
        engine.index("d1", "serialization test document", "meta");

        let json = serde_json::to_string(&engine).unwrap();
        let restored: SearchEngine = serde_json::from_str(&json).unwrap();
        assert_eq!(restored.document_count(), 1);

        let results = restored.search("serialization", 10);
        assert_eq!(results.len(), 1);
    }

    #[test]
    fn large_corpus_search() {
        let mut engine = make_engine();
        for i in 0..500 {
            engine.index(
                format!("doc-{i}"),
                &format!(
                    "document number {i} about topic {} with content {}",
                    i % 50,
                    i % 10
                ),
                "",
            );
        }

        assert_eq!(engine.document_count(), 500);

        let results = engine.search("topic", 10);
        assert_eq!(results.len(), 10);
        // All results should have positive scores.
        for r in &results {
            assert!(r.score > 0.0);
        }
    }

    #[test]
    fn remove_updates_postings() {
        let mut engine = make_engine();
        engine.index("d1", "unique term alpha", "");
        engine.index("d2", "another document beta", "");

        assert_eq!(engine.document_frequency("unique"), 1);
        engine.remove("d1");
        assert_eq!(engine.document_frequency("unique"), 0);
        assert_eq!(engine.document_frequency("another"), 1);
    }

    #[test]
    fn avg_doc_len_updates() {
        let mut engine = make_engine();
        engine.index("d1", "short", "");
        engine.index("d2", "longer document text content", "");
        let avg = engine.avg_doc_len();
        assert!(avg > 1.0);
        assert!(avg < 4.0);
    }
}
