//! Filesystem-backed content-addressed file store.
//!
//! Stores files by SHA-256 hash on disk at a configurable root directory
//! (defaults to `~/.nous/files/`). Chunk data lives in `{root}/chunks/{hash}`
//! and manifests in `{root}/manifests/{id}.json`. A metadata index tracks
//! stored files with size, mime type, and timestamp.

use std::fs;
use std::path::PathBuf;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use nous_core::{Error, Result};

use crate::chunk;
use crate::manifest::{ChunkRef, FileManifest, VersionHistory};

/// Metadata about a stored file.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileMetadata {
    /// Original filename.
    pub name: String,
    /// MIME type.
    pub mime_type: String,
    /// Total file size in bytes.
    pub size: u64,
    /// Owner identifier.
    pub owner: String,
    /// Current manifest ID.
    pub manifest_id: String,
    /// Number of versions.
    pub version_count: u32,
    /// When the file was first stored.
    pub created_at: DateTime<Utc>,
    /// When the file was last updated.
    pub updated_at: DateTime<Utc>,
}

/// Statistics about the disk file store.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiskStoreStats {
    /// Number of chunk files on disk.
    pub total_chunks: usize,
    /// Number of tracked files (latest versions).
    pub total_files: usize,
    /// Bytes used by chunks on disk.
    pub stored_bytes: u64,
    /// Sum of all files' logical sizes.
    pub logical_bytes: u64,
    /// Deduplication ratio (logical / stored). Greater than 1.0 means savings.
    pub dedup_ratio: f64,
}

/// A content-addressed file store backed by the local filesystem.
///
/// Chunks are stored as individual files under `{root}/chunks/{sha256_hex}`.
/// Manifests and version histories are serialized as JSON under
/// `{root}/manifests/`. A metadata index at `{root}/index.json` tracks
/// all stored files for fast listing.
///
/// # Layout
/// ```text
/// {root}/
///   chunks/
///     {sha256_hex}          # raw chunk bytes
///   manifests/
///     {owner}:{name}.json   # VersionHistory as JSON
///   index.json              # Vec<FileMetadata>
/// ```
pub struct DiskFileStore {
    root: PathBuf,
    /// In-memory metadata index, flushed to disk on mutation.
    index: Vec<FileMetadata>,
}

impl DiskFileStore {
    /// Open or create a disk file store at the given root directory.
    pub fn open(root: impl Into<PathBuf>) -> Result<Self> {
        let root = root.into();
        let chunks_dir = root.join("chunks");
        let manifests_dir = root.join("manifests");

        fs::create_dir_all(&chunks_dir)
            .map_err(|e| Error::Storage(format!("failed to create chunks dir: {e}")))?;
        fs::create_dir_all(&manifests_dir)
            .map_err(|e| Error::Storage(format!("failed to create manifests dir: {e}")))?;

        let index_path = root.join("index.json");
        let index = if index_path.exists() {
            let data = fs::read_to_string(&index_path)
                .map_err(|e| Error::Storage(format!("failed to read index: {e}")))?;
            serde_json::from_str(&data)
                .map_err(|e| Error::Storage(format!("failed to parse index: {e}")))?
        } else {
            Vec::new()
        };

        Ok(Self { root, index })
    }

    /// Open a disk file store at the default location (`~/.nous/files/`).
    pub fn open_default() -> Result<Self> {
        let home = std::env::var("HOME")
            .map_err(|_| Error::Storage("HOME environment variable not set".into()))?;
        Self::open(PathBuf::from(home).join(".nous").join("files"))
    }

    /// Store a file. Splits it into chunks, deduplicates, and writes to disk.
    pub fn put(
        &mut self,
        name: &str,
        mime_type: &str,
        data: &[u8],
        owner: &str,
    ) -> Result<FileManifest> {
        let chunks = chunk::chunk_data(data)?;
        let chunk_refs: Vec<ChunkRef> = chunks.iter().map(ChunkRef::from).collect();

        // Write chunks to disk, deduplicating.
        for c in &chunks {
            let chunk_path = self.chunk_path(&c.hash);
            if !chunk_path.exists() {
                fs::write(&chunk_path, &c.data)
                    .map_err(|e| Error::Storage(format!("failed to write chunk: {e}")))?;
            }
        }

        let history_key = format!("{owner}:{name}");
        let manifest_path = self.manifest_path(&history_key);

        let manifest = if manifest_path.exists() {
            let history = self.load_history(&history_key)?;
            history.current.new_version(chunk_refs)
        } else {
            FileManifest::new(name, mime_type, chunk_refs, owner)
        };

        // Update version history.
        let mut history = if manifest_path.exists() {
            self.load_history(&history_key)?
        } else {
            VersionHistory::new(manifest.clone())
        };
        if manifest_path.exists() {
            history.push(manifest.clone());
        }
        self.save_history(&history_key, &history)?;

        // Update metadata index.
        let now = Utc::now();
        if let Some(meta) = self
            .index
            .iter_mut()
            .find(|m| m.owner == owner && m.name == name)
        {
            meta.manifest_id = manifest.id.0.clone();
            meta.size = manifest.total_size;
            meta.version_count = manifest.version;
            meta.updated_at = now;
        } else {
            self.index.push(FileMetadata {
                name: name.to_string(),
                mime_type: mime_type.to_string(),
                size: manifest.total_size,
                owner: owner.to_string(),
                manifest_id: manifest.id.0.clone(),
                version_count: manifest.version,
                created_at: now,
                updated_at: now,
            });
        }
        self.flush_index()?;

        Ok(manifest)
    }

    /// Retrieve a file by reassembling its chunks from a manifest ID.
    pub fn get(&self, name: &str, owner: &str) -> Result<Vec<u8>> {
        let history_key = format!("{owner}:{name}");
        let history = self.load_history(&history_key)?;
        self.assemble_manifest(&history.current)
    }

    /// Retrieve a specific version of a file.
    pub fn get_version(&self, name: &str, owner: &str, version: u32) -> Result<Vec<u8>> {
        let history_key = format!("{owner}:{name}");
        let history = self.load_history(&history_key)?;
        let manifest = history
            .get_version(version)
            .ok_or_else(|| Error::NotFound(format!("version {version} not found")))?;
        self.assemble_manifest(manifest)
    }

    /// List all files with their metadata.
    pub fn list(&self) -> &[FileMetadata] {
        &self.index
    }

    /// List files for a specific owner.
    pub fn list_for_owner(&self, owner: &str) -> Vec<&FileMetadata> {
        self.index.iter().filter(|m| m.owner == owner).collect()
    }

    /// Delete a file and all its versions. Orphaned chunks are removed.
    pub fn delete(&mut self, name: &str, owner: &str) -> Result<u64> {
        let history_key = format!("{owner}:{name}");
        let manifest_path = self.manifest_path(&history_key);
        if !manifest_path.exists() {
            return Err(Error::NotFound(format!("file '{name}' not found")));
        }

        let history = self.load_history(&history_key)?;

        // Collect all chunk hashes from all versions of this file.
        let mut file_chunk_hashes: std::collections::HashSet<String> =
            std::collections::HashSet::new();
        for chunk_ref in &history.current.chunks {
            file_chunk_hashes.insert(chunk_ref.hash.clone());
        }
        for version in &history.history {
            for chunk_ref in &version.chunks {
                file_chunk_hashes.insert(chunk_ref.hash.clone());
            }
        }

        // Remove manifest file.
        fs::remove_file(&manifest_path)
            .map_err(|e| Error::Storage(format!("failed to remove manifest: {e}")))?;

        // Collect chunk hashes still referenced by other files.
        let referenced = self.all_referenced_chunks_except(&history_key);

        // Remove orphaned chunks.
        let mut freed = 0u64;
        for hash in &file_chunk_hashes {
            if !referenced.contains(hash) {
                let chunk_path = self.chunk_path(hash);
                if let Ok(meta) = fs::metadata(&chunk_path) {
                    freed += meta.len();
                    let _ = fs::remove_file(&chunk_path);
                }
            }
        }

        // Update metadata index.
        self.index.retain(|m| !(m.owner == owner && m.name == name));
        self.flush_index()?;

        Ok(freed)
    }

    /// Check if a chunk exists on disk.
    pub fn has_chunk(&self, hash: &str) -> bool {
        self.chunk_path(hash).exists()
    }

    /// Get store statistics.
    pub fn stats(&self) -> Result<DiskStoreStats> {
        let chunks_dir = self.root.join("chunks");
        let mut total_chunks = 0usize;
        let mut stored_bytes = 0u64;

        if chunks_dir.exists() {
            for entry in fs::read_dir(&chunks_dir)
                .map_err(|e| Error::Storage(format!("failed to read chunks dir: {e}")))?
            {
                let entry =
                    entry.map_err(|e| Error::Storage(format!("failed to read entry: {e}")))?;
                if entry.path().is_file() {
                    total_chunks += 1;
                    if let Ok(meta) = entry.metadata() {
                        stored_bytes += meta.len();
                    }
                }
            }
        }

        let logical_bytes: u64 = self.index.iter().map(|m| m.size).sum();
        let dedup_ratio = if stored_bytes > 0 {
            logical_bytes as f64 / stored_bytes as f64
        } else {
            1.0
        };

        Ok(DiskStoreStats {
            total_chunks,
            total_files: self.index.len(),
            stored_bytes,
            logical_bytes,
            dedup_ratio,
        })
    }

    /// Get the version history for a file.
    pub fn get_history(&self, name: &str, owner: &str) -> Result<VersionHistory> {
        let history_key = format!("{owner}:{name}");
        self.load_history(&history_key)
    }

    // ── Internal helpers ─────────────────────────────────────────────

    fn chunk_path(&self, hash: &str) -> PathBuf {
        self.root.join("chunks").join(hash)
    }

    fn manifest_path(&self, history_key: &str) -> PathBuf {
        // Replace colons and slashes to make a safe filename.
        let safe_key = history_key.replace([':', '/'], "_");
        self.root.join("manifests").join(format!("{safe_key}.json"))
    }

    fn load_history(&self, history_key: &str) -> Result<VersionHistory> {
        let path = self.manifest_path(history_key);
        let data = fs::read_to_string(&path)
            .map_err(|e| Error::NotFound(format!("manifest not found: {e}")))?;
        serde_json::from_str(&data)
            .map_err(|e| Error::Storage(format!("failed to parse manifest: {e}")))
    }

    fn save_history(&self, history_key: &str, history: &VersionHistory) -> Result<()> {
        let path = self.manifest_path(history_key);
        let data = serde_json::to_string_pretty(history)
            .map_err(|e| Error::Storage(format!("failed to serialize manifest: {e}")))?;
        fs::write(&path, data).map_err(|e| Error::Storage(format!("failed to write manifest: {e}")))
    }

    fn flush_index(&self) -> Result<()> {
        let path = self.root.join("index.json");
        let data = serde_json::to_string_pretty(&self.index)
            .map_err(|e| Error::Storage(format!("failed to serialize index: {e}")))?;
        fs::write(&path, data).map_err(|e| Error::Storage(format!("failed to write index: {e}")))
    }

    fn assemble_manifest(&self, manifest: &FileManifest) -> Result<Vec<u8>> {
        let mut data = Vec::with_capacity(manifest.total_size as usize);
        for chunk_ref in &manifest.chunks {
            let chunk_path = self.chunk_path(&chunk_ref.hash);
            let chunk_data = fs::read(&chunk_path).map_err(|e| {
                Error::NotFound(format!("chunk {} missing from disk: {e}", chunk_ref.hash))
            })?;
            data.extend_from_slice(&chunk_data);
        }
        Ok(data)
    }

    /// Collect all chunk hashes referenced by files other than the given key.
    fn all_referenced_chunks_except(&self, exclude_key: &str) -> std::collections::HashSet<String> {
        let manifests_dir = self.root.join("manifests");
        let mut referenced = std::collections::HashSet::new();
        let exclude_path = self.manifest_path(exclude_key);

        if let Ok(entries) = fs::read_dir(&manifests_dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path == exclude_path || !path.is_file() {
                    continue;
                }
                if let Ok(data) = fs::read_to_string(&path)
                    && let Ok(history) = serde_json::from_str::<VersionHistory>(&data)
                {
                    for chunk_ref in &history.current.chunks {
                        referenced.insert(chunk_ref.hash.clone());
                    }
                    for version in &history.history {
                        for chunk_ref in &version.chunks {
                            referenced.insert(chunk_ref.hash.clone());
                        }
                    }
                }
            }
        }

        referenced
    }
}

impl std::fmt::Debug for DiskFileStore {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("DiskFileStore")
            .field("root", &self.root)
            .field("files", &self.index.len())
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_store() -> (tempfile::TempDir, DiskFileStore) {
        let dir = tempfile::tempdir().unwrap();
        let store = DiskFileStore::open(dir.path().join("files")).unwrap();
        (dir, store)
    }

    const OWNER: &str = "did:key:zTestOwner";

    #[test]
    fn open_creates_directories() {
        let (_dir, store) = test_store();
        assert!(store.root.join("chunks").exists());
        assert!(store.root.join("manifests").exists());
    }

    #[test]
    fn put_and_get_small_file() {
        let (_dir, mut store) = test_store();
        let data = b"hello world from disk store";
        store.put("test.txt", "text/plain", data, OWNER).unwrap();

        let retrieved = store.get("test.txt", OWNER).unwrap();
        assert_eq!(retrieved, data);
    }

    #[test]
    fn put_and_get_large_file() {
        let (_dir, mut store) = test_store();
        let data = vec![0xABu8; 512 * 1024]; // 512 KiB
        store
            .put("big.bin", "application/octet-stream", &data, OWNER)
            .unwrap();

        let retrieved = store.get("big.bin", OWNER).unwrap();
        assert_eq!(retrieved, data);
    }

    #[test]
    fn put_and_get_empty_file() {
        let (_dir, mut store) = test_store();
        store
            .put("empty.bin", "application/octet-stream", b"", OWNER)
            .unwrap();

        let retrieved = store.get("empty.bin", OWNER).unwrap();
        assert!(retrieved.is_empty());
    }

    #[test]
    fn versioning() {
        let (_dir, mut store) = test_store();
        store
            .put("doc.txt", "text/plain", b"version 1", OWNER)
            .unwrap();
        store
            .put("doc.txt", "text/plain", b"version 2", OWNER)
            .unwrap();

        let latest = store.get("doc.txt", OWNER).unwrap();
        assert_eq!(latest, b"version 2");

        let v1 = store.get_version("doc.txt", OWNER, 1).unwrap();
        assert_eq!(v1, b"version 1");
    }

    #[test]
    fn list_files() {
        let (_dir, mut store) = test_store();
        store.put("a.txt", "text/plain", b"aaa", OWNER).unwrap();
        store.put("b.txt", "text/plain", b"bbb", OWNER).unwrap();
        store
            .put("c.txt", "text/plain", b"ccc", "did:key:zOther")
            .unwrap();

        assert_eq!(store.list().len(), 3);
        assert_eq!(store.list_for_owner(OWNER).len(), 2);
    }

    #[test]
    fn list_metadata_correct() {
        let (_dir, mut store) = test_store();
        store
            .put("readme.md", "text/markdown", b"# Hello", OWNER)
            .unwrap();

        let meta = &store.list()[0];
        assert_eq!(meta.name, "readme.md");
        assert_eq!(meta.mime_type, "text/markdown");
        assert_eq!(meta.size, 7);
        assert_eq!(meta.owner, OWNER);
        assert_eq!(meta.version_count, 1);
    }

    #[test]
    fn delete_file() {
        let (_dir, mut store) = test_store();
        store
            .put("temp.txt", "text/plain", b"temporary", OWNER)
            .unwrap();
        assert_eq!(store.list().len(), 1);

        let freed = store.delete("temp.txt", OWNER).unwrap();
        assert!(freed > 0);
        assert!(store.list().is_empty());
        assert!(store.get("temp.txt", OWNER).is_err());
    }

    #[test]
    fn delete_preserves_shared_chunks() {
        let (_dir, mut store) = test_store();
        let data = b"shared chunk data between files";
        store.put("a.txt", "text/plain", data, OWNER).unwrap();
        store.put("b.txt", "text/plain", data, OWNER).unwrap();

        let stats_before = store.stats().unwrap();
        store.delete("a.txt", OWNER).unwrap();
        let stats_after = store.stats().unwrap();

        // Chunks should still exist because b.txt references them.
        assert_eq!(stats_before.total_chunks, stats_after.total_chunks);
        assert_eq!(store.get("b.txt", OWNER).unwrap(), data);
    }

    #[test]
    fn delete_nonexistent() {
        let (_dir, mut store) = test_store();
        assert!(store.delete("nope.txt", OWNER).is_err());
    }

    #[test]
    fn deduplication() {
        let (_dir, mut store) = test_store();
        let data = b"identical content across files";

        store.put("file1.txt", "text/plain", data, OWNER).unwrap();
        let stats1 = store.stats().unwrap();

        store.put("file2.txt", "text/plain", data, OWNER).unwrap();
        let stats2 = store.stats().unwrap();

        // Same chunks should not increase stored bytes.
        assert_eq!(stats1.stored_bytes, stats2.stored_bytes);
        assert_eq!(stats2.total_files, 2);
    }

    #[test]
    fn has_chunk() {
        let (_dir, mut store) = test_store();
        store
            .put("test.txt", "text/plain", b"chunk check", OWNER)
            .unwrap();

        let hash = crate::chunk::ContentId::from_bytes(b"chunk check").0;
        assert!(store.has_chunk(&hash));
        assert!(!store.has_chunk("nonexistent_hash"));
    }

    #[test]
    fn stats() {
        let (_dir, mut store) = test_store();
        let stats = store.stats().unwrap();
        assert_eq!(stats.total_files, 0);
        assert_eq!(stats.total_chunks, 0);

        store.put("a.txt", "text/plain", b"hello", OWNER).unwrap();
        let stats = store.stats().unwrap();
        assert_eq!(stats.total_files, 1);
        assert!(stats.total_chunks > 0);
        assert!(stats.stored_bytes > 0);
    }

    #[test]
    fn persistence_across_opens() {
        let dir = tempfile::tempdir().unwrap();
        let store_path = dir.path().join("files");

        {
            let mut store = DiskFileStore::open(&store_path).unwrap();
            store
                .put("persist.txt", "text/plain", b"survives reopen", OWNER)
                .unwrap();
        }

        {
            let store = DiskFileStore::open(&store_path).unwrap();
            assert_eq!(store.list().len(), 1);
            let data = store.get("persist.txt", OWNER).unwrap();
            assert_eq!(data, b"survives reopen");
        }
    }

    #[test]
    fn version_history() {
        let (_dir, mut store) = test_store();
        store.put("doc.txt", "text/plain", b"v1", OWNER).unwrap();
        store.put("doc.txt", "text/plain", b"v2", OWNER).unwrap();
        store.put("doc.txt", "text/plain", b"v3", OWNER).unwrap();

        let history = store.get_history("doc.txt", OWNER).unwrap();
        assert_eq!(history.version_count(), 3);
        assert_eq!(history.current.version, 3);
    }

    #[test]
    fn metadata_updates_on_new_version() {
        let (_dir, mut store) = test_store();
        store.put("doc.txt", "text/plain", b"short", OWNER).unwrap();
        store
            .put("doc.txt", "text/plain", b"longer content here", OWNER)
            .unwrap();

        let meta = store.list().iter().find(|m| m.name == "doc.txt").unwrap();
        assert_eq!(meta.version_count, 2);
        assert_eq!(meta.size, b"longer content here".len() as u64);
    }

    #[test]
    fn get_nonexistent_file() {
        let (_dir, store) = test_store();
        assert!(store.get("missing.txt", OWNER).is_err());
    }

    #[test]
    fn get_nonexistent_version() {
        let (_dir, mut store) = test_store();
        store.put("doc.txt", "text/plain", b"v1", OWNER).unwrap();
        assert!(store.get_version("doc.txt", OWNER, 99).is_err());
    }

    #[test]
    fn debug_format() {
        let (_dir, store) = test_store();
        let debug = format!("{store:?}");
        assert!(debug.contains("DiskFileStore"));
    }
}
