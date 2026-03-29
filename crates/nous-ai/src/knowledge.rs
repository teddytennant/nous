use chrono::{DateTime, Utc};
use rusqlite::{Connection, params};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use nous_core::Error;

use crate::chunking::{ChunkOptions, chunk_text};
use crate::embedding::{Embedding, EmbeddingIndex, SearchResult};

/// A document stored in the knowledge base.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Document {
    pub id: String,
    pub title: String,
    pub content: String,
    pub source: String,
    pub chunk_count: usize,
    pub created_at: DateTime<Utc>,
}

/// A chunk stored alongside its embedding vector.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DocumentChunk {
    pub id: String,
    pub document_id: String,
    pub text: String,
    pub index: usize,
    pub offset: usize,
}

/// Result from a knowledge base search.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KnowledgeResult {
    pub chunk_id: String,
    pub document_id: String,
    pub document_title: String,
    pub text: String,
    pub score: f32,
}

/// Persistent knowledge base: documents → chunks → embeddings → semantic search.
///
/// Documents are stored in SQLite. Embeddings live in an in-memory index that
/// is rebuilt from the database on open. This keeps search fast while
/// ensuring durability.
pub struct KnowledgeBase {
    conn: Connection,
    index: EmbeddingIndex,
}

impl KnowledgeBase {
    /// Open a file-backed knowledge base.
    pub fn open(path: &std::path::Path) -> Result<Self, Error> {
        let conn = Connection::open(path)
            .map_err(|e| Error::Storage(format!("failed to open knowledge db: {e}")))?;
        Self::init(conn)
    }

    /// Create an in-memory knowledge base (useful for tests).
    pub fn in_memory() -> Result<Self, Error> {
        let conn = Connection::open_in_memory()
            .map_err(|e| Error::Storage(format!("in-memory open failed: {e}")))?;
        Self::init(conn)
    }

    fn init(conn: Connection) -> Result<Self, Error> {
        conn.execute_batch(
            "PRAGMA journal_mode = WAL;
             PRAGMA synchronous = NORMAL;
             PRAGMA foreign_keys = ON;",
        )
        .map_err(|e| Error::Storage(format!("pragma failed: {e}")))?;

        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS documents (
                id         TEXT PRIMARY KEY,
                title      TEXT NOT NULL,
                content    TEXT NOT NULL,
                source     TEXT NOT NULL DEFAULT '',
                chunk_count INTEGER NOT NULL DEFAULT 0,
                created_at TEXT NOT NULL
            );

            CREATE TABLE IF NOT EXISTS chunks (
                id          TEXT PRIMARY KEY,
                document_id TEXT NOT NULL,
                text        TEXT NOT NULL,
                chunk_index INTEGER NOT NULL,
                byte_offset INTEGER NOT NULL,
                embedding   BLOB,
                FOREIGN KEY (document_id) REFERENCES documents(id) ON DELETE CASCADE
            );
            CREATE INDEX IF NOT EXISTS idx_chunks_doc ON chunks(document_id);",
        )
        .map_err(|e| Error::Storage(format!("migration failed: {e}")))?;

        let mut kb = Self {
            conn,
            index: EmbeddingIndex::new(),
        };
        kb.rebuild_index()?;
        Ok(kb)
    }

    /// Rebuild the in-memory embedding index from stored vectors.
    fn rebuild_index(&mut self) -> Result<(), Error> {
        self.index = EmbeddingIndex::new();

        let mut stmt = self
            .conn
            .prepare(
                "SELECT c.id, c.embedding, c.text
                 FROM chunks c
                 WHERE c.embedding IS NOT NULL",
            )
            .map_err(|e| Error::Storage(format!("prepare rebuild failed: {e}")))?;

        let rows = stmt
            .query_map([], |row| {
                let id: String = row.get(0)?;
                let blob: Vec<u8> = row.get(1)?;
                let text: String = row.get(2)?;
                Ok((id, blob, text))
            })
            .map_err(|e| Error::Storage(format!("query rebuild failed: {e}")))?;

        for row in rows {
            let (id, blob, text) = row.map_err(|e| Error::Storage(format!("row error: {e}")))?;
            if let Some(emb) = deserialize_embedding(&blob) {
                self.index.insert(id, emb, text);
            }
        }

        Ok(())
    }

    /// Ingest a document: chunk it and store the chunks. Embeddings are not
    /// generated here — call `set_chunk_embedding` for each chunk after
    /// generating vectors through your embedding model.
    pub fn ingest(
        &mut self,
        title: &str,
        content: &str,
        source: &str,
        options: &ChunkOptions,
    ) -> Result<Document, Error> {
        if title.trim().is_empty() {
            return Err(Error::InvalidInput("document title cannot be empty".into()));
        }
        if content.trim().is_empty() {
            return Err(Error::InvalidInput(
                "document content cannot be empty".into(),
            ));
        }

        let doc_id = format!("doc:{}", Uuid::new_v4());
        let now = Utc::now();
        let chunks = chunk_text(content, options);

        self.conn
            .execute(
                "INSERT INTO documents (id, title, content, source, chunk_count, created_at)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
                params![
                    doc_id,
                    title,
                    content,
                    source,
                    chunks.len() as i64,
                    now.to_rfc3339()
                ],
            )
            .map_err(|e| Error::Storage(format!("insert document failed: {e}")))?;

        for chunk in &chunks {
            let chunk_id = format!("chunk:{}", Uuid::new_v4());
            self.conn
                .execute(
                    "INSERT INTO chunks (id, document_id, text, chunk_index, byte_offset)
                     VALUES (?1, ?2, ?3, ?4, ?5)",
                    params![
                        chunk_id,
                        doc_id,
                        chunk.text,
                        chunk.index as i64,
                        chunk.offset as i64
                    ],
                )
                .map_err(|e| Error::Storage(format!("insert chunk failed: {e}")))?;
        }

        Ok(Document {
            id: doc_id,
            title: title.to_string(),
            content: content.to_string(),
            source: source.to_string(),
            chunk_count: chunks.len(),
            created_at: now,
        })
    }

    /// Attach an embedding vector to a chunk and add it to the search index.
    pub fn set_chunk_embedding(
        &mut self,
        chunk_id: &str,
        embedding: Embedding,
    ) -> Result<(), Error> {
        let blob = serialize_embedding(&embedding);
        let text: String = self
            .conn
            .query_row(
                "SELECT text FROM chunks WHERE id = ?1",
                params![chunk_id],
                |row| row.get(0),
            )
            .map_err(|e| Error::NotFound(format!("chunk {chunk_id}: {e}")))?;

        self.conn
            .execute(
                "UPDATE chunks SET embedding = ?1 WHERE id = ?2",
                params![blob, chunk_id],
            )
            .map_err(|e| Error::Storage(format!("update embedding failed: {e}")))?;

        self.index.insert(chunk_id, embedding, text);
        Ok(())
    }

    /// Semantic search across all indexed chunks.
    pub fn search(
        &self,
        query_embedding: &Embedding,
        limit: usize,
    ) -> Result<Vec<KnowledgeResult>, Error> {
        let raw = self.index.search(query_embedding, limit);
        let mut results = Vec::with_capacity(raw.len());

        for SearchResult {
            id,
            score,
            metadata,
        } in raw
        {
            let doc_title = self.document_title_for_chunk(&id).unwrap_or_default();
            let doc_id = self.document_id_for_chunk(&id).unwrap_or_default();
            results.push(KnowledgeResult {
                chunk_id: id,
                document_id: doc_id,
                document_title: doc_title,
                text: metadata,
                score,
            });
        }

        Ok(results)
    }

    /// List all documents in the knowledge base.
    pub fn list_documents(&self) -> Result<Vec<Document>, Error> {
        let mut stmt = self
            .conn
            .prepare("SELECT id, title, content, source, chunk_count, created_at FROM documents ORDER BY created_at DESC")
            .map_err(|e| Error::Storage(format!("prepare list failed: {e}")))?;

        let rows = stmt
            .query_map([], |row| {
                let created_str: String = row.get(5)?;
                Ok(Document {
                    id: row.get(0)?,
                    title: row.get(1)?,
                    content: row.get(2)?,
                    source: row.get(3)?,
                    chunk_count: row.get::<_, i64>(4)? as usize,
                    created_at: DateTime::parse_from_rfc3339(&created_str)
                        .map(|dt| dt.with_timezone(&Utc))
                        .unwrap_or_else(|_| Utc::now()),
                })
            })
            .map_err(|e| Error::Storage(format!("query list failed: {e}")))?;

        rows.collect::<Result<Vec<_>, _>>()
            .map_err(|e| Error::Storage(format!("collect docs failed: {e}")))
    }

    /// Get a document by id.
    pub fn get_document(&self, id: &str) -> Result<Option<Document>, Error> {
        let mut stmt = self
            .conn
            .prepare("SELECT id, title, content, source, chunk_count, created_at FROM documents WHERE id = ?1")
            .map_err(|e| Error::Storage(format!("prepare get failed: {e}")))?;

        let result = stmt.query_row(params![id], |row| {
            let created_str: String = row.get(5)?;
            Ok(Document {
                id: row.get(0)?,
                title: row.get(1)?,
                content: row.get(2)?,
                source: row.get(3)?,
                chunk_count: row.get::<_, i64>(4)? as usize,
                created_at: DateTime::parse_from_rfc3339(&created_str)
                    .map(|dt| dt.with_timezone(&Utc))
                    .unwrap_or_else(|_| Utc::now()),
            })
        });

        match result {
            Ok(doc) => Ok(Some(doc)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(Error::Storage(format!("get document failed: {e}"))),
        }
    }

    /// List chunks belonging to a document.
    pub fn get_chunks(&self, document_id: &str) -> Result<Vec<DocumentChunk>, Error> {
        let mut stmt = self
            .conn
            .prepare("SELECT id, document_id, text, chunk_index, byte_offset FROM chunks WHERE document_id = ?1 ORDER BY chunk_index")
            .map_err(|e| Error::Storage(format!("prepare chunks failed: {e}")))?;

        let rows = stmt
            .query_map(params![document_id], |row| {
                Ok(DocumentChunk {
                    id: row.get(0)?,
                    document_id: row.get(1)?,
                    text: row.get(2)?,
                    index: row.get::<_, i64>(3)? as usize,
                    offset: row.get::<_, i64>(4)? as usize,
                })
            })
            .map_err(|e| Error::Storage(format!("query chunks failed: {e}")))?;

        rows.collect::<Result<Vec<_>, _>>()
            .map_err(|e| Error::Storage(format!("collect chunks failed: {e}")))
    }

    /// Delete a document and all its chunks. Removes embeddings from the index.
    pub fn delete_document(&mut self, id: &str) -> Result<bool, Error> {
        let chunk_ids: Vec<String> = {
            let mut stmt = self
                .conn
                .prepare("SELECT id FROM chunks WHERE document_id = ?1")
                .map_err(|e| Error::Storage(format!("prepare delete failed: {e}")))?;
            let rows = stmt
                .query_map(params![id], |row| row.get(0))
                .map_err(|e| Error::Storage(format!("query chunks for delete failed: {e}")))?;
            rows.collect::<Result<Vec<_>, _>>()
                .map_err(|e| Error::Storage(format!("collect chunk ids failed: {e}")))?
        };

        for cid in &chunk_ids {
            self.index.remove(cid);
        }

        self.conn
            .execute("DELETE FROM chunks WHERE document_id = ?1", params![id])
            .map_err(|e| Error::Storage(format!("delete chunks failed: {e}")))?;

        let count = self
            .conn
            .execute("DELETE FROM documents WHERE id = ?1", params![id])
            .map_err(|e| Error::Storage(format!("delete document failed: {e}")))?;

        Ok(count > 0)
    }

    /// Total number of documents.
    pub fn document_count(&self) -> Result<usize, Error> {
        let count: i64 = self
            .conn
            .query_row("SELECT COUNT(*) FROM documents", [], |row| row.get(0))
            .map_err(|e| Error::Storage(format!("count failed: {e}")))?;
        Ok(count as usize)
    }

    /// Total number of indexed chunks (those with embeddings).
    pub fn indexed_chunk_count(&self) -> usize {
        self.index.len()
    }

    fn document_title_for_chunk(&self, chunk_id: &str) -> Option<String> {
        self.conn
            .query_row(
                "SELECT d.title FROM documents d JOIN chunks c ON c.document_id = d.id WHERE c.id = ?1",
                params![chunk_id],
                |row| row.get(0),
            )
            .ok()
    }

    fn document_id_for_chunk(&self, chunk_id: &str) -> Option<String> {
        self.conn
            .query_row(
                "SELECT document_id FROM chunks WHERE id = ?1",
                params![chunk_id],
                |row| row.get(0),
            )
            .ok()
    }
}

fn serialize_embedding(emb: &Embedding) -> Vec<u8> {
    let mut buf = Vec::with_capacity(emb.vector.len() * 4);
    for &v in &emb.vector {
        buf.extend_from_slice(&v.to_le_bytes());
    }
    buf
}

fn deserialize_embedding(blob: &[u8]) -> Option<Embedding> {
    if !blob.len().is_multiple_of(4) {
        return None;
    }
    let vector: Vec<f32> = blob
        .chunks_exact(4)
        .map(|b| f32::from_le_bytes([b[0], b[1], b[2], b[3]]))
        .collect();
    Some(Embedding::new(vector, "stored"))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_kb() -> KnowledgeBase {
        KnowledgeBase::in_memory().unwrap()
    }

    fn make_embedding(dim: usize, val: f32) -> Embedding {
        Embedding::new(vec![val; dim], "test")
    }

    #[test]
    fn ingest_document() {
        let mut kb = test_kb();
        let doc = kb
            .ingest(
                "Test Doc",
                "Hello world.\n\nSecond paragraph.",
                "",
                &ChunkOptions::default(),
            )
            .unwrap();
        assert!(doc.id.starts_with("doc:"));
        assert_eq!(doc.title, "Test Doc");
        assert_eq!(doc.chunk_count, 1); // short text, merges into one chunk
    }

    #[test]
    fn ingest_rejects_empty_title() {
        let mut kb = test_kb();
        let result = kb.ingest("", "content", "", &ChunkOptions::default());
        assert!(result.is_err());
    }

    #[test]
    fn ingest_rejects_empty_content() {
        let mut kb = test_kb();
        let result = kb.ingest("Title", "", "", &ChunkOptions::default());
        assert!(result.is_err());
    }

    #[test]
    fn list_documents() {
        let mut kb = test_kb();
        kb.ingest("Doc A", "Content A", "src-a", &ChunkOptions::default())
            .unwrap();
        kb.ingest("Doc B", "Content B", "src-b", &ChunkOptions::default())
            .unwrap();

        let docs = kb.list_documents().unwrap();
        assert_eq!(docs.len(), 2);
    }

    #[test]
    fn get_document() {
        let mut kb = test_kb();
        let doc = kb
            .ingest("My Doc", "Some content", "test", &ChunkOptions::default())
            .unwrap();

        let fetched = kb.get_document(&doc.id).unwrap().unwrap();
        assert_eq!(fetched.title, "My Doc");
    }

    #[test]
    fn get_missing_document_returns_none() {
        let kb = test_kb();
        assert!(kb.get_document("doc:nonexistent").unwrap().is_none());
    }

    #[test]
    fn get_chunks() {
        let mut kb = test_kb();
        let opts = ChunkOptions {
            max_chars: 20,
            strategy: crate::chunking::ChunkStrategy::Paragraph,
            ..Default::default()
        };
        let doc = kb
            .ingest("Chunked", "Alpha.\n\nBravo.\n\nCharlie.", "", &opts)
            .unwrap();

        let chunks = kb.get_chunks(&doc.id).unwrap();
        assert_eq!(chunks.len(), doc.chunk_count);
        assert_eq!(chunks[0].index, 0);
    }

    #[test]
    fn delete_document() {
        let mut kb = test_kb();
        let doc = kb
            .ingest("To Delete", "Content", "", &ChunkOptions::default())
            .unwrap();
        assert_eq!(kb.document_count().unwrap(), 1);

        assert!(kb.delete_document(&doc.id).unwrap());
        assert_eq!(kb.document_count().unwrap(), 0);
        assert!(kb.get_chunks(&doc.id).unwrap().is_empty());
    }

    #[test]
    fn delete_missing_returns_false() {
        let mut kb = test_kb();
        assert!(!kb.delete_document("doc:ghost").unwrap());
    }

    #[test]
    fn set_embedding_and_search() {
        let mut kb = test_kb();
        let doc = kb
            .ingest(
                "Cats",
                "All about cats and kittens.",
                "",
                &ChunkOptions::default(),
            )
            .unwrap();

        let chunks = kb.get_chunks(&doc.id).unwrap();
        assert!(!chunks.is_empty());

        let emb = make_embedding(8, 1.0);
        kb.set_chunk_embedding(&chunks[0].id, emb).unwrap();

        assert_eq!(kb.indexed_chunk_count(), 1);

        let query = make_embedding(8, 1.0);
        let results = kb.search(&query, 5).unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].document_title, "Cats");
        assert!((results[0].score - 1.0).abs() < 1e-6);
    }

    #[test]
    fn search_ranks_by_similarity() {
        let mut kb = test_kb();

        let doc_a = kb
            .ingest("Alpha", "Alpha content", "", &ChunkOptions::default())
            .unwrap();
        let doc_b = kb
            .ingest("Beta", "Beta content", "", &ChunkOptions::default())
            .unwrap();

        let chunks_a = kb.get_chunks(&doc_a.id).unwrap();
        let chunks_b = kb.get_chunks(&doc_b.id).unwrap();

        // Alpha gets a vector close to the query; Beta gets an orthogonal one.
        kb.set_chunk_embedding(&chunks_a[0].id, Embedding::new(vec![1.0, 0.0, 0.0], "test"))
            .unwrap();
        kb.set_chunk_embedding(&chunks_b[0].id, Embedding::new(vec![0.0, 1.0, 0.0], "test"))
            .unwrap();

        let query = Embedding::new(vec![1.0, 0.0, 0.0], "test");
        let results = kb.search(&query, 10).unwrap();

        assert_eq!(results.len(), 2);
        assert_eq!(results[0].document_title, "Alpha");
        assert!(results[0].score > results[1].score);
    }

    #[test]
    fn delete_removes_from_index() {
        let mut kb = test_kb();
        let doc = kb
            .ingest(
                "Indexed",
                "Indexed content here.",
                "",
                &ChunkOptions::default(),
            )
            .unwrap();
        let chunks = kb.get_chunks(&doc.id).unwrap();
        kb.set_chunk_embedding(&chunks[0].id, make_embedding(4, 1.0))
            .unwrap();
        assert_eq!(kb.indexed_chunk_count(), 1);

        kb.delete_document(&doc.id).unwrap();
        assert_eq!(kb.indexed_chunk_count(), 0);
    }

    #[test]
    fn embedding_serialization_roundtrip() {
        let emb = Embedding::new(vec![1.0, -2.5, 3.14, 0.0], "test");
        let blob = serialize_embedding(&emb);
        let restored = deserialize_embedding(&blob).unwrap();
        assert_eq!(restored.vector, emb.vector);
    }

    #[test]
    fn document_count() {
        let mut kb = test_kb();
        assert_eq!(kb.document_count().unwrap(), 0);
        kb.ingest("A", "Content", "", &ChunkOptions::default())
            .unwrap();
        assert_eq!(kb.document_count().unwrap(), 1);
    }

    #[test]
    fn file_backed_persistence() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("knowledge.db");

        let doc_id = {
            let mut kb = KnowledgeBase::open(&path).unwrap();
            let doc = kb
                .ingest(
                    "Persisted",
                    "This will survive.",
                    "",
                    &ChunkOptions::default(),
                )
                .unwrap();
            doc.id
        };

        {
            let kb = KnowledgeBase::open(&path).unwrap();
            let doc = kb.get_document(&doc_id).unwrap().unwrap();
            assert_eq!(doc.title, "Persisted");
        }
    }
}
