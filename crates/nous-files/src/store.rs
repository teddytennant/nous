use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use nous_core::{Error, Result};

use crate::chunk::{self, ContentId};
use crate::manifest::{ChunkRef, FileManifest, VersionHistory};

/// A content-addressed file store.
///
/// Files are split into chunks, deduplicated by content hash, and tracked
/// via manifests. This provides:
/// - Deduplication: identical chunks stored only once
/// - Efficient diffs: only changed chunks transferred on updates
/// - Integrity: every chunk verified by its content hash
#[derive(Debug, Serialize, Deserialize)]
pub struct FileStore {
    /// Chunk data indexed by content hash.
    chunks: HashMap<String, Vec<u8>>,
    /// File manifests indexed by content ID.
    manifests: HashMap<String, FileManifest>,
    /// Version histories indexed by filename + owner.
    histories: HashMap<String, VersionHistory>,
    /// Total bytes stored (after dedup).
    stored_bytes: u64,
}

/// Statistics about the file store.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StoreStats {
    pub total_chunks: usize,
    pub total_manifests: usize,
    pub total_files: usize,
    pub stored_bytes: u64,
    pub logical_bytes: u64,
    pub dedup_ratio: f64,
}

impl FileStore {
    /// Create a new empty file store.
    pub fn new() -> Self {
        Self {
            chunks: HashMap::new(),
            manifests: HashMap::new(),
            histories: HashMap::new(),
            stored_bytes: 0,
        }
    }

    /// Store a file by splitting it into chunks and creating a manifest.
    pub fn put(
        &mut self,
        name: &str,
        mime_type: &str,
        data: &[u8],
        owner: &str,
    ) -> Result<FileManifest> {
        let chunks = chunk::chunk_data(data)?;
        let chunk_refs: Vec<ChunkRef> = chunks.iter().map(ChunkRef::from).collect();

        // Store chunks (deduplicating).
        for c in &chunks {
            if !self.chunks.contains_key(&c.hash) {
                self.stored_bytes += c.size;
                self.chunks.insert(c.hash.clone(), c.data.clone());
            }
        }

        let history_key = format!("{owner}:{name}");

        let manifest = if let Some(history) = self.histories.get(&history_key) {
            // New version of existing file.
            history.current.new_version(chunk_refs)
        } else {
            // First version.
            FileManifest::new(name, mime_type, chunk_refs, owner)
        };

        self.manifests
            .insert(manifest.id.0.clone(), manifest.clone());

        if let Some(history) = self.histories.get_mut(&history_key) {
            history.push(manifest.clone());
        } else {
            self.histories
                .insert(history_key, VersionHistory::new(manifest.clone()));
        }

        Ok(manifest)
    }

    /// Retrieve a file by reassembling its chunks from a manifest.
    pub fn get(&self, manifest_id: &str) -> Result<Vec<u8>> {
        let manifest = self
            .manifests
            .get(manifest_id)
            .ok_or_else(|| Error::NotFound(format!("manifest {manifest_id} not found")))?;

        self.get_by_manifest(manifest)
    }

    /// Retrieve a file using a manifest reference.
    pub fn get_by_manifest(&self, manifest: &FileManifest) -> Result<Vec<u8>> {
        let mut data = Vec::with_capacity(manifest.total_size as usize);

        for chunk_ref in &manifest.chunks {
            let chunk_data = self
                .chunks
                .get(&chunk_ref.hash)
                .ok_or_else(|| Error::NotFound(format!("chunk {} missing", chunk_ref.hash)))?;
            data.extend_from_slice(chunk_data);
        }

        Ok(data)
    }

    /// Get the latest version of a file by name and owner.
    pub fn get_latest(&self, name: &str, owner: &str) -> Result<Vec<u8>> {
        let history_key = format!("{owner}:{name}");
        let history = self
            .histories
            .get(&history_key)
            .ok_or_else(|| Error::NotFound(format!("file '{name}' not found")))?;

        self.get_by_manifest(&history.current)
    }

    /// Get the version history for a file.
    pub fn get_history(&self, name: &str, owner: &str) -> Option<&VersionHistory> {
        let key = format!("{owner}:{name}");
        self.histories.get(&key)
    }

    /// Get a manifest by its content ID.
    pub fn get_manifest(&self, id: &str) -> Option<&FileManifest> {
        self.manifests.get(id)
    }

    /// Check if a chunk exists in the store.
    pub fn has_chunk(&self, hash: &str) -> bool {
        self.chunks.contains_key(hash)
    }

    /// Get a raw chunk by hash.
    pub fn get_chunk(&self, hash: &str) -> Option<&[u8]> {
        self.chunks.get(hash).map(|v| v.as_slice())
    }

    /// Import a chunk directly (for P2P sync).
    pub fn import_chunk(&mut self, hash: String, data: Vec<u8>) -> Result<()> {
        // Verify hash.
        let actual = ContentId::from_bytes(&data);
        if actual.0 != hash {
            return Err(Error::Crypto(format!(
                "chunk hash mismatch: expected {hash}, got {}",
                actual.0
            )));
        }

        if !self.chunks.contains_key(&hash) {
            self.stored_bytes += data.len() as u64;
            self.chunks.insert(hash, data);
        }

        Ok(())
    }

    /// List all files (latest versions) for an owner.
    pub fn list_files(&self, owner: &str) -> Vec<&FileManifest> {
        self.histories
            .values()
            .filter(|h| h.current.owner == owner)
            .map(|h| &h.current)
            .collect()
    }

    /// Delete a file and all its versions. Returns freed bytes.
    pub fn delete(&mut self, name: &str, owner: &str) -> Result<u64> {
        let key = format!("{owner}:{name}");
        let history = self
            .histories
            .remove(&key)
            .ok_or_else(|| Error::NotFound(format!("file '{name}' not found")))?;

        // Collect all chunk hashes from all versions.
        let mut file_chunk_hashes: std::collections::HashSet<String> = std::collections::HashSet::new();
        for chunk_ref in &history.current.chunks {
            file_chunk_hashes.insert(chunk_ref.hash.clone());
        }
        for version in &history.history {
            for chunk_ref in &version.chunks {
                file_chunk_hashes.insert(chunk_ref.hash.clone());
            }
        }

        // Remove manifests.
        self.manifests.remove(&history.current.id.0);
        for version in &history.history {
            self.manifests.remove(&version.id.0);
        }

        // Only remove chunks not referenced by other files.
        let referenced: std::collections::HashSet<String> = self
            .histories
            .values()
            .flat_map(|h| {
                let current_chunks = h.current.chunks.iter().map(|c| c.hash.clone());
                let history_chunks = h
                    .history
                    .iter()
                    .flat_map(|v| v.chunks.iter().map(|c| c.hash.clone()));
                current_chunks.chain(history_chunks)
            })
            .collect();

        let mut freed = 0u64;
        for hash in &file_chunk_hashes {
            if !referenced.contains(hash) {
                if let Some(data) = self.chunks.remove(hash) {
                    freed += data.len() as u64;
                    self.stored_bytes -= data.len() as u64;
                }
            }
        }

        Ok(freed)
    }

    /// Get store statistics.
    pub fn stats(&self) -> StoreStats {
        let logical_bytes: u64 = self
            .histories
            .values()
            .map(|h| h.current.total_size)
            .sum();

        let dedup_ratio = if self.stored_bytes > 0 {
            logical_bytes as f64 / self.stored_bytes as f64
        } else {
            1.0
        };

        StoreStats {
            total_chunks: self.chunks.len(),
            total_manifests: self.manifests.len(),
            total_files: self.histories.len(),
            stored_bytes: self.stored_bytes,
            logical_bytes,
            dedup_ratio,
        }
    }
}

impl Default for FileStore {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const OWNER: &str = "did:key:zTestOwner";

    #[test]
    fn new_store_is_empty() {
        let store = FileStore::new();
        let stats = store.stats();
        assert_eq!(stats.total_chunks, 0);
        assert_eq!(stats.total_files, 0);
        assert_eq!(stats.stored_bytes, 0);
    }

    #[test]
    fn put_and_get_small_file() {
        let mut store = FileStore::new();
        let data = b"amor fati";
        let manifest = store.put("test.txt", "text/plain", data, OWNER).unwrap();

        let retrieved = store.get(&manifest.id.0).unwrap();
        assert_eq!(retrieved, data);
    }

    #[test]
    fn put_and_get_large_file() {
        let mut store = FileStore::new();
        let data = vec![0xABu8; 512 * 1024]; // 512 KiB
        let manifest = store.put("big.bin", "application/octet-stream", &data, OWNER).unwrap();

        let retrieved = store.get(&manifest.id.0).unwrap();
        assert_eq!(retrieved, data);
    }

    #[test]
    fn get_latest_version() {
        let mut store = FileStore::new();
        store.put("readme.md", "text/markdown", b"version 1", OWNER).unwrap();
        store.put("readme.md", "text/markdown", b"version 2", OWNER).unwrap();

        let latest = store.get_latest("readme.md", OWNER).unwrap();
        assert_eq!(latest, b"version 2");
    }

    #[test]
    fn version_history_tracked() {
        let mut store = FileStore::new();
        store.put("doc.txt", "text/plain", b"v1", OWNER).unwrap();
        store.put("doc.txt", "text/plain", b"v2", OWNER).unwrap();
        store.put("doc.txt", "text/plain", b"v3", OWNER).unwrap();

        let history = store.get_history("doc.txt", OWNER).unwrap();
        assert_eq!(history.version_count(), 3);
        assert_eq!(history.current.version, 3);
    }

    #[test]
    fn deduplication() {
        let mut store = FileStore::new();
        let data = b"identical content across files";

        store.put("file1.txt", "text/plain", data, OWNER).unwrap();
        let stats_after_first = store.stats();

        store.put("file2.txt", "text/plain", data, OWNER).unwrap();
        let stats_after_second = store.stats();

        // Same chunks should not increase stored bytes.
        assert_eq!(stats_after_first.stored_bytes, stats_after_second.stored_bytes);
        assert_eq!(stats_after_second.total_files, 2);
    }

    #[test]
    fn list_files() {
        let mut store = FileStore::new();
        store.put("a.txt", "text/plain", b"aaa", OWNER).unwrap();
        store.put("b.txt", "text/plain", b"bbb", OWNER).unwrap();
        store.put("c.txt", "text/plain", b"ccc", "did:key:zOther").unwrap();

        let files = store.list_files(OWNER);
        assert_eq!(files.len(), 2);
    }

    #[test]
    fn delete_file() {
        let mut store = FileStore::new();
        store.put("temp.txt", "text/plain", b"temporary data", OWNER).unwrap();
        assert_eq!(store.stats().total_files, 1);

        let freed = store.delete("temp.txt", OWNER).unwrap();
        assert!(freed > 0);
        assert_eq!(store.stats().total_files, 0);
        assert_eq!(store.stats().total_chunks, 0);
    }

    #[test]
    fn delete_preserves_shared_chunks() {
        let mut store = FileStore::new();
        let data = b"shared chunk data";

        store.put("a.txt", "text/plain", data, OWNER).unwrap();
        store.put("b.txt", "text/plain", data, OWNER).unwrap();

        let chunks_before = store.stats().total_chunks;
        store.delete("a.txt", OWNER).unwrap();

        // Chunks should still exist because b.txt references them.
        assert_eq!(store.stats().total_chunks, chunks_before);
        assert_eq!(store.get_latest("b.txt", OWNER).unwrap(), data);
    }

    #[test]
    fn delete_nonexistent_file() {
        let mut store = FileStore::new();
        assert!(store.delete("nope.txt", OWNER).is_err());
    }

    #[test]
    fn get_nonexistent_manifest() {
        let store = FileStore::new();
        assert!(store.get("nonexistent-hash").is_err());
    }

    #[test]
    fn import_chunk() {
        let mut store = FileStore::new();
        let data = b"chunk for import";
        let hash = ContentId::from_bytes(data).0;

        store.import_chunk(hash.clone(), data.to_vec()).unwrap();
        assert!(store.has_chunk(&hash));
        assert_eq!(store.get_chunk(&hash).unwrap(), data);
    }

    #[test]
    fn import_chunk_rejects_bad_hash() {
        let mut store = FileStore::new();
        assert!(store.import_chunk("wrong-hash".into(), b"data".to_vec()).is_err());
    }

    #[test]
    fn import_chunk_deduplicates() {
        let mut store = FileStore::new();
        let data = b"dedup import";
        let hash = ContentId::from_bytes(data).0;

        store.import_chunk(hash.clone(), data.to_vec()).unwrap();
        let bytes_after_first = store.stats().stored_bytes;

        store.import_chunk(hash, data.to_vec()).unwrap();
        assert_eq!(store.stats().stored_bytes, bytes_after_first);
    }

    #[test]
    fn empty_file() {
        let mut store = FileStore::new();
        let manifest = store.put("empty.bin", "application/octet-stream", b"", OWNER).unwrap();
        assert_eq!(manifest.total_size, 0);

        let retrieved = store.get(&manifest.id.0).unwrap();
        assert!(retrieved.is_empty());
    }

    #[test]
    fn store_stats() {
        let mut store = FileStore::new();
        store.put("a.txt", "text/plain", b"hello", OWNER).unwrap();
        store.put("b.txt", "text/plain", b"world", OWNER).unwrap();

        let stats = store.stats();
        assert_eq!(stats.total_files, 2);
        assert!(stats.stored_bytes > 0);
        assert!(stats.logical_bytes > 0);
    }

    #[test]
    fn store_serde_roundtrip() {
        let mut store = FileStore::new();
        store.put("test.txt", "text/plain", b"serialize me", OWNER).unwrap();

        let json = serde_json::to_string(&store).unwrap();
        let deserialized: FileStore = serde_json::from_str(&json).unwrap();

        let data = deserialized.get_latest("test.txt", OWNER).unwrap();
        assert_eq!(data, b"serialize me");
    }
}
