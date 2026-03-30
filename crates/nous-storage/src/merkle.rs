//! Merkle tree for content-addressed verification and efficient sync.
//!
//! Binary Merkle tree with SHA-256 hashing. Supports:
//! - Building a tree from leaf data
//! - Generating inclusion proofs for any leaf
//! - Verifying proofs against a root hash
//! - Incremental updates (replace a leaf and recompute)
//! - Diff detection between trees (find diverging subtrees)

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

/// A 32-byte hash (SHA-256 output).
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct MerkleHash(pub [u8; 32]);

impl MerkleHash {
    pub fn as_bytes(&self) -> &[u8; 32] {
        &self.0
    }

    pub fn to_hex(&self) -> String {
        hex::encode(self.0)
    }

    pub fn from_hex(s: &str) -> Result<Self, hex::FromHexError> {
        let bytes = hex::decode(s)?;
        let mut arr = [0u8; 32];
        arr.copy_from_slice(&bytes);
        Ok(Self(arr))
    }
}

impl std::fmt::Display for MerkleHash {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", &self.to_hex()[..12])
    }
}

/// Hash leaf data with a domain separator to prevent second-preimage attacks.
fn hash_leaf(data: &[u8]) -> MerkleHash {
    let mut hasher = Sha256::new();
    hasher.update([0x00]); // Leaf prefix.
    hasher.update(data);
    MerkleHash(hasher.finalize().into())
}

/// Hash two child nodes with a domain separator.
fn hash_node(left: &MerkleHash, right: &MerkleHash) -> MerkleHash {
    let mut hasher = Sha256::new();
    hasher.update([0x01]); // Node prefix.
    hasher.update(left.0);
    hasher.update(right.0);
    MerkleHash(hasher.finalize().into())
}

/// Direction in a proof path.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ProofDirection {
    Left,
    Right,
}

/// A single step in a Merkle proof.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProofStep {
    pub hash: MerkleHash,
    pub direction: ProofDirection,
}

/// An inclusion proof for a leaf in a Merkle tree.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MerkleProof {
    pub leaf_hash: MerkleHash,
    pub path: Vec<ProofStep>,
    pub root: MerkleHash,
}

impl MerkleProof {
    /// Verify this proof against a root hash.
    pub fn verify(&self) -> bool {
        self.verify_against(&self.root)
    }

    /// Verify this proof against a specific root hash.
    pub fn verify_against(&self, expected_root: &MerkleHash) -> bool {
        let mut current = self.leaf_hash.clone();

        for step in &self.path {
            current = match step.direction {
                ProofDirection::Left => hash_node(&step.hash, &current),
                ProofDirection::Right => hash_node(&current, &step.hash),
            };
        }

        current == *expected_root
    }
}

/// A binary Merkle tree.
///
/// Stores all nodes in a flat array for cache-friendly access.
/// The tree is always a complete binary tree — if the number of leaves
/// is not a power of two, empty leaves are padded.
pub struct MerkleTree {
    /// All nodes in level order. nodes[0] is the root.
    nodes: Vec<MerkleHash>,
    /// Number of actual (non-padding) leaves.
    leaf_count: usize,
    /// Total leaves including padding (always a power of two).
    padded_count: usize,
}

impl MerkleTree {
    /// Build a Merkle tree from leaf data.
    pub fn from_leaves(data: &[&[u8]]) -> Self {
        if data.is_empty() {
            return Self {
                nodes: vec![hash_leaf(b"")],
                leaf_count: 0,
                padded_count: 1,
            };
        }

        let padded_count = data.len().next_power_of_two();
        let total_nodes = 2 * padded_count - 1;
        let internal_count = padded_count - 1;

        let mut nodes = vec![MerkleHash([0u8; 32]); total_nodes];

        // Fill leaf layer.
        let empty_hash = hash_leaf(b"");
        for i in 0..padded_count {
            let hash = if i < data.len() {
                hash_leaf(data[i])
            } else {
                empty_hash.clone()
            };
            nodes[internal_count + i] = hash;
        }

        // Build internal nodes bottom-up.
        for i in (0..internal_count).rev() {
            let left = &nodes[2 * i + 1];
            let right = &nodes[2 * i + 2];
            nodes[i] = hash_node(left, right);
        }

        Self {
            nodes,
            leaf_count: data.len(),
            padded_count,
        }
    }

    /// Get the root hash.
    pub fn root(&self) -> &MerkleHash {
        &self.nodes[0]
    }

    /// Number of actual leaves (excluding padding).
    pub fn leaf_count(&self) -> usize {
        self.leaf_count
    }

    /// Get the hash of a leaf at a given index.
    pub fn leaf_hash(&self, index: usize) -> Option<&MerkleHash> {
        if index >= self.leaf_count {
            return None;
        }
        let internal_count = self.padded_count - 1;
        Some(&self.nodes[internal_count + index])
    }

    /// Generate an inclusion proof for the leaf at `index`.
    pub fn proof(&self, index: usize) -> Option<MerkleProof> {
        if index >= self.leaf_count {
            return None;
        }

        let internal_count = self.padded_count - 1;
        let mut path = Vec::new();
        let mut node_idx = internal_count + index;

        while node_idx > 0 {
            let sibling_idx = if node_idx % 2 == 1 {
                node_idx + 1 // We're left child, sibling is right.
            } else {
                node_idx - 1 // We're right child, sibling is left.
            };

            let direction = if node_idx % 2 == 1 {
                ProofDirection::Right
            } else {
                ProofDirection::Left
            };

            path.push(ProofStep {
                hash: self.nodes[sibling_idx].clone(),
                direction,
            });

            node_idx = (node_idx - 1) / 2; // Move to parent.
        }

        Some(MerkleProof {
            leaf_hash: self.nodes[internal_count + index].clone(),
            path,
            root: self.root().clone(),
        })
    }

    /// Update a leaf and recompute affected nodes.
    pub fn update_leaf(&mut self, index: usize, data: &[u8]) -> bool {
        if index >= self.leaf_count {
            return false;
        }

        let internal_count = self.padded_count - 1;
        let leaf_idx = internal_count + index;
        self.nodes[leaf_idx] = hash_leaf(data);

        // Recompute up to root.
        let mut node_idx = leaf_idx;
        while node_idx > 0 {
            let parent = (node_idx - 1) / 2;
            let left = &self.nodes[2 * parent + 1].clone();
            let right = &self.nodes[2 * parent + 2].clone();
            self.nodes[parent] = hash_node(left, right);
            node_idx = parent;
        }

        true
    }

    /// Find differing leaf indices between two trees of the same size.
    pub fn diff(&self, other: &MerkleTree) -> Vec<usize> {
        if self.padded_count != other.padded_count {
            // Different sizes — all leaves differ.
            return (0..self.leaf_count.max(other.leaf_count)).collect();
        }

        if self.root() == other.root() {
            return Vec::new();
        }

        let mut diffs = Vec::new();
        self.diff_recursive(other, 0, &mut diffs);
        // Only return actual (non-padding) leaf indices.
        diffs.retain(|&i| i < self.leaf_count.max(other.leaf_count));
        diffs
    }

    fn diff_recursive(&self, other: &MerkleTree, node_idx: usize, diffs: &mut Vec<usize>) {
        if node_idx >= self.nodes.len() {
            return;
        }

        if self.nodes[node_idx] == other.nodes[node_idx] {
            return; // Subtree is identical.
        }

        let internal_count = self.padded_count - 1;
        if node_idx >= internal_count {
            // Leaf node — record the difference.
            diffs.push(node_idx - internal_count);
            return;
        }

        // Recurse into children.
        self.diff_recursive(other, 2 * node_idx + 1, diffs);
        self.diff_recursive(other, 2 * node_idx + 2, diffs);
    }

    /// Total number of nodes (internal + leaves including padding).
    pub fn node_count(&self) -> usize {
        self.nodes.len()
    }

    /// Depth of the tree (number of levels including root).
    pub fn depth(&self) -> usize {
        if self.nodes.is_empty() {
            return 0;
        }
        (self.padded_count as f64).log2() as usize + 1
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hash_leaf_deterministic() {
        let h1 = hash_leaf(b"hello");
        let h2 = hash_leaf(b"hello");
        assert_eq!(h1, h2);
    }

    #[test]
    fn hash_leaf_differs_for_different_data() {
        let h1 = hash_leaf(b"hello");
        let h2 = hash_leaf(b"world");
        assert_ne!(h1, h2);
    }

    #[test]
    fn hash_node_differs_from_leaf() {
        let h_leaf = hash_leaf(b"hello");
        let h_node = hash_node(&hash_leaf(b"he"), &hash_leaf(b"llo"));
        assert_ne!(h_leaf, h_node);
    }

    #[test]
    fn merkle_hash_hex_roundtrip() {
        let hash = hash_leaf(b"test");
        let hex_str = hash.to_hex();
        let restored = MerkleHash::from_hex(&hex_str).unwrap();
        assert_eq!(hash, restored);
    }

    #[test]
    fn merkle_hash_display() {
        let hash = hash_leaf(b"test");
        let display = format!("{hash}");
        assert_eq!(display.len(), 12); // First 12 hex chars.
    }

    #[test]
    fn tree_single_leaf() {
        let tree = MerkleTree::from_leaves(&[b"only"]);
        assert_eq!(tree.leaf_count(), 1);
        assert_eq!(tree.root(), tree.leaf_hash(0).unwrap());
    }

    #[test]
    fn tree_two_leaves() {
        let tree = MerkleTree::from_leaves(&[b"left", b"right"]);
        assert_eq!(tree.leaf_count(), 2);

        let expected_root = hash_node(
            &hash_leaf(b"left"),
            &hash_leaf(b"right"),
        );
        assert_eq!(*tree.root(), expected_root);
    }

    #[test]
    fn tree_four_leaves() {
        let tree = MerkleTree::from_leaves(&[b"a", b"b", b"c", b"d"]);
        assert_eq!(tree.leaf_count(), 4);
        assert_eq!(tree.depth(), 3); // root + 2 levels.
        assert_eq!(tree.node_count(), 7); // 4 leaves + 3 internal.
    }

    #[test]
    fn tree_three_leaves_padded() {
        let tree = MerkleTree::from_leaves(&[b"a", b"b", b"c"]);
        assert_eq!(tree.leaf_count(), 3);
        // Padded to 4 leaves.
        assert_eq!(tree.node_count(), 7);
    }

    #[test]
    fn tree_empty() {
        let tree = MerkleTree::from_leaves(&[]);
        assert_eq!(tree.leaf_count(), 0);
    }

    #[test]
    fn tree_root_changes_with_data() {
        let t1 = MerkleTree::from_leaves(&[b"a", b"b"]);
        let t2 = MerkleTree::from_leaves(&[b"a", b"c"]);
        assert_ne!(t1.root(), t2.root());
    }

    #[test]
    fn tree_root_order_matters() {
        let t1 = MerkleTree::from_leaves(&[b"a", b"b"]);
        let t2 = MerkleTree::from_leaves(&[b"b", b"a"]);
        assert_ne!(t1.root(), t2.root());
    }

    #[test]
    fn proof_single_leaf() {
        let tree = MerkleTree::from_leaves(&[b"only"]);
        let proof = tree.proof(0).unwrap();
        assert!(proof.verify());
        assert!(proof.path.is_empty());
    }

    #[test]
    fn proof_two_leaves() {
        let tree = MerkleTree::from_leaves(&[b"left", b"right"]);

        let proof0 = tree.proof(0).unwrap();
        assert!(proof0.verify());
        assert_eq!(proof0.path.len(), 1);

        let proof1 = tree.proof(1).unwrap();
        assert!(proof1.verify());
    }

    #[test]
    fn proof_four_leaves() {
        let tree = MerkleTree::from_leaves(&[b"a", b"b", b"c", b"d"]);

        for i in 0..4 {
            let proof = tree.proof(i).unwrap();
            assert!(proof.verify(), "proof for leaf {i} should verify");
            assert_eq!(proof.path.len(), 2); // log2(4) = 2.
        }
    }

    #[test]
    fn proof_eight_leaves() {
        let data: Vec<Vec<u8>> = (0..8).map(|i| vec![i as u8]).collect();
        let refs: Vec<&[u8]> = data.iter().map(|d| d.as_slice()).collect();
        let tree = MerkleTree::from_leaves(&refs);

        for i in 0..8 {
            let proof = tree.proof(i).unwrap();
            assert!(proof.verify(), "proof for leaf {i} should verify");
            assert_eq!(proof.path.len(), 3); // log2(8) = 3.
        }
    }

    #[test]
    fn proof_rejects_wrong_root() {
        let tree = MerkleTree::from_leaves(&[b"a", b"b", b"c", b"d"]);
        let proof = tree.proof(0).unwrap();

        let wrong_root = hash_leaf(b"wrong");
        assert!(!proof.verify_against(&wrong_root));
    }

    #[test]
    fn proof_out_of_bounds() {
        let tree = MerkleTree::from_leaves(&[b"a", b"b"]);
        assert!(tree.proof(2).is_none());
        assert!(tree.proof(100).is_none());
    }

    #[test]
    fn proof_serializes() {
        let tree = MerkleTree::from_leaves(&[b"a", b"b", b"c", b"d"]);
        let proof = tree.proof(2).unwrap();

        let json = serde_json::to_string(&proof).unwrap();
        let restored: MerkleProof = serde_json::from_str(&json).unwrap();
        assert!(restored.verify());
    }

    #[test]
    fn update_leaf_changes_root() {
        let mut tree = MerkleTree::from_leaves(&[b"a", b"b", b"c", b"d"]);
        let old_root = tree.root().clone();

        tree.update_leaf(2, b"modified");
        assert_ne!(*tree.root(), old_root);

        // Old proofs should no longer verify.
        // But new proofs should.
        let proof = tree.proof(2).unwrap();
        assert!(proof.verify());
        assert_eq!(proof.leaf_hash, hash_leaf(b"modified"));
    }

    #[test]
    fn update_leaf_preserves_others() {
        let mut tree = MerkleTree::from_leaves(&[b"a", b"b", b"c", b"d"]);
        let hash_a = tree.leaf_hash(0).unwrap().clone();

        tree.update_leaf(2, b"modified");
        assert_eq!(*tree.leaf_hash(0).unwrap(), hash_a);
    }

    #[test]
    fn update_leaf_out_of_bounds() {
        let mut tree = MerkleTree::from_leaves(&[b"a", b"b"]);
        assert!(!tree.update_leaf(5, b"nope"));
    }

    #[test]
    fn diff_identical_trees() {
        let t1 = MerkleTree::from_leaves(&[b"a", b"b", b"c", b"d"]);
        let t2 = MerkleTree::from_leaves(&[b"a", b"b", b"c", b"d"]);
        assert!(t1.diff(&t2).is_empty());
    }

    #[test]
    fn diff_single_change() {
        let t1 = MerkleTree::from_leaves(&[b"a", b"b", b"c", b"d"]);
        let t2 = MerkleTree::from_leaves(&[b"a", b"b", b"X", b"d"]);
        let diffs = t1.diff(&t2);
        assert_eq!(diffs, vec![2]);
    }

    #[test]
    fn diff_multiple_changes() {
        let t1 = MerkleTree::from_leaves(&[b"a", b"b", b"c", b"d"]);
        let t2 = MerkleTree::from_leaves(&[b"X", b"b", b"c", b"Y"]);
        let diffs = t1.diff(&t2);
        assert_eq!(diffs, vec![0, 3]);
    }

    #[test]
    fn diff_all_different() {
        let t1 = MerkleTree::from_leaves(&[b"a", b"b"]);
        let t2 = MerkleTree::from_leaves(&[b"x", b"y"]);
        let diffs = t1.diff(&t2);
        assert_eq!(diffs.len(), 2);
    }

    #[test]
    fn leaf_hash_returns_none_for_invalid() {
        let tree = MerkleTree::from_leaves(&[b"a"]);
        assert!(tree.leaf_hash(0).is_some());
        assert!(tree.leaf_hash(1).is_none());
    }

    #[test]
    fn tree_large_dataset() {
        let data: Vec<Vec<u8>> = (0..100).map(|i| format!("item-{i}").into_bytes()).collect();
        let refs: Vec<&[u8]> = data.iter().map(|d| d.as_slice()).collect();
        let tree = MerkleTree::from_leaves(&refs);

        assert_eq!(tree.leaf_count(), 100);
        // 100 → padded to 128.
        assert_eq!(tree.node_count(), 255); // 2*128 - 1.

        // Verify random proofs.
        for i in [0, 42, 99] {
            let proof = tree.proof(i).unwrap();
            assert!(proof.verify(), "proof for leaf {i} should verify");
        }
    }
}
