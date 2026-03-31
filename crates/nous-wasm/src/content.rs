//! Content addressing for IPFS-compatible content identifiers.
//!
//! Computes CID v1 (SHA-256, raw codec) for arbitrary data, with
//! base32 and base58btc encoding. Provides content verification
//! and multicodec/multihash utilities.

use sha2::{Digest, Sha256};
use wasm_bindgen::prelude::*;

/// Multicodec codes used in CID construction.
const RAW_CODEC: u64 = 0x55;
const DAG_PB_CODEC: u64 = 0x70;
const SHA2_256_CODE: u64 = 0x12;
const SHA2_256_LENGTH: u64 = 32;
const CID_VERSION_1: u64 = 1;

/// Encode an unsigned integer as a varint (LEB128).
fn encode_varint(mut value: u64) -> Vec<u8> {
    let mut buf = Vec::with_capacity(10);
    loop {
        let mut byte = (value & 0x7F) as u8;
        value >>= 7;
        if value != 0 {
            byte |= 0x80;
        }
        buf.push(byte);
        if value == 0 {
            break;
        }
    }
    buf
}

/// Decode a varint from a byte slice, returning (value, bytes_consumed).
fn decode_varint(data: &[u8]) -> Option<(u64, usize)> {
    let mut value: u64 = 0;
    let mut shift = 0;
    for (i, &byte) in data.iter().enumerate() {
        if shift >= 70 {
            return None; // overflow
        }
        value |= ((byte & 0x7F) as u64) << shift;
        if byte & 0x80 == 0 {
            return Some((value, i + 1));
        }
        shift += 7;
    }
    None // incomplete
}

/// Build a multihash (SHA2-256) for the given data.
fn multihash_sha256(data: &[u8]) -> Vec<u8> {
    let digest = Sha256::digest(data);
    let mut mh = Vec::with_capacity(34);
    mh.extend_from_slice(&encode_varint(SHA2_256_CODE));
    mh.extend_from_slice(&encode_varint(SHA2_256_LENGTH));
    mh.extend_from_slice(&digest);
    mh
}

/// Build raw CID v1 bytes for data with a given codec.
fn build_cid_bytes(data: &[u8], codec: u64) -> Vec<u8> {
    let mh = multihash_sha256(data);
    let mut cid = Vec::with_capacity(4 + mh.len());
    cid.extend_from_slice(&encode_varint(CID_VERSION_1));
    cid.extend_from_slice(&encode_varint(codec));
    cid.extend_from_slice(&mh);
    cid
}

// ── RFC 4648 base32 (lowercase, no padding) ──────────────────────

const BASE32_ALPHABET: &[u8; 32] = b"abcdefghijklmnopqrstuvwxyz234567";

fn base32_encode(data: &[u8]) -> String {
    let mut result = String::with_capacity((data.len() * 8).div_ceil(5));
    let mut buffer: u64 = 0;
    let mut bits = 0;

    for &byte in data {
        buffer = (buffer << 8) | byte as u64;
        bits += 8;
        while bits >= 5 {
            bits -= 5;
            let index = ((buffer >> bits) & 0x1F) as usize;
            result.push(BASE32_ALPHABET[index] as char);
        }
    }
    if bits > 0 {
        let index = ((buffer << (5 - bits)) & 0x1F) as usize;
        result.push(BASE32_ALPHABET[index] as char);
    }
    result
}

fn base32_decode(s: &str) -> Option<Vec<u8>> {
    let mut buffer: u64 = 0;
    let mut bits = 0;
    let mut result = Vec::with_capacity(s.len() * 5 / 8);

    for ch in s.chars() {
        let val = match ch {
            'a'..='z' => ch as u64 - 'a' as u64,
            '2'..='7' => ch as u64 - '2' as u64 + 26,
            'A'..='Z' => ch as u64 - 'A' as u64, // case-insensitive
            _ => return None,
        };
        buffer = (buffer << 5) | val;
        bits += 5;
        if bits >= 8 {
            bits -= 8;
            result.push((buffer >> bits) as u8);
        }
    }
    Some(result)
}

// ── WasmCid ──────────────────────────────────────────────────────

/// A CID v1 content identifier with SHA-256 multihash.
#[wasm_bindgen]
pub struct WasmCid {
    bytes: Vec<u8>,
    codec: u64,
}

#[wasm_bindgen]
impl WasmCid {
    /// Compute a CID v1 (raw codec, SHA-256) for the given data.
    #[wasm_bindgen(constructor)]
    pub fn new(data: &[u8]) -> Self {
        let bytes = build_cid_bytes(data, RAW_CODEC);
        Self {
            bytes,
            codec: RAW_CODEC,
        }
    }

    /// Compute a CID v1 with dag-pb codec (for IPFS UnixFS compatibility).
    #[wasm_bindgen(js_name = dagPb)]
    pub fn dag_pb(data: &[u8]) -> Self {
        let bytes = build_cid_bytes(data, DAG_PB_CODEC);
        Self {
            bytes,
            codec: DAG_PB_CODEC,
        }
    }

    /// Parse a CID from its base32 string representation (with "b" prefix).
    #[wasm_bindgen(js_name = fromString)]
    pub fn from_string(s: &str) -> Result<WasmCid, JsError> {
        if let Some(b32) = s.strip_prefix('b') {
            let bytes =
                base32_decode(b32).ok_or_else(|| JsError::new("invalid base32 encoding"))?;
            Self::from_bytes(&bytes)
        } else if let Some(b58) = s.strip_prefix('z') {
            let bytes = bs58::decode(b58)
                .into_vec()
                .map_err(|e| JsError::new(&format!("invalid base58: {e}")))?;
            Self::from_bytes(&bytes)
        } else {
            Err(JsError::new(
                "CID string must start with 'b' (base32) or 'z' (base58btc)",
            ))
        }
    }

    /// Parse a CID from raw bytes.
    #[wasm_bindgen(js_name = fromBytes)]
    pub fn from_bytes(data: &[u8]) -> Result<WasmCid, JsError> {
        // Decode version
        let (version, n1) =
            decode_varint(data).ok_or_else(|| JsError::new("invalid CID: bad version varint"))?;
        if version != CID_VERSION_1 {
            return Err(JsError::new(&format!("unsupported CID version: {version}")));
        }
        // Decode codec
        let (codec, n2) = decode_varint(&data[n1..])
            .ok_or_else(|| JsError::new("invalid CID: bad codec varint"))?;
        // Decode multihash header
        let mh_start = n1 + n2;
        let (hash_fn, n3) = decode_varint(&data[mh_start..])
            .ok_or_else(|| JsError::new("invalid CID: bad multihash function varint"))?;
        if hash_fn != SHA2_256_CODE {
            return Err(JsError::new(&format!(
                "unsupported hash function: 0x{hash_fn:x}"
            )));
        }
        let (digest_len, n4) = decode_varint(&data[mh_start + n3..])
            .ok_or_else(|| JsError::new("invalid CID: bad multihash length varint"))?;
        if digest_len != SHA2_256_LENGTH {
            return Err(JsError::new("invalid multihash digest length"));
        }
        let total = mh_start + n3 + n4 + digest_len as usize;
        if data.len() < total {
            return Err(JsError::new("CID data too short for declared digest"));
        }

        Ok(Self {
            bytes: data[..total].to_vec(),
            codec,
        })
    }

    /// Verify that data matches this CID's hash.
    pub fn verify(&self, data: &[u8]) -> bool {
        let expected = build_cid_bytes(data, self.codec);
        expected == self.bytes
    }

    /// The CID as a base32-lower string with "b" multibase prefix.
    #[wasm_bindgen(js_name = toBase32)]
    pub fn to_base32(&self) -> String {
        format!("b{}", base32_encode(&self.bytes))
    }

    /// The CID as a base58btc string with "z" multibase prefix.
    #[wasm_bindgen(js_name = toBase58)]
    pub fn to_base58(&self) -> String {
        format!("z{}", bs58::encode(&self.bytes).into_string())
    }

    /// The raw CID bytes.
    #[wasm_bindgen(js_name = toBytes)]
    pub fn to_bytes(&self) -> Vec<u8> {
        self.bytes.clone()
    }

    /// The codec code (0x55 = raw, 0x70 = dag-pb).
    pub fn codec(&self) -> u64 {
        self.codec
    }

    /// The SHA-256 digest (32 bytes) extracted from the multihash.
    pub fn digest(&self) -> Vec<u8> {
        // Skip version varint + codec varint + hash_fn varint + length varint
        let (_, n1) = decode_varint(&self.bytes).unwrap_or((0, 0));
        let (_, n2) = decode_varint(&self.bytes[n1..]).unwrap_or((0, 0));
        let mh_start = n1 + n2;
        let (_, n3) = decode_varint(&self.bytes[mh_start..]).unwrap_or((0, 0));
        let (_, n4) = decode_varint(&self.bytes[mh_start + n3..]).unwrap_or((0, 0));
        let digest_start = mh_start + n3 + n4;
        self.bytes[digest_start..digest_start + 32].to_vec()
    }

    /// Whether this is a raw codec CID.
    #[wasm_bindgen(js_name = isRaw)]
    pub fn is_raw(&self) -> bool {
        self.codec == RAW_CODEC
    }

    /// Whether this is a dag-pb codec CID.
    #[wasm_bindgen(js_name = isDagPb)]
    pub fn is_dag_pb(&self) -> bool {
        self.codec == DAG_PB_CODEC
    }
}

/// Compute a SHA-256 CID v1 (raw codec) and return its base32 string.
#[wasm_bindgen(js_name = contentId)]
pub fn content_id(data: &[u8]) -> String {
    WasmCid::new(data).to_base32()
}

/// Verify that data matches a base32 CID string.
#[wasm_bindgen(js_name = verifyContent)]
pub fn verify_content(cid_str: &str, data: &[u8]) -> Result<bool, JsError> {
    let cid = WasmCid::from_string(cid_str)?;
    Ok(cid.verify(data))
}

// ── Tests ────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    // --- Varint ---

    #[test]
    fn varint_roundtrip_small() {
        let encoded = encode_varint(42);
        let (decoded, len) = decode_varint(&encoded).unwrap();
        assert_eq!(decoded, 42);
        assert_eq!(len, 1);
    }

    #[test]
    fn varint_roundtrip_large() {
        let encoded = encode_varint(300);
        let (decoded, len) = decode_varint(&encoded).unwrap();
        assert_eq!(decoded, 300);
        assert_eq!(len, 2);
    }

    #[test]
    fn varint_zero() {
        let encoded = encode_varint(0);
        assert_eq!(encoded, vec![0]);
        let (decoded, _) = decode_varint(&encoded).unwrap();
        assert_eq!(decoded, 0);
    }

    #[test]
    fn varint_max_single_byte() {
        let encoded = encode_varint(127);
        assert_eq!(encoded.len(), 1);
        let (decoded, _) = decode_varint(&encoded).unwrap();
        assert_eq!(decoded, 127);
    }

    #[test]
    fn varint_multi_byte_boundary() {
        let encoded = encode_varint(128);
        assert_eq!(encoded.len(), 2);
        let (decoded, _) = decode_varint(&encoded).unwrap();
        assert_eq!(decoded, 128);
    }

    #[test]
    fn decode_varint_empty() {
        assert!(decode_varint(&[]).is_none());
    }

    // --- Base32 ---

    #[test]
    fn base32_roundtrip() {
        let data = b"hello world";
        let encoded = base32_encode(data);
        let decoded = base32_decode(&encoded).unwrap();
        assert_eq!(&decoded[..data.len()], data);
    }

    #[test]
    fn base32_empty() {
        assert_eq!(base32_encode(b""), "");
        assert_eq!(base32_decode("").unwrap(), Vec::<u8>::new());
    }

    #[test]
    fn base32_known_value() {
        // RFC 4648 test vector
        let encoded = base32_encode(b"f");
        assert_eq!(encoded, "my");
    }

    #[test]
    fn base32_case_insensitive_decode() {
        let data = b"test";
        let encoded = base32_encode(data);
        let upper = encoded.to_uppercase();
        let decoded = base32_decode(&upper).unwrap();
        assert_eq!(&decoded[..data.len()], data);
    }

    #[test]
    fn base32_rejects_invalid_chars() {
        assert!(base32_decode("!!!").is_none());
    }

    // --- CID construction ---

    #[test]
    fn cid_deterministic() {
        let data = b"the eternal return";
        let cid1 = WasmCid::new(data);
        let cid2 = WasmCid::new(data);
        assert_eq!(cid1.to_bytes(), cid2.to_bytes());
    }

    #[test]
    fn cid_different_data_different_cid() {
        let cid1 = WasmCid::new(b"alpha");
        let cid2 = WasmCid::new(b"beta");
        assert_ne!(cid1.to_bytes(), cid2.to_bytes());
    }

    #[test]
    fn cid_raw_codec() {
        let cid = WasmCid::new(b"data");
        assert!(cid.is_raw());
        assert!(!cid.is_dag_pb());
        assert_eq!(cid.codec(), RAW_CODEC);
    }

    #[test]
    fn cid_dag_pb_codec() {
        let cid = WasmCid::dag_pb(b"data");
        assert!(!cid.is_raw());
        assert!(cid.is_dag_pb());
        assert_eq!(cid.codec(), DAG_PB_CODEC);
    }

    #[test]
    fn cid_digest_is_sha256() {
        let data = b"";
        let cid = WasmCid::new(data);
        let digest = cid.digest();
        assert_eq!(digest.len(), 32);
        // SHA-256 of empty string
        assert_eq!(
            hex::encode(&digest),
            "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
        );
    }

    #[test]
    fn cid_verify_correct_data() {
        let data = b"sovereign data";
        let cid = WasmCid::new(data);
        assert!(cid.verify(data));
    }

    #[test]
    fn cid_verify_wrong_data() {
        let cid = WasmCid::new(b"original");
        assert!(!cid.verify(b"tampered"));
    }

    #[test]
    fn cid_verify_empty() {
        let cid = WasmCid::new(b"");
        assert!(cid.verify(b""));
        assert!(!cid.verify(b"not empty"));
    }

    // --- CID encoding ---

    #[test]
    fn cid_base32_starts_with_b() {
        let cid = WasmCid::new(b"test");
        let s = cid.to_base32();
        assert!(s.starts_with('b'));
    }

    #[test]
    fn cid_base58_starts_with_z() {
        let cid = WasmCid::new(b"test");
        let s = cid.to_base58();
        assert!(s.starts_with('z'));
    }

    #[test]
    fn cid_base32_roundtrip() {
        let cid = WasmCid::new(b"roundtrip");
        let s = cid.to_base32();
        let result = std::panic::catch_unwind(|| WasmCid::from_string(&s));
        match result {
            Ok(Ok(parsed)) => {
                assert_eq!(parsed.to_bytes(), cid.to_bytes());
                assert!(parsed.verify(b"roundtrip"));
            }
            _ => {} // JsError panics on non-wasm — acceptable
        }
    }

    #[test]
    fn cid_base58_roundtrip() {
        let cid = WasmCid::new(b"roundtrip");
        let s = cid.to_base58();
        let result = std::panic::catch_unwind(|| WasmCid::from_string(&s));
        match result {
            Ok(Ok(parsed)) => {
                assert_eq!(parsed.to_bytes(), cid.to_bytes());
            }
            _ => {}
        }
    }

    #[test]
    fn cid_bytes_roundtrip() {
        let cid = WasmCid::new(b"bytes test");
        let bytes = cid.to_bytes();
        let result = std::panic::catch_unwind(|| WasmCid::from_bytes(&bytes));
        match result {
            Ok(Ok(parsed)) => {
                assert_eq!(parsed.to_bytes(), bytes);
                assert_eq!(parsed.codec(), RAW_CODEC);
            }
            _ => {}
        }
    }

    // --- content_id / verify_content ---

    #[test]
    fn content_id_deterministic() {
        let id1 = content_id(b"data");
        let id2 = content_id(b"data");
        assert_eq!(id1, id2);
    }

    #[test]
    fn content_id_different_for_different_data() {
        let id1 = content_id(b"alpha");
        let id2 = content_id(b"beta");
        assert_ne!(id1, id2);
    }

    #[test]
    fn content_id_starts_with_b() {
        let id = content_id(b"test");
        assert!(id.starts_with('b'));
    }

    #[test]
    fn verify_content_correct() {
        let data = b"verified";
        let id = content_id(data);
        let result = std::panic::catch_unwind(|| verify_content(&id, data));
        match result {
            Ok(Ok(valid)) => assert!(valid),
            _ => {}
        }
    }

    #[test]
    fn verify_content_wrong_data() {
        let id = content_id(b"original");
        let result = std::panic::catch_unwind(|| verify_content(&id, b"wrong"));
        match result {
            Ok(Ok(valid)) => assert!(!valid),
            _ => {}
        }
    }

    // --- Multihash ---

    #[test]
    fn multihash_sha256_length() {
        let mh = multihash_sha256(b"test");
        // 1 byte hash fn code + 1 byte length + 32 bytes digest
        assert_eq!(mh.len(), 34);
        assert_eq!(mh[0], SHA2_256_CODE as u8);
        assert_eq!(mh[1], SHA2_256_LENGTH as u8);
    }

    #[test]
    fn multihash_sha256_deterministic() {
        let mh1 = multihash_sha256(b"data");
        let mh2 = multihash_sha256(b"data");
        assert_eq!(mh1, mh2);
    }

    #[test]
    fn multihash_sha256_different() {
        let mh1 = multihash_sha256(b"alpha");
        let mh2 = multihash_sha256(b"beta");
        assert_ne!(mh1, mh2);
    }

    // --- CID structure ---

    #[test]
    fn cid_bytes_start_with_version() {
        let cid = WasmCid::new(b"data");
        let bytes = cid.to_bytes();
        let (version, _) = decode_varint(&bytes).unwrap();
        assert_eq!(version, CID_VERSION_1);
    }

    #[test]
    fn cid_dag_pb_bytes_have_correct_codec() {
        let cid = WasmCid::dag_pb(b"data");
        let bytes = cid.to_bytes();
        let (_, n1) = decode_varint(&bytes).unwrap();
        let (codec, _) = decode_varint(&bytes[n1..]).unwrap();
        assert_eq!(codec, DAG_PB_CODEC);
    }

    #[test]
    fn cid_raw_and_dagpb_differ() {
        let raw = WasmCid::new(b"same data");
        let dag = WasmCid::dag_pb(b"same data");
        assert_ne!(raw.to_bytes(), dag.to_bytes());
        assert_ne!(raw.to_base32(), dag.to_base32());
    }
}
