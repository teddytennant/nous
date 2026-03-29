use serde::{Deserialize, Serialize};

use crate::chunk::ContentId;

/// A file manifest describes a file's content as an ordered list of chunk references.
///
/// Manifests are content-addressed themselves — the manifest hash is derived from
/// the ordered list of chunk hashes, forming a Merkle-like structure.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct FileManifest {
    /// Content ID of this manifest (hash of all chunk hashes in order).
    pub id: ContentId,
    /// Original filename.
    pub name: String,
    /// MIME type.
    pub mime_type: String,
    /// Total file size in bytes.
    pub total_size: u64,
    /// Ordered chunk references.
    pub chunks: Vec<ChunkRef>,
    /// Version number (1-indexed).
    pub version: u32,
    /// Previous version's manifest ID, if any.
    pub parent: Option<ContentId>,
    /// Owner DID.
    pub owner: String,
    /// Creation timestamp.
    pub created_at: chrono::DateTime<chrono::Utc>,
}

/// A reference to a chunk by its content hash and size.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ChunkRef {
    pub hash: String,
    pub offset: u64,
    pub size: u64,
}

/// Version history for a file — a linked chain of manifests.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VersionHistory {
    /// The current (latest) manifest.
    pub current: FileManifest,
    /// All previous versions, newest first.
    pub history: Vec<FileManifest>,
}

impl FileManifest {
    /// Create a new file manifest from chunk references.
    pub fn new(name: &str, mime_type: &str, chunks: Vec<ChunkRef>, owner: &str) -> Self {
        let total_size: u64 = chunks.iter().map(|c| c.size).sum();
        let id = Self::compute_id(&chunks);

        Self {
            id,
            name: name.to_string(),
            mime_type: mime_type.to_string(),
            total_size,
            chunks,
            version: 1,
            parent: None,
            owner: owner.to_string(),
            created_at: chrono::Utc::now(),
        }
    }

    /// Create a new version of this manifest with updated chunks.
    pub fn new_version(&self, chunks: Vec<ChunkRef>) -> Self {
        let total_size: u64 = chunks.iter().map(|c| c.size).sum();
        let id = Self::compute_id(&chunks);

        Self {
            id,
            name: self.name.clone(),
            mime_type: self.mime_type.clone(),
            total_size,
            chunks,
            version: self.version + 1,
            parent: Some(self.id.clone()),
            owner: self.owner.clone(),
            created_at: chrono::Utc::now(),
        }
    }

    /// Compute the manifest ID from the ordered chunk hashes.
    fn compute_id(chunks: &[ChunkRef]) -> ContentId {
        use sha2::{Digest, Sha256};
        let mut hasher = Sha256::new();
        for chunk in chunks {
            hasher.update(chunk.hash.as_bytes());
        }
        let hash = hasher.finalize();
        ContentId(hash.iter().fold(String::new(), |mut s, b| {
            use std::fmt::Write;
            let _ = write!(s, "{b:02x}");
            s
        }))
    }

    /// Compute which chunks differ between two manifests.
    /// Returns (added, removed) chunk hashes.
    pub fn diff(&self, other: &FileManifest) -> (Vec<String>, Vec<String>) {
        use std::collections::HashSet;

        let self_hashes: HashSet<&str> = self.chunks.iter().map(|c| c.hash.as_str()).collect();
        let other_hashes: HashSet<&str> = other.chunks.iter().map(|c| c.hash.as_str()).collect();

        let added: Vec<String> = other_hashes
            .difference(&self_hashes)
            .map(|h| h.to_string())
            .collect();
        let removed: Vec<String> = self_hashes
            .difference(&other_hashes)
            .map(|h| h.to_string())
            .collect();

        (added, removed)
    }
}

impl VersionHistory {
    /// Create a new version history from the first manifest.
    pub fn new(manifest: FileManifest) -> Self {
        Self {
            current: manifest,
            history: Vec::new(),
        }
    }

    /// Push a new version onto the history.
    pub fn push(&mut self, manifest: FileManifest) {
        let old = std::mem::replace(&mut self.current, manifest);
        self.history.insert(0, old);
    }

    /// Get the version count.
    pub fn version_count(&self) -> usize {
        1 + self.history.len()
    }

    /// Get a specific version (1-indexed).
    pub fn get_version(&self, version: u32) -> Option<&FileManifest> {
        if version == self.current.version {
            Some(&self.current)
        } else {
            self.history.iter().find(|m| m.version == version)
        }
    }

    /// Roll back to a previous version. Returns the removed versions.
    pub fn rollback_to(&mut self, version: u32) -> Option<Vec<FileManifest>> {
        if version >= self.current.version {
            return None;
        }

        let idx = self.history.iter().position(|m| m.version == version)?;

        // Remove all history entries before the target (newer versions).
        let removed_from_history: Vec<FileManifest> = self.history.drain(..idx).collect();
        let old_current = std::mem::replace(&mut self.current, self.history.remove(0));

        let mut removed = vec![old_current];
        removed.extend(removed_from_history);
        Some(removed)
    }
}

impl From<&crate::chunk::Chunk> for ChunkRef {
    fn from(chunk: &crate::chunk::Chunk) -> Self {
        Self {
            hash: chunk.hash.clone(),
            offset: chunk.offset,
            size: chunk.size,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_chunks() -> Vec<ChunkRef> {
        vec![
            ChunkRef {
                hash: "aaa".into(),
                offset: 0,
                size: 100,
            },
            ChunkRef {
                hash: "bbb".into(),
                offset: 100,
                size: 200,
            },
        ]
    }

    fn other_chunks() -> Vec<ChunkRef> {
        vec![
            ChunkRef {
                hash: "aaa".into(),
                offset: 0,
                size: 100,
            },
            ChunkRef {
                hash: "ccc".into(),
                offset: 100,
                size: 150,
            },
        ]
    }

    #[test]
    fn create_manifest() {
        let manifest = FileManifest::new("test.txt", "text/plain", sample_chunks(), "did:key:z123");
        assert_eq!(manifest.name, "test.txt");
        assert_eq!(manifest.total_size, 300);
        assert_eq!(manifest.version, 1);
        assert!(manifest.parent.is_none());
        assert_eq!(manifest.chunks.len(), 2);
    }

    #[test]
    fn manifest_id_deterministic() {
        let m1 = FileManifest::new("a.txt", "text/plain", sample_chunks(), "did:key:z1");
        let m2 = FileManifest::new("b.txt", "text/plain", sample_chunks(), "did:key:z2");
        // Same chunks → same content ID (name/owner don't affect content hash).
        assert_eq!(m1.id, m2.id);
    }

    #[test]
    fn manifest_id_differs_for_different_chunks() {
        let m1 = FileManifest::new("a.txt", "text/plain", sample_chunks(), "did:key:z1");
        let m2 = FileManifest::new("a.txt", "text/plain", other_chunks(), "did:key:z1");
        assert_ne!(m1.id, m2.id);
    }

    #[test]
    fn new_version() {
        let v1 = FileManifest::new("test.txt", "text/plain", sample_chunks(), "did:key:z1");
        let v2 = v1.new_version(other_chunks());

        assert_eq!(v2.version, 2);
        assert_eq!(v2.parent.as_ref(), Some(&v1.id));
        assert_eq!(v2.name, v1.name);
        assert_eq!(v2.total_size, 250);
    }

    #[test]
    fn diff_manifests() {
        let v1 = FileManifest::new("test.txt", "text/plain", sample_chunks(), "did:key:z1");
        let v2 = v1.new_version(other_chunks());

        let (added, removed) = v1.diff(&v2);
        assert_eq!(added, vec!["ccc".to_string()]);
        assert_eq!(removed, vec!["bbb".to_string()]);
    }

    #[test]
    fn diff_identical_manifests() {
        let m = FileManifest::new("a.txt", "text/plain", sample_chunks(), "did:key:z1");
        let (added, removed) = m.diff(&m);
        assert!(added.is_empty());
        assert!(removed.is_empty());
    }

    #[test]
    fn version_history_creation() {
        let m = FileManifest::new("test.txt", "text/plain", sample_chunks(), "did:key:z1");
        let history = VersionHistory::new(m.clone());
        assert_eq!(history.version_count(), 1);
        assert_eq!(history.current.version, 1);
    }

    #[test]
    fn version_history_push() {
        let v1 = FileManifest::new("test.txt", "text/plain", sample_chunks(), "did:key:z1");
        let mut history = VersionHistory::new(v1.clone());

        let v2 = v1.new_version(other_chunks());
        history.push(v2.clone());

        assert_eq!(history.version_count(), 2);
        assert_eq!(history.current.version, 2);
        assert_eq!(history.history[0].version, 1);
    }

    #[test]
    fn get_version() {
        let v1 = FileManifest::new("test.txt", "text/plain", sample_chunks(), "did:key:z1");
        let mut history = VersionHistory::new(v1.clone());
        let v2 = v1.new_version(other_chunks());
        history.push(v2);

        assert_eq!(history.get_version(1).unwrap().version, 1);
        assert_eq!(history.get_version(2).unwrap().version, 2);
        assert!(history.get_version(3).is_none());
    }

    #[test]
    fn rollback() {
        let v1 = FileManifest::new("test.txt", "text/plain", sample_chunks(), "did:key:z1");
        let mut history = VersionHistory::new(v1.clone());

        let v2 = v1.new_version(other_chunks());
        history.push(v2);

        let v3_chunks = vec![ChunkRef {
            hash: "ddd".into(),
            offset: 0,
            size: 50,
        }];
        let v3 = history.current.new_version(v3_chunks);
        history.push(v3);

        assert_eq!(history.version_count(), 3);

        let removed = history.rollback_to(1).unwrap();
        assert_eq!(history.current.version, 1);
        assert_eq!(removed.len(), 2); // v3 and v2
    }

    #[test]
    fn rollback_to_current_version_returns_none() {
        let v1 = FileManifest::new("test.txt", "text/plain", sample_chunks(), "did:key:z1");
        let mut history = VersionHistory::new(v1);
        assert!(history.rollback_to(1).is_none());
    }

    #[test]
    fn manifest_serde_roundtrip() {
        let m = FileManifest::new(
            "serde-test.bin",
            "application/octet-stream",
            sample_chunks(),
            "did:key:z1",
        );
        let json = serde_json::to_string(&m).unwrap();
        let deserialized: FileManifest = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized, m);
    }

    #[test]
    fn chunk_ref_from_chunk() {
        let chunk = crate::chunk::Chunk {
            hash: "abc123".into(),
            offset: 42,
            size: 256,
            data: vec![0u8; 256],
        };
        let chunk_ref = ChunkRef::from(&chunk);
        assert_eq!(chunk_ref.hash, "abc123");
        assert_eq!(chunk_ref.offset, 42);
        assert_eq!(chunk_ref.size, 256);
    }

    #[test]
    fn version_history_serde_roundtrip() {
        let v1 = FileManifest::new("test.txt", "text/plain", sample_chunks(), "did:key:z1");
        let mut history = VersionHistory::new(v1.clone());
        let v2 = v1.new_version(other_chunks());
        history.push(v2);

        let json = serde_json::to_string(&history).unwrap();
        let deserialized: VersionHistory = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.version_count(), 2);
        assert_eq!(deserialized.current.version, 2);
    }
}
