//! Retrieval-augmented generation (RAG) pipeline.
//!
//! Combines keyword search ([`SearchEngine`]) and semantic search
//! ([`EmbeddingIndex`]) into a unified hybrid retrieval system. Retrieved
//! chunks are assembled into structured context with source citations,
//! ready for injection into an LLM prompt.

use serde::{Deserialize, Serialize};

use crate::chunking::{ChunkOptions, chunk_text};
use crate::embedding::{Embedding, EmbeddingIndex};
use crate::search::{KeywordResult, SearchEngine};

/// How to retrieve documents.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SearchMode {
    /// BM25 keyword search only.
    Keyword,
    /// Embedding cosine similarity only.
    Semantic,
    /// Weighted combination of keyword and semantic scores.
    Hybrid,
}

/// Configuration for the RAG pipeline.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RagConfig {
    /// Maximum chunks to retrieve.
    pub max_chunks: usize,
    /// Minimum score threshold (0.0–1.0). Chunks below this are dropped.
    pub min_score: f32,
    /// Search mode.
    pub mode: SearchMode,
    /// Weight for keyword score in hybrid mode (0.0–1.0). Semantic gets 1 - this.
    pub keyword_weight: f32,
    /// Maximum total characters in assembled context.
    pub max_context_chars: usize,
}

impl Default for RagConfig {
    fn default() -> Self {
        Self {
            max_chunks: 8,
            min_score: 0.05,
            mode: SearchMode::Hybrid,
            keyword_weight: 0.3,
            max_context_chars: 12_000,
        }
    }
}

/// A chunk of text retrieved by the RAG pipeline, with its source and score.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RetrievedChunk {
    pub chunk_id: String,
    pub source_id: String,
    pub source_title: String,
    pub text: String,
    pub score: f32,
    pub rank: usize,
}

/// Assembled context ready for prompt injection.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RagContext {
    /// The original query.
    pub query: String,
    /// Retrieved chunks ordered by relevance.
    pub chunks: Vec<RetrievedChunk>,
    /// Formatted context string for LLM consumption.
    pub formatted: String,
    /// Total characters in the formatted context.
    pub char_count: usize,
    /// Number of chunks that were retrieved before filtering.
    pub candidates_considered: usize,
}

/// A source document registered with the RAG pipeline.
#[derive(Debug, Clone, Serialize, Deserialize)]
struct RagDocument {
    id: String,
    title: String,
    chunk_ids: Vec<String>,
}

/// Unified retrieval-augmented generation pipeline.
///
/// Ingests documents, splits them into chunks, indexes them for both keyword
/// and semantic search, and retrieves relevant context for any query.
pub struct RagPipeline {
    config: RagConfig,
    keyword_index: SearchEngine,
    semantic_index: EmbeddingIndex,
    documents: Vec<RagDocument>,
    chunk_texts: Vec<(String, String, String)>, // (chunk_id, source_id, source_title)
}

impl RagPipeline {
    pub fn new(config: RagConfig) -> Self {
        Self {
            config,
            keyword_index: SearchEngine::default(),
            semantic_index: EmbeddingIndex::new(),
            documents: Vec::new(),
            chunk_texts: Vec::new(),
        }
    }

    pub fn document_count(&self) -> usize {
        self.documents.len()
    }

    pub fn chunk_count(&self) -> usize {
        self.chunk_texts.len()
    }

    /// Ingest a document: chunk the text and add to keyword index.
    /// Returns the chunk IDs for embedding attachment.
    pub fn ingest(
        &mut self,
        id: impl Into<String>,
        title: impl Into<String>,
        text: &str,
        chunk_options: &ChunkOptions,
    ) -> Vec<String> {
        let id = id.into();
        let title = title.into();
        let chunks = chunk_text(text, chunk_options);

        let mut chunk_ids = Vec::with_capacity(chunks.len());
        for (i, chunk) in chunks.iter().enumerate() {
            let chunk_id = format!("{}:chunk:{}", id, i);

            // Index for keyword search.
            self.keyword_index
                .index(&chunk_id, &chunk.text, &title);

            self.chunk_texts
                .push((chunk_id.clone(), id.clone(), title.clone()));
            chunk_ids.push(chunk_id);
        }

        self.documents.push(RagDocument {
            id,
            title,
            chunk_ids: chunk_ids.clone(),
        });

        chunk_ids
    }

    /// Attach an embedding to a chunk for semantic search.
    pub fn set_embedding(&mut self, chunk_id: &str, embedding: Embedding) {
        // Find the metadata for this chunk.
        let meta = self
            .chunk_texts
            .iter()
            .find(|(cid, _, _)| cid == chunk_id)
            .map(|(_, _, title)| title.clone())
            .unwrap_or_default();
        self.semantic_index.insert(chunk_id, embedding, meta);
    }

    /// Retrieve relevant chunks for a query.
    pub fn retrieve(&self, query: &str, query_embedding: Option<&Embedding>) -> RagContext {
        let mut scored: Vec<(String, f32)> = Vec::new();
        let mut candidates_considered = 0;

        match self.config.mode {
            SearchMode::Keyword => {
                let results = self.keyword_index.search(query, self.config.max_chunks * 2);
                candidates_considered = results.len();
                for r in results {
                    scored.push((r.id, r.score));
                }
            }
            SearchMode::Semantic => {
                if let Some(emb) = query_embedding {
                    let results = self.semantic_index.search(emb, self.config.max_chunks * 2);
                    candidates_considered = results.len();
                    for r in results {
                        scored.push((r.id, r.score));
                    }
                }
            }
            SearchMode::Hybrid => {
                scored = self.hybrid_search(query, query_embedding);
                candidates_considered = scored.len();
            }
        }

        // Filter by minimum score.
        scored.retain(|(_, s)| *s >= self.config.min_score);

        // Sort by score descending.
        scored.sort_by(|a, b| {
            b.1.partial_cmp(&a.1)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        scored.truncate(self.config.max_chunks);

        // Build retrieved chunks.
        let mut chunks = Vec::new();
        let mut total_chars = 0;

        for (rank, (chunk_id, score)) in scored.iter().enumerate() {
            let (source_id, source_title, text) = self.chunk_metadata(chunk_id);

            // Respect character budget.
            if total_chars + text.len() > self.config.max_context_chars {
                break;
            }
            total_chars += text.len();

            chunks.push(RetrievedChunk {
                chunk_id: chunk_id.clone(),
                source_id,
                source_title,
                text,
                score: *score,
                rank: rank + 1,
            });
        }

        let formatted = format_context(&chunks);
        let char_count = formatted.len();

        RagContext {
            query: query.to_string(),
            chunks,
            formatted,
            char_count,
            candidates_considered,
        }
    }

    /// Remove a document and all its chunks from the pipeline.
    pub fn remove_document(&mut self, id: &str) -> bool {
        let Some(pos) = self.documents.iter().position(|d| d.id == id) else {
            return false;
        };

        let doc = self.documents.remove(pos);
        for chunk_id in &doc.chunk_ids {
            self.keyword_index.remove(chunk_id);
            self.semantic_index.remove(chunk_id);
        }
        self.chunk_texts.retain(|(_, src_id, _)| src_id != id);
        true
    }

    /// Hybrid search: combine keyword and semantic scores via weighted fusion.
    fn hybrid_search(&self, query: &str, query_embedding: Option<&Embedding>) -> Vec<(String, f32)> {
        let kw = self.config.keyword_weight;
        let sem = 1.0 - kw;

        // Keyword results.
        let keyword_results = self.keyword_index.search(query, self.config.max_chunks * 3);

        // Normalize keyword scores to 0..1 range.
        let max_kw_score = keyword_results
            .iter()
            .map(|r| r.score)
            .fold(0.0f32, f32::max);

        let mut merged: std::collections::HashMap<String, f32> = std::collections::HashMap::new();

        for r in &keyword_results {
            let norm_score = if max_kw_score > 0.0 {
                r.score / max_kw_score
            } else {
                0.0
            };
            *merged.entry(r.id.clone()).or_insert(0.0) += norm_score * kw;
        }

        // Semantic results.
        if let Some(emb) = query_embedding {
            let semantic_results = self.semantic_index.search(emb, self.config.max_chunks * 3);
            for r in &semantic_results {
                // Cosine similarity is already in [-1, 1], clamp to [0, 1].
                let norm_score = r.score.max(0.0);
                *merged.entry(r.id.clone()).or_insert(0.0) += norm_score * sem;
            }
        } else {
            // No embedding, fall back to keyword-only with full weight.
            for r in &keyword_results {
                let norm_score = if max_kw_score > 0.0 {
                    r.score / max_kw_score
                } else {
                    0.0
                };
                let entry = merged.entry(r.id.clone()).or_insert(0.0);
                *entry = norm_score; // Keyword gets full weight when no embedding.
            }
        }

        merged.into_iter().collect()
    }

    /// Look up chunk text and source info.
    fn chunk_metadata(&self, chunk_id: &str) -> (String, String, String) {
        // Find in keyword index first (it stores the full text as indexed content).
        let keyword_result = self.keyword_index.search(chunk_id, 1);
        let text = keyword_result
            .first()
            .map(|r| r.metadata.clone())
            .unwrap_or_default();

        // Find source info.
        let (source_id, source_title) = self
            .chunk_texts
            .iter()
            .find(|(cid, _, _)| cid == chunk_id)
            .map(|(_, sid, title)| (sid.clone(), title.clone()))
            .unwrap_or_default();

        // If we didn't get text from keyword search, try the metadata store.
        let text = if text.is_empty() {
            source_title.clone()
        } else {
            text
        };

        (source_id, source_title, text)
    }
}

/// Format retrieved chunks into a structured context string for LLM injection.
fn format_context(chunks: &[RetrievedChunk]) -> String {
    if chunks.is_empty() {
        return String::new();
    }

    let mut out = String::with_capacity(chunks.iter().map(|c| c.text.len() + 80).sum());

    for chunk in chunks {
        out.push_str(&format!(
            "[Source: {} | Relevance: {:.2}]\n{}\n\n",
            chunk.source_title, chunk.score, chunk.text,
        ));
    }

    out.truncate(out.trim_end().len());
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::chunking::ChunkStrategy;

    fn test_config() -> RagConfig {
        RagConfig {
            max_chunks: 5,
            min_score: 0.01,
            mode: SearchMode::Keyword,
            keyword_weight: 0.3,
            max_context_chars: 10_000,
        }
    }

    fn chunk_opts() -> ChunkOptions {
        ChunkOptions {
            max_chars: 200,
            overlap: 0,
            strategy: ChunkStrategy::Paragraph,
        }
    }

    fn make_embedding(dim: usize, seed: u32) -> Embedding {
        let vector: Vec<f32> = (0..dim)
            .map(|i| ((i as f32 + seed as f32) * 0.1).sin())
            .collect();
        Embedding::new(vector, "test")
    }

    #[test]
    fn empty_pipeline() {
        let pipeline = RagPipeline::new(test_config());
        assert_eq!(pipeline.document_count(), 0);
        assert_eq!(pipeline.chunk_count(), 0);
    }

    #[test]
    fn ingest_document() {
        let mut pipeline = RagPipeline::new(test_config());
        let ids = pipeline.ingest("doc-1", "Governance Guide", "Quadratic voting enables more nuanced preference expression.", &chunk_opts());

        assert_eq!(pipeline.document_count(), 1);
        assert!(!ids.is_empty());
        assert!(ids[0].starts_with("doc-1:chunk:"));
    }

    #[test]
    fn keyword_retrieval() {
        let mut pipeline = RagPipeline::new(test_config());
        pipeline.ingest("d1", "Governance", "Quadratic voting enables nuanced preference expression in DAOs.", &chunk_opts());
        pipeline.ingest("d2", "Messaging", "End-to-end encryption protects message privacy using X25519 key exchange.", &chunk_opts());
        pipeline.ingest("d3", "Identity", "Self-sovereign identity uses DID documents for decentralized authentication.", &chunk_opts());

        let ctx = pipeline.retrieve("voting governance", None);
        assert!(!ctx.chunks.is_empty());
        assert_eq!(ctx.chunks[0].source_title, "Governance");
    }

    #[test]
    fn semantic_retrieval() {
        let config = RagConfig {
            mode: SearchMode::Semantic,
            min_score: 0.0,
            ..test_config()
        };
        let mut pipeline = RagPipeline::new(config);

        let ids1 = pipeline.ingest("d1", "Governance", "Voting proposals delegation", &chunk_opts());
        let ids2 = pipeline.ingest("d2", "Messaging", "Encryption chat messages", &chunk_opts());

        // Attach embeddings.
        pipeline.set_embedding(&ids1[0], Embedding::new(vec![1.0, 0.0, 0.0], "test"));
        pipeline.set_embedding(&ids2[0], Embedding::new(vec![0.0, 1.0, 0.0], "test"));

        let query_emb = Embedding::new(vec![0.9, 0.1, 0.0], "test");
        let ctx = pipeline.retrieve("governance", Some(&query_emb));

        assert!(!ctx.chunks.is_empty());
        assert_eq!(ctx.chunks[0].source_title, "Governance");
    }

    #[test]
    fn hybrid_retrieval() {
        let config = RagConfig {
            mode: SearchMode::Hybrid,
            min_score: 0.0,
            keyword_weight: 0.5,
            ..test_config()
        };
        let mut pipeline = RagPipeline::new(config);

        let ids1 = pipeline.ingest("d1", "Governance", "Quadratic voting delegation proposals", &chunk_opts());
        let ids2 = pipeline.ingest("d2", "Messaging", "Encrypted peer messaging protocol", &chunk_opts());

        pipeline.set_embedding(&ids1[0], Embedding::new(vec![1.0, 0.0, 0.0], "test"));
        pipeline.set_embedding(&ids2[0], Embedding::new(vec![0.0, 1.0, 0.0], "test"));

        let query_emb = Embedding::new(vec![0.8, 0.2, 0.0], "test");
        let ctx = pipeline.retrieve("voting governance", Some(&query_emb));

        assert!(!ctx.chunks.is_empty());
        // Governance should rank higher in both keyword and semantic.
        assert_eq!(ctx.chunks[0].source_title, "Governance");
    }

    #[test]
    fn hybrid_falls_back_without_embedding() {
        let config = RagConfig {
            mode: SearchMode::Hybrid,
            ..test_config()
        };
        let mut pipeline = RagPipeline::new(config);
        pipeline.ingest("d1", "Governance", "Quadratic voting proposals", &chunk_opts());

        let ctx = pipeline.retrieve("voting", None);
        assert!(!ctx.chunks.is_empty());
    }

    #[test]
    fn min_score_filters() {
        let config = RagConfig {
            min_score: 100.0, // Impossibly high.
            ..test_config()
        };
        let mut pipeline = RagPipeline::new(config);
        pipeline.ingest("d1", "Doc", "test content here", &chunk_opts());

        let ctx = pipeline.retrieve("test", None);
        assert!(ctx.chunks.is_empty());
    }

    #[test]
    fn max_chunks_limits() {
        let config = RagConfig {
            max_chunks: 2,
            ..test_config()
        };
        let mut pipeline = RagPipeline::new(config);
        for i in 0..10 {
            pipeline.ingest(format!("d{i}"), format!("Doc {i}"), &format!("common keyword text {i}"), &chunk_opts());
        }

        let ctx = pipeline.retrieve("common keyword", None);
        assert!(ctx.chunks.len() <= 2);
    }

    #[test]
    fn context_character_budget() {
        let config = RagConfig {
            max_context_chars: 100,
            ..test_config()
        };
        let mut pipeline = RagPipeline::new(config);
        pipeline.ingest("d1", "Doc", &"keyword ".repeat(200), &chunk_opts());

        let ctx = pipeline.retrieve("keyword", None);
        // Should have limited context.
        assert!(ctx.char_count <= 200); // Some overhead from formatting.
    }

    #[test]
    fn remove_document_from_pipeline() {
        let mut pipeline = RagPipeline::new(test_config());
        pipeline.ingest("d1", "Governance", "Quadratic voting proposals", &chunk_opts());
        pipeline.ingest("d2", "Messaging", "Encrypted messaging protocol", &chunk_opts());

        assert_eq!(pipeline.document_count(), 2);
        assert!(pipeline.remove_document("d1"));
        assert_eq!(pipeline.document_count(), 1);

        let ctx = pipeline.retrieve("voting", None);
        assert!(ctx.chunks.is_empty());
    }

    #[test]
    fn remove_nonexistent_document() {
        let mut pipeline = RagPipeline::new(test_config());
        assert!(!pipeline.remove_document("ghost"));
    }

    #[test]
    fn formatted_context_structure() {
        let mut pipeline = RagPipeline::new(test_config());
        pipeline.ingest("d1", "Governance Guide", "Quadratic voting enables preference expression.", &chunk_opts());

        let ctx = pipeline.retrieve("voting", None);
        assert!(ctx.formatted.contains("[Source: Governance Guide"));
        assert!(ctx.formatted.contains("Relevance:"));
    }

    #[test]
    fn empty_query_returns_empty() {
        let mut pipeline = RagPipeline::new(test_config());
        pipeline.ingest("d1", "Doc", "content here", &chunk_opts());

        let ctx = pipeline.retrieve("", None);
        assert!(ctx.chunks.is_empty());
    }

    #[test]
    fn chunks_have_rank() {
        let mut pipeline = RagPipeline::new(test_config());
        pipeline.ingest("d1", "Doc A", "governance voting proposal system", &chunk_opts());
        pipeline.ingest("d2", "Doc B", "governance delegation mechanism", &chunk_opts());

        let ctx = pipeline.retrieve("governance", None);
        assert!(ctx.chunks.len() >= 2);
        assert_eq!(ctx.chunks[0].rank, 1);
        assert_eq!(ctx.chunks[1].rank, 2);
    }

    #[test]
    fn multiple_chunks_per_document() {
        let config = RagConfig {
            max_chunks: 10,
            ..test_config()
        };
        let mut pipeline = RagPipeline::new(config);

        let text = "Governance overview.\n\nQuadratic voting details.\n\nDelegation mechanisms.\n\nProposal lifecycle.";
        let opts = ChunkOptions {
            max_chars: 30,
            overlap: 0,
            strategy: ChunkStrategy::Paragraph,
        };
        let ids = pipeline.ingest("d1", "Governance", text, &opts);
        assert!(ids.len() > 1);

        let ctx = pipeline.retrieve("voting", None);
        assert!(!ctx.chunks.is_empty());
    }

    #[test]
    fn rag_config_serializes() {
        let config = RagConfig::default();
        let json = serde_json::to_string(&config).unwrap();
        let restored: RagConfig = serde_json::from_str(&json).unwrap();
        assert_eq!(restored.max_chunks, config.max_chunks);
        assert_eq!(restored.mode, config.mode);
    }

    #[test]
    fn retrieved_chunk_serializes() {
        let chunk = RetrievedChunk {
            chunk_id: "c1".into(),
            source_id: "d1".into(),
            source_title: "Test".into(),
            text: "content".into(),
            score: 0.85,
            rank: 1,
        };
        let json = serde_json::to_string(&chunk).unwrap();
        let restored: RetrievedChunk = serde_json::from_str(&json).unwrap();
        assert_eq!(restored.chunk_id, "c1");
        assert!((restored.score - 0.85).abs() < 1e-6);
    }
}
