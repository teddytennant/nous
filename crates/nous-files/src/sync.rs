//! Merkle-tree-based file synchronization.
//!
//! Uses [`nous_storage::MerkleTree`] to efficiently detect which files
//! differ between two peers without transferring full manifests. The sync
//! protocol works in three phases:
//!
//! 1. **Exchange roots** — each peer computes a Merkle root over its file
//!    manifests and sends it to the other.
//! 2. **Diff if needed** — if roots differ, exchange full trees and compute
//!    the diff to find which files diverge.
//! 3. **Transfer** — send/request only the differing chunks.

use std::collections::HashMap;

use nous_storage::merkle::{MerkleHash, MerkleProof, MerkleTree};
use serde::{Deserialize, Serialize};

use crate::manifest::{ChunkRef, FileManifest};

/// A snapshot of a peer's file state, suitable for sync comparison.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncSnapshot {
    /// The Merkle root hash over all file manifests.
    pub root: MerkleHash,
    /// Ordered list of file IDs that were used to build the tree.
    pub file_ids: Vec<String>,
    /// Number of files in this snapshot.
    pub file_count: usize,
}

/// The result of comparing two sync snapshots.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncDiff {
    /// File IDs that exist locally but differ or are missing on the remote.
    pub local_changed: Vec<String>,
    /// File IDs that exist on the remote but differ or are missing locally.
    pub remote_changed: Vec<String>,
    /// Files that are identical on both sides.
    pub in_sync: usize,
}

impl SyncDiff {
    /// Total number of files that need synchronization.
    pub fn changes(&self) -> usize {
        self.local_changed.len() + self.remote_changed.len()
    }

    /// Whether the two snapshots are fully in sync.
    pub fn is_synced(&self) -> bool {
        self.local_changed.is_empty() && self.remote_changed.is_empty()
    }
}

/// A plan describing what needs to be transferred for sync.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncPlan {
    /// Chunks to send to the remote peer.
    pub chunks_to_send: Vec<String>,
    /// Chunks to request from the remote peer.
    pub chunks_to_request: Vec<String>,
    /// Files to send (full manifests).
    pub files_to_send: Vec<String>,
    /// Files to request (full manifests).
    pub files_to_request: Vec<String>,
}

impl SyncPlan {
    /// Total transfer items.
    pub fn total_items(&self) -> usize {
        self.chunks_to_send.len()
            + self.chunks_to_request.len()
            + self.files_to_send.len()
            + self.files_to_request.len()
    }

    /// Whether there's anything to sync.
    pub fn is_empty(&self) -> bool {
        self.total_items() == 0
    }
}

/// Builds Merkle trees from file manifests for efficient sync comparison.
pub struct SyncEngine {
    /// File manifests indexed by file ID.
    manifests: HashMap<String, FileManifest>,
    /// Sorted file IDs (consistent ordering for deterministic tree).
    file_ids: Vec<String>,
}

impl SyncEngine {
    pub fn new() -> Self {
        Self {
            manifests: HashMap::new(),
            file_ids: Vec::new(),
        }
    }

    /// Add or update a file manifest.
    pub fn upsert(&mut self, file_id: impl Into<String>, manifest: FileManifest) {
        let file_id = file_id.into();
        self.manifests.insert(file_id.clone(), manifest);
        if !self.file_ids.contains(&file_id) {
            self.file_ids.push(file_id);
            self.file_ids.sort();
        }
    }

    /// Remove a file.
    pub fn remove(&mut self, file_id: &str) -> bool {
        if self.manifests.remove(file_id).is_some() {
            self.file_ids.retain(|id| id != file_id);
            true
        } else {
            false
        }
    }

    /// Number of tracked files.
    pub fn file_count(&self) -> usize {
        self.manifests.len()
    }

    /// Build a Merkle tree over all file manifests.
    /// Each leaf is the SHA-256 of the manifest's content ID.
    pub fn build_tree(&self) -> MerkleTree {
        if self.file_ids.is_empty() {
            return MerkleTree::from_leaves(&[]);
        }

        let leaves: Vec<Vec<u8>> = self
            .file_ids
            .iter()
            .map(|id| {
                self.manifests
                    .get(id)
                    .map(|m| m.id.0.as_bytes().to_vec())
                    .unwrap_or_default()
            })
            .collect();

        let refs: Vec<&[u8]> = leaves.iter().map(|l| l.as_slice()).collect();
        MerkleTree::from_leaves(&refs)
    }

    /// Create a snapshot of the current state for exchange with a peer.
    pub fn snapshot(&self) -> SyncSnapshot {
        let tree = self.build_tree();
        SyncSnapshot {
            root: tree.root().clone(),
            file_ids: self.file_ids.clone(),
            file_count: self.file_ids.len(),
        }
    }

    /// Generate an inclusion proof for a specific file.
    pub fn proof_for(&self, file_id: &str) -> Option<MerkleProof> {
        let index = self.file_ids.iter().position(|id| id == file_id)?;
        let tree = self.build_tree();
        tree.proof(index)
    }

    /// Compare local state against a remote snapshot.
    /// Returns a diff describing which files changed on each side.
    pub fn diff(&self, remote: &SyncSnapshot) -> SyncDiff {
        let local_tree = self.build_tree();
        let local_root = local_tree.root().clone();

        if local_root == remote.root {
            return SyncDiff {
                local_changed: Vec::new(),
                remote_changed: Vec::new(),
                in_sync: self.file_ids.len(),
            };
        }

        // Find files that are on one side but not the other.
        let local_set: std::collections::HashSet<&str> =
            self.file_ids.iter().map(|s| s.as_str()).collect();
        let remote_set: std::collections::HashSet<&str> =
            remote.file_ids.iter().map(|s| s.as_str()).collect();

        let mut local_changed: Vec<String> = Vec::new();
        let mut remote_changed: Vec<String> = Vec::new();
        let mut in_sync = 0;

        // Files only on local side.
        for id in &self.file_ids {
            if !remote_set.contains(id.as_str()) {
                local_changed.push(id.clone());
            }
        }

        // Files only on remote side.
        for id in &remote.file_ids {
            if !local_set.contains(id.as_str()) {
                remote_changed.push(id.clone());
            }
        }

        // Files on both sides — need tree comparison.
        // Build a tree of just the shared files to compare.
        let shared: Vec<&str> = self
            .file_ids
            .iter()
            .filter(|id| remote_set.contains(id.as_str()))
            .map(|s| s.as_str())
            .collect();

        // For shared files, we can't do a tree diff directly since the trees
        // may have different file sets. Instead compare manifest IDs.
        // The remote doesn't send us manifests in the snapshot — just file IDs.
        // So we mark shared files as potentially changed if the roots differ.
        // The actual resolution happens when manifests are exchanged.
        if !shared.is_empty() && local_root != remote.root {
            // All shared files are potentially changed.
            // In practice, the peer would send manifest IDs for comparison.
            for id in &shared {
                // We can't tell which ones changed without more data,
                // so mark them all as needing verification.
                local_changed.push(id.to_string());
            }
        } else {
            in_sync = shared.len();
        }

        SyncDiff {
            local_changed,
            remote_changed,
            in_sync,
        }
    }

    /// Build a sync plan from two sets of manifests.
    /// `local` is our manifests, `remote` is the peer's manifests for
    /// the files identified in the diff.
    pub fn plan(
        local: &HashMap<String, FileManifest>,
        remote: &HashMap<String, FileManifest>,
    ) -> SyncPlan {
        let mut chunks_to_send = Vec::new();
        let mut chunks_to_request = Vec::new();
        let mut files_to_send = Vec::new();
        let mut files_to_request = Vec::new();

        // Files we have but remote doesn't.
        for (id, manifest) in local {
            if !remote.contains_key(id) {
                files_to_send.push(id.clone());
                for chunk in &manifest.chunks {
                    chunks_to_send.push(chunk.hash.clone());
                }
            }
        }

        // Files remote has but we don't.
        for (id, manifest) in remote {
            if !local.contains_key(id) {
                files_to_request.push(id.clone());
                for chunk in &manifest.chunks {
                    chunks_to_request.push(chunk.hash.clone());
                }
            }
        }

        // Files both have but with different content.
        for (id, local_manifest) in local {
            if let Some(remote_manifest) = remote.get(id) {
                if local_manifest.id != remote_manifest.id {
                    // Use manifest diff to find exactly which chunks differ.
                    let (added, removed) = local_manifest.diff(remote_manifest);
                    chunks_to_request.extend(added);
                    chunks_to_send.extend(removed);

                    // If remote has newer version, request the file.
                    if remote_manifest.version > local_manifest.version {
                        files_to_request.push(id.clone());
                    } else if local_manifest.version > remote_manifest.version {
                        files_to_send.push(id.clone());
                    }
                }
            }
        }

        SyncPlan {
            chunks_to_send,
            chunks_to_request,
            files_to_send,
            files_to_request,
        }
    }
}

impl Default for SyncEngine {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_manifest(name: &str, chunks: Vec<ChunkRef>) -> FileManifest {
        FileManifest::new(name, "application/octet-stream", chunks, "did:key:zTest")
    }

    fn chunk_ref(hash: &str, size: u64) -> ChunkRef {
        ChunkRef {
            hash: hash.to_string(),
            offset: 0,
            size,
        }
    }

    #[test]
    fn empty_engine() {
        let engine = SyncEngine::new();
        assert_eq!(engine.file_count(), 0);
        let snapshot = engine.snapshot();
        assert_eq!(snapshot.file_count, 0);
    }

    #[test]
    fn upsert_file() {
        let mut engine = SyncEngine::new();
        let manifest = make_manifest("test.txt", vec![chunk_ref("aaa", 100)]);
        engine.upsert("file-1", manifest);
        assert_eq!(engine.file_count(), 1);
    }

    #[test]
    fn remove_file() {
        let mut engine = SyncEngine::new();
        engine.upsert("file-1", make_manifest("a.txt", vec![chunk_ref("aaa", 100)]));
        assert!(engine.remove("file-1"));
        assert_eq!(engine.file_count(), 0);
        assert!(!engine.remove("file-1"));
    }

    #[test]
    fn snapshot_deterministic() {
        let mut engine = SyncEngine::new();
        engine.upsert("f1", make_manifest("a.txt", vec![chunk_ref("aaa", 100)]));
        engine.upsert("f2", make_manifest("b.txt", vec![chunk_ref("bbb", 200)]));

        let s1 = engine.snapshot();
        let s2 = engine.snapshot();
        assert_eq!(s1.root, s2.root);
    }

    #[test]
    fn snapshot_changes_with_data() {
        let mut engine = SyncEngine::new();
        engine.upsert("f1", make_manifest("a.txt", vec![chunk_ref("aaa", 100)]));
        let s1 = engine.snapshot();

        engine.upsert("f1", make_manifest("a.txt", vec![chunk_ref("bbb", 200)]));
        let s2 = engine.snapshot();

        assert_ne!(s1.root, s2.root);
    }

    #[test]
    fn diff_identical() {
        let mut engine = SyncEngine::new();
        engine.upsert("f1", make_manifest("a.txt", vec![chunk_ref("aaa", 100)]));

        let snapshot = engine.snapshot();
        let diff = engine.diff(&snapshot);
        assert!(diff.is_synced());
        assert_eq!(diff.in_sync, 1);
    }

    #[test]
    fn diff_local_only_file() {
        let mut local = SyncEngine::new();
        local.upsert("f1", make_manifest("a.txt", vec![chunk_ref("aaa", 100)]));
        local.upsert("f2", make_manifest("b.txt", vec![chunk_ref("bbb", 200)]));

        let mut remote = SyncEngine::new();
        remote.upsert("f1", make_manifest("a.txt", vec![chunk_ref("aaa", 100)]));

        let remote_snapshot = remote.snapshot();
        let diff = local.diff(&remote_snapshot);

        assert!(!diff.is_synced());
        assert!(diff.local_changed.contains(&"f2".to_string()));
    }

    #[test]
    fn diff_remote_only_file() {
        let mut local = SyncEngine::new();
        local.upsert("f1", make_manifest("a.txt", vec![chunk_ref("aaa", 100)]));

        let mut remote = SyncEngine::new();
        remote.upsert("f1", make_manifest("a.txt", vec![chunk_ref("aaa", 100)]));
        remote.upsert("f3", make_manifest("c.txt", vec![chunk_ref("ccc", 300)]));

        let remote_snapshot = remote.snapshot();
        let diff = local.diff(&remote_snapshot);

        assert!(!diff.is_synced());
        assert!(diff.remote_changed.contains(&"f3".to_string()));
    }

    #[test]
    fn proof_for_file() {
        let mut engine = SyncEngine::new();
        engine.upsert("f1", make_manifest("a.txt", vec![chunk_ref("aaa", 100)]));
        engine.upsert("f2", make_manifest("b.txt", vec![chunk_ref("bbb", 200)]));

        let proof = engine.proof_for("f1").unwrap();
        assert!(proof.verify());
    }

    #[test]
    fn proof_for_nonexistent() {
        let engine = SyncEngine::new();
        assert!(engine.proof_for("ghost").is_none());
    }

    #[test]
    fn plan_local_only() {
        let mut local = HashMap::new();
        local.insert(
            "f1".to_string(),
            make_manifest("a.txt", vec![chunk_ref("aaa", 100), chunk_ref("bbb", 200)]),
        );

        let remote = HashMap::new();
        let plan = SyncEngine::plan(&local, &remote);

        assert_eq!(plan.files_to_send.len(), 1);
        assert_eq!(plan.chunks_to_send.len(), 2);
        assert!(plan.files_to_request.is_empty());
        assert!(plan.chunks_to_request.is_empty());
    }

    #[test]
    fn plan_remote_only() {
        let local = HashMap::new();
        let mut remote = HashMap::new();
        remote.insert(
            "f1".to_string(),
            make_manifest("a.txt", vec![chunk_ref("aaa", 100)]),
        );

        let plan = SyncEngine::plan(&local, &remote);

        assert_eq!(plan.files_to_request.len(), 1);
        assert_eq!(plan.chunks_to_request.len(), 1);
        assert!(plan.files_to_send.is_empty());
    }

    #[test]
    fn plan_diverged_file() {
        let mut local = HashMap::new();
        let v1 = make_manifest("a.txt", vec![chunk_ref("aaa", 100), chunk_ref("bbb", 200)]);
        local.insert("f1".to_string(), v1.clone());

        let mut remote = HashMap::new();
        let v2 = v1.new_version(vec![chunk_ref("aaa", 100), chunk_ref("ccc", 150)]);
        remote.insert("f1".to_string(), v2);

        let plan = SyncEngine::plan(&local, &remote);

        // bbb was removed (local has it, remote doesn't), ccc was added.
        assert!(plan.chunks_to_send.contains(&"bbb".to_string()));
        assert!(plan.chunks_to_request.contains(&"ccc".to_string()));
        assert!(plan.files_to_request.contains(&"f1".to_string()));
    }

    #[test]
    fn plan_empty() {
        let plan = SyncEngine::plan(&HashMap::new(), &HashMap::new());
        assert!(plan.is_empty());
    }

    #[test]
    fn sync_snapshot_serializes() {
        let mut engine = SyncEngine::new();
        engine.upsert("f1", make_manifest("a.txt", vec![chunk_ref("aaa", 100)]));
        let snapshot = engine.snapshot();

        let json = serde_json::to_string(&snapshot).unwrap();
        let restored: SyncSnapshot = serde_json::from_str(&json).unwrap();
        assert_eq!(restored.root, snapshot.root);
        assert_eq!(restored.file_count, 1);
    }

    #[test]
    fn sync_diff_serializes() {
        let diff = SyncDiff {
            local_changed: vec!["f1".into()],
            remote_changed: vec!["f2".into()],
            in_sync: 3,
        };
        let json = serde_json::to_string(&diff).unwrap();
        let restored: SyncDiff = serde_json::from_str(&json).unwrap();
        assert_eq!(restored.changes(), 2);
    }

    #[test]
    fn sync_plan_serializes() {
        let plan = SyncPlan {
            chunks_to_send: vec!["aaa".into()],
            chunks_to_request: vec!["bbb".into()],
            files_to_send: vec!["f1".into()],
            files_to_request: vec!["f2".into()],
        };
        let json = serde_json::to_string(&plan).unwrap();
        let restored: SyncPlan = serde_json::from_str(&json).unwrap();
        assert_eq!(restored.total_items(), 4);
    }

    #[test]
    fn upsert_updates_existing() {
        let mut engine = SyncEngine::new();
        engine.upsert("f1", make_manifest("a.txt", vec![chunk_ref("aaa", 100)]));
        let s1 = engine.snapshot();

        engine.upsert("f1", make_manifest("a.txt", vec![chunk_ref("bbb", 200)]));
        let s2 = engine.snapshot();

        assert_ne!(s1.root, s2.root);
        assert_eq!(engine.file_count(), 1);
    }

    #[test]
    fn file_ids_sorted() {
        let mut engine = SyncEngine::new();
        engine.upsert("z-file", make_manifest("z.txt", vec![chunk_ref("zzz", 100)]));
        engine.upsert("a-file", make_manifest("a.txt", vec![chunk_ref("aaa", 100)]));
        engine.upsert("m-file", make_manifest("m.txt", vec![chunk_ref("mmm", 100)]));

        let snapshot = engine.snapshot();
        assert_eq!(snapshot.file_ids, vec!["a-file", "m-file", "z-file"]);
    }
}
