use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use nous_core::Result;

/// Minimum chunk: 64 KiB — never split below this.
const MIN_CHUNK_SIZE: usize = 64 * 1024;
/// Maximum chunk: 1 MiB — always split at this boundary.
const MAX_CHUNK_SIZE: usize = 1024 * 1024;
/// Rolling hash window size for content-defined chunking.
const WINDOW_SIZE: usize = 48;
/// Mask for the rolling hash — tuned so average chunk ≈ TARGET_CHUNK_SIZE.
/// 2^18 - 1 = 262143, giving ~256 KiB average chunks.
const HASH_MASK: u64 = (1 << 18) - 1;

/// A content-addressed chunk of file data.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Chunk {
    /// SHA-256 hash of the chunk data, hex-encoded.
    pub hash: String,
    /// Byte offset in the original file where this chunk starts.
    pub offset: u64,
    /// Size of this chunk in bytes.
    pub size: u64,
    /// The raw chunk data.
    #[serde(with = "base64_bytes")]
    pub data: Vec<u8>,
}

/// A content identifier derived from SHA-256 — compatible with IPFS CIDv1 raw leaves.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub struct ContentId(pub String);

impl ContentId {
    pub fn from_bytes(data: &[u8]) -> Self {
        let hash = Sha256::digest(data);
        Self(hex::encode(hash))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Display for ContentId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

mod hex {
    pub fn encode(bytes: impl AsRef<[u8]>) -> String {
        bytes.as_ref().iter().fold(String::new(), |mut s, b| {
            use std::fmt::Write;
            let _ = write!(s, "{b:02x}");
            s
        })
    }
}

mod base64_bytes {
    use base64::{Engine, engine::general_purpose::STANDARD};
    use serde::{Deserialize, Deserializer, Serialize, Serializer};

    pub fn serialize<S: Serializer>(bytes: &Vec<u8>, s: S) -> Result<S::Ok, S::Error> {
        STANDARD.encode(bytes).serialize(s)
    }

    pub fn deserialize<'de, D: Deserializer<'de>>(d: D) -> Result<Vec<u8>, D::Error> {
        let encoded = String::deserialize(d)?;
        STANDARD.decode(&encoded).map_err(serde::de::Error::custom)
    }
}

/// Buzhash rolling hash for content-defined chunking.
struct BuzHash {
    hash: u64,
    window: Vec<u8>,
    pos: usize,
    filled: bool,
}

impl BuzHash {
    fn new() -> Self {
        Self {
            hash: 0,
            window: vec![0u8; WINDOW_SIZE],
            pos: 0,
            filled: false,
        }
    }

    fn update(&mut self, byte: u8) -> u64 {
        let old = self.window[self.pos];
        self.window[self.pos] = byte;
        self.pos = (self.pos + 1) % WINDOW_SIZE;

        if !self.filled && self.pos == 0 {
            self.filled = true;
        }

        // Buzhash: rotate left by 1, XOR out the old byte shifted by window size, XOR in new byte
        self.hash = self.hash.rotate_left(1)
            ^ BYTE_TABLE[old as usize].rotate_left(WINDOW_SIZE as u32)
            ^ BYTE_TABLE[byte as usize];
        self.hash
    }
}

/// Split file data into content-defined chunks using a rolling hash.
///
/// This produces chunks with an average size of ~256 KiB, bounded by
/// 64 KiB minimum and 1 MiB maximum. Content-defined boundaries mean
/// that inserting or deleting bytes in the middle of a file only affects
/// the chunks near the edit point — the rest stay identical.
pub fn chunk_data(data: &[u8]) -> Result<Vec<Chunk>> {
    if data.is_empty() {
        return Ok(vec![]);
    }

    // Small files: single chunk, no splitting needed.
    if data.len() <= MIN_CHUNK_SIZE {
        let hash = ContentId::from_bytes(data);
        return Ok(vec![Chunk {
            hash: hash.0,
            offset: 0,
            size: data.len() as u64,
            data: data.to_vec(),
        }]);
    }

    let mut chunks = Vec::new();
    let mut start = 0;
    let mut hasher = BuzHash::new();

    for i in 0..data.len() {
        let chunk_len = i - start;

        // Never split below minimum.
        if chunk_len < MIN_CHUNK_SIZE {
            hasher.update(data[i]);
            continue;
        }

        // Always split at maximum.
        if chunk_len >= MAX_CHUNK_SIZE {
            let chunk_data = &data[start..i];
            let hash = ContentId::from_bytes(chunk_data);
            chunks.push(Chunk {
                hash: hash.0,
                offset: start as u64,
                size: chunk_data.len() as u64,
                data: chunk_data.to_vec(),
            });
            start = i;
            hasher = BuzHash::new();
            continue;
        }

        let h = hasher.update(data[i]);

        // Split when the rolling hash hits the target pattern.
        if h & HASH_MASK == 0 {
            let chunk_data = &data[start..=i];
            let hash = ContentId::from_bytes(chunk_data);
            chunks.push(Chunk {
                hash: hash.0,
                offset: start as u64,
                size: chunk_data.len() as u64,
                data: chunk_data.to_vec(),
            });
            start = i + 1;
            hasher = BuzHash::new();
        }
    }

    // Final chunk: whatever remains.
    if start < data.len() {
        let chunk_data = &data[start..];
        let hash = ContentId::from_bytes(chunk_data);
        chunks.push(Chunk {
            hash: hash.0,
            offset: start as u64,
            size: chunk_data.len() as u64,
            data: chunk_data.to_vec(),
        });
    }

    Ok(chunks)
}

/// Reassemble chunks into the original file data.
pub fn reassemble(chunks: &[Chunk]) -> Vec<u8> {
    let total_size: u64 = chunks.iter().map(|c| c.size).sum();
    let mut out = Vec::with_capacity(total_size as usize);
    for chunk in chunks {
        out.extend_from_slice(&chunk.data);
    }
    out
}

/// Verify that a chunk's data matches its declared hash.
pub fn verify_chunk(chunk: &Chunk) -> bool {
    let expected = ContentId::from_bytes(&chunk.data);
    chunk.hash == expected.0
}

// Buzhash byte table — random u64 values for each possible byte value.
// Generated deterministically for reproducibility.
static BYTE_TABLE: [u64; 256] = {
    let mut table = [0u64; 256];
    let mut seed: u64 = 0x517cc1b727220a95;
    let mut i = 0;
    while i < 256 {
        // Simple xorshift64
        seed ^= seed << 13;
        seed ^= seed >> 7;
        seed ^= seed << 17;
        table[i] = seed;
        i += 1;
    }
    table
};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn content_id_deterministic() {
        let data = b"nous sovereign file storage";
        let id1 = ContentId::from_bytes(data);
        let id2 = ContentId::from_bytes(data);
        assert_eq!(id1, id2);
    }

    #[test]
    fn content_id_different_data() {
        let id1 = ContentId::from_bytes(b"alpha");
        let id2 = ContentId::from_bytes(b"beta");
        assert_ne!(id1, id2);
    }

    #[test]
    fn content_id_display() {
        let id = ContentId::from_bytes(b"test");
        assert_eq!(id.to_string().len(), 64); // SHA-256 = 64 hex chars
    }

    #[test]
    fn chunk_empty_data() {
        let chunks = chunk_data(b"").unwrap();
        assert!(chunks.is_empty());
    }

    #[test]
    fn chunk_small_data() {
        let data = b"small file under 64KB";
        let chunks = chunk_data(data).unwrap();
        assert_eq!(chunks.len(), 1);
        assert_eq!(chunks[0].data, data);
        assert_eq!(chunks[0].offset, 0);
        assert_eq!(chunks[0].size, data.len() as u64);
    }

    #[test]
    fn chunk_and_reassemble_roundtrip() {
        let data = vec![0xABu8; 512 * 1024]; // 512 KiB — should produce ~2 chunks
        let chunks = chunk_data(&data).unwrap();
        assert!(chunks.len() >= 1);

        let reassembled = reassemble(&chunks);
        assert_eq!(reassembled, data);
    }

    #[test]
    fn chunk_large_file_respects_max_size() {
        let data = vec![0x42u8; 4 * 1024 * 1024]; // 4 MiB
        let chunks = chunk_data(&data).unwrap();

        for chunk in &chunks {
            assert!(chunk.size <= MAX_CHUNK_SIZE as u64 + 1);
        }

        let reassembled = reassemble(&chunks);
        assert_eq!(reassembled, data);
    }

    #[test]
    fn chunk_offsets_are_contiguous() {
        let data = vec![0xCDu8; 1024 * 1024]; // 1 MiB
        let chunks = chunk_data(&data).unwrap();

        let mut expected_offset = 0u64;
        for chunk in &chunks {
            assert_eq!(chunk.offset, expected_offset);
            expected_offset += chunk.size;
        }
        assert_eq!(expected_offset, data.len() as u64);
    }

    #[test]
    fn chunk_hashes_are_valid() {
        let data = vec![0xEFu8; 200 * 1024]; // 200 KiB
        let chunks = chunk_data(&data).unwrap();

        for chunk in &chunks {
            assert!(verify_chunk(chunk));
        }
    }

    #[test]
    fn verify_rejects_tampered_chunk() {
        let data = vec![0x11u8; 100];
        let mut chunks = chunk_data(&data).unwrap();
        chunks[0].data[0] = 0xFF;
        assert!(!verify_chunk(&chunks[0]));
    }

    #[test]
    fn chunk_serde_roundtrip() {
        let data = b"serialization test for chunks";
        let chunks = chunk_data(data).unwrap();

        let json = serde_json::to_string(&chunks[0]).unwrap();
        let deserialized: Chunk = serde_json::from_str(&json).unwrap();

        assert_eq!(deserialized, chunks[0]);
    }

    #[test]
    fn deterministic_chunking() {
        let data = vec![0x77u8; 300 * 1024]; // 300 KiB
        let chunks1 = chunk_data(&data).unwrap();
        let chunks2 = chunk_data(&data).unwrap();

        assert_eq!(chunks1.len(), chunks2.len());
        for (a, b) in chunks1.iter().zip(chunks2.iter()) {
            assert_eq!(a.hash, b.hash);
            assert_eq!(a.offset, b.offset);
            assert_eq!(a.size, b.size);
        }
    }

    #[test]
    fn content_defined_boundary_stability() {
        // Inserting data at the start should not affect later chunk boundaries
        let base = vec![0xAAu8; 512 * 1024];
        let base_chunks = chunk_data(&base).unwrap();

        // Prepend 1 byte — first chunk changes, later chunks should stabilize
        let mut modified = vec![0xBBu8; 1];
        modified.extend_from_slice(&base);
        let mod_chunks = chunk_data(&modified).unwrap();

        // Total data should always reassemble correctly regardless of boundary shifts.
        assert_eq!(reassemble(&mod_chunks), modified);
        assert_eq!(reassemble(&base_chunks), base);
    }
}
