//! File attachment handling for encrypted messaging.
//!
//! Files are chunked, individually encrypted, and content-addressed for
//! distributed storage (IPFS, Arweave, etc.). The attachment metadata
//! is sent as part of the message; the encrypted chunks are stored separately.

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use nous_core::{Error, Result};
use nous_crypto::encryption::{self, EncryptedPayload};

/// Default chunk size: 256 KiB.
const DEFAULT_CHUNK_SIZE: usize = 256 * 1024;

/// Maximum file size: 100 MiB.
const MAX_FILE_SIZE: usize = 100 * 1024 * 1024;

/// Metadata describing a file attachment.
/// Sent inside the message envelope; the actual data lives in storage.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AttachmentMeta {
    pub file_name: String,
    pub mime_type: String,
    pub size: u64,
    pub hash: String,
    pub chunk_count: usize,
    pub chunk_size: usize,
    pub chunks: Vec<ChunkRef>,
}

/// Reference to an encrypted chunk stored in content-addressed storage.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChunkRef {
    pub index: usize,
    pub hash: String,
    pub encrypted_size: usize,
}

/// An encrypted chunk ready for storage.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EncryptedChunk {
    pub index: usize,
    pub hash: String,
    pub data: EncryptedPayload,
}

/// Prepares a file for encrypted storage by chunking and encrypting.
pub struct AttachmentEncoder {
    chunk_size: usize,
}

impl AttachmentEncoder {
    pub fn new() -> Self {
        Self {
            chunk_size: DEFAULT_CHUNK_SIZE,
        }
    }

    pub fn with_chunk_size(mut self, size: usize) -> Self {
        assert!(size > 0, "chunk size must be positive");
        self.chunk_size = size;
        self
    }

    /// Encode a file into encrypted chunks.
    /// Returns the attachment metadata and the encrypted chunks.
    pub fn encode(
        &self,
        file_name: &str,
        mime_type: &str,
        data: &[u8],
        encryption_key: &[u8; 32],
    ) -> Result<(AttachmentMeta, Vec<EncryptedChunk>)> {
        if data.len() > MAX_FILE_SIZE {
            return Err(Error::InvalidInput(format!(
                "file too large: {} bytes (max {})",
                data.len(),
                MAX_FILE_SIZE
            )));
        }

        let file_hash = hex_sha256(data);
        let chunks: Vec<&[u8]> = data.chunks(self.chunk_size).collect();
        let chunk_count = chunks.len().max(1); // empty file still has 1 (empty) chunk

        let mut encrypted_chunks = Vec::with_capacity(chunk_count);
        let mut chunk_refs = Vec::with_capacity(chunk_count);

        if data.is_empty() {
            // Handle empty file: one empty chunk
            let encrypted = encryption::encrypt(encryption_key, b"")?;
            let hash = hex_sha256(b"");
            chunk_refs.push(ChunkRef {
                index: 0,
                hash: hash.clone(),
                encrypted_size: serde_json::to_vec(&encrypted)
                    .map(|v| v.len())
                    .unwrap_or(0),
            });
            encrypted_chunks.push(EncryptedChunk {
                index: 0,
                hash,
                data: encrypted,
            });
        } else {
            for (i, chunk) in chunks.iter().enumerate() {
                let chunk_hash = hex_sha256(chunk);
                let encrypted = encryption::encrypt(encryption_key, chunk)?;
                let enc_size = serde_json::to_vec(&encrypted)
                    .map(|v| v.len())
                    .unwrap_or(0);

                chunk_refs.push(ChunkRef {
                    index: i,
                    hash: chunk_hash.clone(),
                    encrypted_size: enc_size,
                });
                encrypted_chunks.push(EncryptedChunk {
                    index: i,
                    hash: chunk_hash,
                    data: encrypted,
                });
            }
        }

        let meta = AttachmentMeta {
            file_name: file_name.into(),
            mime_type: mime_type.into(),
            size: data.len() as u64,
            hash: file_hash,
            chunk_count: encrypted_chunks.len(),
            chunk_size: self.chunk_size,
            chunks: chunk_refs,
        };

        Ok((meta, encrypted_chunks))
    }
}

impl Default for AttachmentEncoder {
    fn default() -> Self {
        Self::new()
    }
}

/// Reassembles a file from encrypted chunks.
pub struct AttachmentDecoder;

impl AttachmentDecoder {
    /// Decrypt and reassemble chunks into the original file data.
    /// Chunks must be provided in order. Verifies integrity against the metadata.
    pub fn decode(
        meta: &AttachmentMeta,
        chunks: &[EncryptedChunk],
        encryption_key: &[u8; 32],
    ) -> Result<Vec<u8>> {
        if chunks.len() != meta.chunk_count {
            return Err(Error::InvalidInput(format!(
                "expected {} chunks, got {}",
                meta.chunk_count,
                chunks.len()
            )));
        }

        let mut data = Vec::with_capacity(meta.size as usize);

        for (i, chunk) in chunks.iter().enumerate() {
            if chunk.index != i {
                return Err(Error::InvalidInput(format!(
                    "chunk out of order: expected index {i}, got {}",
                    chunk.index
                )));
            }

            let plaintext = encryption::decrypt(encryption_key, &chunk.data)?;
            let chunk_hash = hex_sha256(&plaintext);

            if chunk_hash != meta.chunks[i].hash {
                return Err(Error::InvalidInput(format!(
                    "chunk {i} hash mismatch: expected {}, got {chunk_hash}",
                    meta.chunks[i].hash
                )));
            }

            data.extend_from_slice(&plaintext);
        }

        // Verify overall file hash
        let file_hash = hex_sha256(&data);
        if file_hash != meta.hash {
            return Err(Error::InvalidInput(format!(
                "file hash mismatch: expected {}, got {file_hash}",
                meta.hash
            )));
        }

        Ok(data)
    }
}

fn hex_sha256(data: &[u8]) -> String {
    let hash = Sha256::digest(data);
    hex::encode(hash)
}

#[cfg(test)]
mod tests {
    use super::*;
    use rand::rngs::OsRng;
    use rand::RngCore;

    fn random_key() -> [u8; 32] {
        let mut key = [0u8; 32];
        OsRng.fill_bytes(&mut key);
        key
    }

    #[test]
    fn encode_and_decode_small_file() {
        let key = random_key();
        let data = b"hello world";
        let encoder = AttachmentEncoder::new();

        let (meta, chunks) = encoder.encode("test.txt", "text/plain", data, &key).unwrap();

        assert_eq!(meta.file_name, "test.txt");
        assert_eq!(meta.mime_type, "text/plain");
        assert_eq!(meta.size, 11);
        assert_eq!(meta.chunk_count, 1);

        let decoded = AttachmentDecoder::decode(&meta, &chunks, &key).unwrap();
        assert_eq!(decoded, data);
    }

    #[test]
    fn encode_and_decode_multi_chunk() {
        let key = random_key();
        let data = vec![0xAB; 1000];
        let encoder = AttachmentEncoder::new().with_chunk_size(400);

        let (meta, chunks) = encoder
            .encode("big.bin", "application/octet-stream", &data, &key)
            .unwrap();

        assert_eq!(meta.chunk_count, 3); // 400 + 400 + 200
        assert_eq!(meta.chunk_size, 400);

        let decoded = AttachmentDecoder::decode(&meta, &chunks, &key).unwrap();
        assert_eq!(decoded, data);
    }

    #[test]
    fn empty_file() {
        let key = random_key();
        let encoder = AttachmentEncoder::new();

        let (meta, chunks) = encoder.encode("empty.txt", "text/plain", b"", &key).unwrap();

        assert_eq!(meta.size, 0);
        assert_eq!(meta.chunk_count, 1);

        let decoded = AttachmentDecoder::decode(&meta, &chunks, &key).unwrap();
        assert!(decoded.is_empty());
    }

    #[test]
    fn wrong_key_fails() {
        let key = random_key();
        let wrong_key = random_key();
        let encoder = AttachmentEncoder::new();

        let (meta, chunks) = encoder
            .encode("secret.txt", "text/plain", b"secret data", &key)
            .unwrap();

        assert!(AttachmentDecoder::decode(&meta, &chunks, &wrong_key).is_err());
    }

    #[test]
    fn tampered_chunk_detected() {
        let key = random_key();
        let encoder = AttachmentEncoder::new().with_chunk_size(10);
        let data = b"hello world!"; // 12 bytes → 2 chunks

        let (meta, mut chunks) = encoder
            .encode("test.txt", "text/plain", data, &key)
            .unwrap();

        // Tamper: replace chunk 1's data with chunk 0's data
        chunks[1].data = chunks[0].data.clone();

        assert!(AttachmentDecoder::decode(&meta, &chunks, &key).is_err());
    }

    #[test]
    fn chunk_count_mismatch() {
        let key = random_key();
        let encoder = AttachmentEncoder::new();

        let (meta, mut chunks) = encoder
            .encode("test.txt", "text/plain", b"data", &key)
            .unwrap();

        chunks.push(chunks[0].clone());
        assert!(AttachmentDecoder::decode(&meta, &chunks, &key).is_err());
    }

    #[test]
    fn chunk_order_mismatch() {
        let key = random_key();
        let encoder = AttachmentEncoder::new().with_chunk_size(5);
        let data = b"hello world!"; // 12 bytes → 3 chunks

        let (meta, mut chunks) = encoder
            .encode("test.txt", "text/plain", data, &key)
            .unwrap();

        chunks.swap(0, 1);
        assert!(AttachmentDecoder::decode(&meta, &chunks, &key).is_err());
    }

    #[test]
    fn file_too_large() {
        let key = random_key();
        let encoder = AttachmentEncoder::new();
        let data = vec![0u8; MAX_FILE_SIZE + 1];

        assert!(encoder
            .encode("huge.bin", "application/octet-stream", &data, &key)
            .is_err());
    }

    #[test]
    fn exact_chunk_boundary() {
        let key = random_key();
        let encoder = AttachmentEncoder::new().with_chunk_size(10);
        let data = vec![0xCC; 30]; // exactly 3 chunks

        let (meta, chunks) = encoder
            .encode("exact.bin", "application/octet-stream", &data, &key)
            .unwrap();

        assert_eq!(meta.chunk_count, 3);
        let decoded = AttachmentDecoder::decode(&meta, &chunks, &key).unwrap();
        assert_eq!(decoded, data);
    }

    #[test]
    fn metadata_serializes() {
        let key = random_key();
        let encoder = AttachmentEncoder::new();
        let (meta, _) = encoder
            .encode("test.txt", "text/plain", b"hello", &key)
            .unwrap();

        let json = serde_json::to_string(&meta).unwrap();
        let restored: AttachmentMeta = serde_json::from_str(&json).unwrap();
        assert_eq!(restored.file_name, "test.txt");
        assert_eq!(restored.hash, meta.hash);
    }

    #[test]
    fn encrypted_chunk_serializes() {
        let key = random_key();
        let encoder = AttachmentEncoder::new();
        let (_, chunks) = encoder
            .encode("test.txt", "text/plain", b"hello", &key)
            .unwrap();

        let json = serde_json::to_string(&chunks[0]).unwrap();
        let restored: EncryptedChunk = serde_json::from_str(&json).unwrap();
        assert_eq!(restored.index, 0);
    }

    #[test]
    fn hash_is_deterministic() {
        let key = random_key();
        let encoder = AttachmentEncoder::new();
        let data = b"deterministic";

        let (meta1, _) = encoder
            .encode("a.txt", "text/plain", data, &key)
            .unwrap();
        let (meta2, _) = encoder
            .encode("b.txt", "text/plain", data, &key)
            .unwrap();

        assert_eq!(meta1.hash, meta2.hash);
        assert_eq!(meta1.chunks[0].hash, meta2.chunks[0].hash);
    }

    #[test]
    fn chunk_refs_have_correct_indices() {
        let key = random_key();
        let encoder = AttachmentEncoder::new().with_chunk_size(5);
        let data = b"abcdefghij12345"; // 15 bytes → 3 chunks

        let (meta, _) = encoder
            .encode("test.txt", "text/plain", data, &key)
            .unwrap();

        for (i, chunk_ref) in meta.chunks.iter().enumerate() {
            assert_eq!(chunk_ref.index, i);
            assert!(!chunk_ref.hash.is_empty());
            assert!(chunk_ref.encrypted_size > 0);
        }
    }
}
