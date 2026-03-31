//! WebAssembly bindings for Nous primitives.
//!
//! Exposes identity generation, signing, verification, encryption,
//! DID operations, zero-knowledge proofs, and CRDTs to JavaScript.

pub mod content;
pub mod crdt;

use aes_gcm::aead::{Aead, KeyInit};
use aes_gcm::{Aes256Gcm, Nonce};
use curve25519_dalek::constants::RISTRETTO_BASEPOINT_POINT;
use curve25519_dalek::ristretto::CompressedRistretto;
use curve25519_dalek::scalar::Scalar;
use ed25519_dalek::{Signer as DalekSigner, Verifier as DalekVerifier};
use hkdf::Hkdf;
use rand::RngCore;
use rand::rngs::OsRng;
use sha2::{Digest, Sha256, Sha512};
use wasm_bindgen::prelude::*;
use zeroize::Zeroize;

// ── Identity ──────────────────────────────────────────────────────

/// A self-sovereign identity backed by Ed25519 + X25519 keys.
///
/// Holds signing (Ed25519) and key-exchange (X25519) key pairs,
/// with the DID derived from the signing public key.
#[wasm_bindgen]
pub struct WasmIdentity {
    signing_secret: [u8; 32],
    signing_public: [u8; 32],
    exchange_secret: [u8; 32],
    exchange_public: [u8; 32],
    did: String,
}

#[wasm_bindgen]
impl WasmIdentity {
    /// Generate a new random identity.
    #[wasm_bindgen(constructor)]
    #[allow(clippy::new_without_default)]
    pub fn new() -> Self {
        let signing_key = ed25519_dalek::SigningKey::generate(&mut OsRng);
        let verifying_key = signing_key.verifying_key();
        let exchange_secret = x25519_dalek::StaticSecret::random_from_rng(OsRng);
        let exchange_public = x25519_dalek::PublicKey::from(&exchange_secret);

        let did = public_key_to_did(&verifying_key.to_bytes());

        Self {
            signing_secret: signing_key.to_bytes(),
            signing_public: verifying_key.to_bytes(),
            exchange_secret: exchange_secret.to_bytes(),
            exchange_public: exchange_public.to_bytes(),
            did,
        }
    }

    /// Restore an identity from a 32-byte Ed25519 signing key.
    #[wasm_bindgen(js_name = fromSigningKey)]
    pub fn from_signing_key(secret_bytes: &[u8]) -> Result<WasmIdentity, JsError> {
        if secret_bytes.len() != 32 {
            return Err(JsError::new("signing key must be 32 bytes"));
        }
        let mut bytes = [0u8; 32];
        bytes.copy_from_slice(secret_bytes);
        let signing_key = ed25519_dalek::SigningKey::from_bytes(&bytes);
        let verifying_key = signing_key.verifying_key();
        let exchange_secret = x25519_dalek::StaticSecret::random_from_rng(OsRng);
        let exchange_public = x25519_dalek::PublicKey::from(&exchange_secret);
        let did = public_key_to_did(&verifying_key.to_bytes());

        Ok(Self {
            signing_secret: signing_key.to_bytes(),
            signing_public: verifying_key.to_bytes(),
            exchange_secret: exchange_secret.to_bytes(),
            exchange_public: exchange_public.to_bytes(),
            did,
        })
    }

    /// The DID:key identifier for this identity.
    #[wasm_bindgen(getter)]
    pub fn did(&self) -> String {
        self.did.clone()
    }

    /// The 32-byte Ed25519 signing public key.
    #[wasm_bindgen(js_name = signingPublicKey)]
    pub fn signing_public_key(&self) -> Vec<u8> {
        self.signing_public.to_vec()
    }

    /// The 32-byte X25519 exchange public key.
    #[wasm_bindgen(js_name = exchangePublicKey)]
    pub fn exchange_public_key(&self) -> Vec<u8> {
        self.exchange_public.to_vec()
    }

    /// Export the 32-byte signing secret key (handle with care).
    #[wasm_bindgen(js_name = exportSigningKey)]
    pub fn export_signing_key(&self) -> Vec<u8> {
        self.signing_secret.to_vec()
    }

    /// Sign a message, returning a 64-byte Ed25519 signature.
    pub fn sign(&self, message: &[u8]) -> Vec<u8> {
        let signing_key = ed25519_dalek::SigningKey::from_bytes(&self.signing_secret);
        let sig = signing_key.sign(message);
        sig.to_bytes().to_vec()
    }

    /// Verify a signature against this identity's public key.
    pub fn verify(&self, message: &[u8], signature: &[u8]) -> Result<bool, JsError> {
        let verifying_key = ed25519_dalek::VerifyingKey::from_bytes(&self.signing_public)
            .map_err(|e| JsError::new(&format!("invalid public key: {e}")))?;
        let sig_bytes: [u8; 64] = signature
            .try_into()
            .map_err(|_| JsError::new("signature must be 64 bytes"))?;
        let sig = ed25519_dalek::Signature::from_bytes(&sig_bytes);
        Ok(verifying_key.verify(message, &sig).is_ok())
    }

    /// Compute a shared secret with another party's X25519 public key.
    #[wasm_bindgen(js_name = keyExchange)]
    pub fn key_exchange(&self, their_public: &[u8]) -> Result<Vec<u8>, JsError> {
        if their_public.len() != 32 {
            return Err(JsError::new("public key must be 32 bytes"));
        }
        let mut their_bytes = [0u8; 32];
        their_bytes.copy_from_slice(their_public);
        let their_pk = x25519_dalek::PublicKey::from(their_bytes);
        let our_secret = x25519_dalek::StaticSecret::from(self.exchange_secret);
        let shared = our_secret.diffie_hellman(&their_pk);
        Ok(shared.as_bytes().to_vec())
    }

    /// Get the DID Document as a JSON string.
    #[wasm_bindgen(js_name = didDocument)]
    pub fn did_document(&self) -> String {
        let signing_multibase = format!("z{}", bs58::encode(&self.signing_public).into_string());
        let exchange_multibase = format!("z{}", bs58::encode(&self.exchange_public).into_string());
        #[cfg(target_arch = "wasm32")]
        let now = js_sys::Date::new_0()
            .to_iso_string()
            .as_string()
            .unwrap_or_default();
        #[cfg(not(target_arch = "wasm32"))]
        let now = {
            use std::time::SystemTime;
            let d = SystemTime::now()
                .duration_since(SystemTime::UNIX_EPOCH)
                .unwrap_or_default();
            format!("{}Z", d.as_secs())
        };

        serde_json::json!({
            "@context": [
                "https://www.w3.org/ns/did/v1",
                "https://w3id.org/security/suites/ed25519-2020/v1",
                "https://w3id.org/security/suites/x25519-2020/v1"
            ],
            "id": self.did,
            "verificationMethod": [
                {
                    "id": format!("{}#signing", self.did),
                    "type": "Ed25519VerificationKey2020",
                    "controller": self.did,
                    "publicKeyMultibase": signing_multibase
                },
                {
                    "id": format!("{}#exchange", self.did),
                    "type": "X25519KeyAgreementKey2020",
                    "controller": self.did,
                    "publicKeyMultibase": exchange_multibase
                }
            ],
            "authentication": [format!("{}#signing", self.did)],
            "keyAgreement": [format!("{}#exchange", self.did)],
            "created": now,
            "updated": now
        })
        .to_string()
    }
}

impl Drop for WasmIdentity {
    fn drop(&mut self) {
        self.signing_secret.zeroize();
        self.exchange_secret.zeroize();
    }
}

// ── Standalone verification ───────────────────────────────────────

/// Verify an Ed25519 signature given raw public key bytes, message, and signature.
#[wasm_bindgen(js_name = verifySignature)]
pub fn verify_signature(
    public_key: &[u8],
    message: &[u8],
    signature: &[u8],
) -> Result<bool, JsError> {
    let pk_bytes: [u8; 32] = public_key
        .try_into()
        .map_err(|_| JsError::new("public key must be 32 bytes"))?;
    let verifying_key = ed25519_dalek::VerifyingKey::from_bytes(&pk_bytes)
        .map_err(|e| JsError::new(&format!("invalid public key: {e}")))?;
    let sig_bytes: [u8; 64] = signature
        .try_into()
        .map_err(|_| JsError::new("signature must be 64 bytes"))?;
    let sig = ed25519_dalek::Signature::from_bytes(&sig_bytes);
    Ok(verifying_key.verify(message, &sig).is_ok())
}

// ── Encryption ────────────────────────────────────────────────────

/// AES-256-GCM encrypted payload.
#[wasm_bindgen]
pub struct WasmEncrypted {
    nonce: Vec<u8>,
    ciphertext: Vec<u8>,
}

#[wasm_bindgen]
impl WasmEncrypted {
    /// The 12-byte nonce.
    #[wasm_bindgen(getter)]
    pub fn nonce(&self) -> Vec<u8> {
        self.nonce.clone()
    }

    /// The ciphertext (with appended GCM tag).
    #[wasm_bindgen(getter)]
    pub fn ciphertext(&self) -> Vec<u8> {
        self.ciphertext.clone()
    }

    /// Serialize to a JSON string for transport.
    #[wasm_bindgen(js_name = toJson)]
    pub fn to_json(&self) -> String {
        serde_json::json!({
            "nonce": base64::Engine::encode(&base64::engine::general_purpose::STANDARD, &self.nonce),
            "ciphertext": base64::Engine::encode(&base64::engine::general_purpose::STANDARD, &self.ciphertext),
        })
        .to_string()
    }

    /// Deserialize from a JSON string.
    #[wasm_bindgen(js_name = fromJson)]
    pub fn from_json(json: &str) -> Result<WasmEncrypted, JsError> {
        let v: serde_json::Value =
            serde_json::from_str(json).map_err(|e| JsError::new(&format!("invalid JSON: {e}")))?;
        let nonce = base64::Engine::decode(
            &base64::engine::general_purpose::STANDARD,
            v["nonce"]
                .as_str()
                .ok_or_else(|| JsError::new("missing nonce"))?,
        )
        .map_err(|e| JsError::new(&format!("invalid nonce base64: {e}")))?;
        let ciphertext = base64::Engine::decode(
            &base64::engine::general_purpose::STANDARD,
            v["ciphertext"]
                .as_str()
                .ok_or_else(|| JsError::new("missing ciphertext"))?,
        )
        .map_err(|e| JsError::new(&format!("invalid ciphertext base64: {e}")))?;
        Ok(Self { nonce, ciphertext })
    }
}

/// Encrypt plaintext with a 32-byte AES-256-GCM key.
#[wasm_bindgen]
pub fn encrypt(key: &[u8], plaintext: &[u8]) -> Result<WasmEncrypted, JsError> {
    if key.len() != 32 {
        return Err(JsError::new("key must be 32 bytes"));
    }
    let cipher = Aes256Gcm::new_from_slice(key)
        .map_err(|e| JsError::new(&format!("cipher init failed: {e}")))?;
    let mut nonce_bytes = [0u8; 12];
    OsRng.fill_bytes(&mut nonce_bytes);
    let nonce = Nonce::from_slice(&nonce_bytes);
    let ciphertext = cipher
        .encrypt(nonce, plaintext)
        .map_err(|e| JsError::new(&format!("encryption failed: {e}")))?;
    Ok(WasmEncrypted {
        nonce: nonce_bytes.to_vec(),
        ciphertext,
    })
}

/// Decrypt ciphertext with a 32-byte AES-256-GCM key.
#[wasm_bindgen]
pub fn decrypt(key: &[u8], encrypted: &WasmEncrypted) -> Result<Vec<u8>, JsError> {
    if key.len() != 32 {
        return Err(JsError::new("key must be 32 bytes"));
    }
    let cipher = Aes256Gcm::new_from_slice(key)
        .map_err(|e| JsError::new(&format!("cipher init failed: {e}")))?;
    let nonce_bytes: [u8; 12] = encrypted
        .nonce
        .as_slice()
        .try_into()
        .map_err(|_| JsError::new("nonce must be 12 bytes"))?;
    let nonce = Nonce::from_slice(&nonce_bytes);
    cipher
        .decrypt(nonce, encrypted.ciphertext.as_ref())
        .map_err(|e| JsError::new(&format!("decryption failed: {e}")))
}

/// Derive a 32-byte key from a shared secret and context using HKDF-SHA256.
#[wasm_bindgen(js_name = deriveKey)]
pub fn derive_key(shared_secret: &[u8], context: &[u8]) -> Result<Vec<u8>, JsError> {
    if shared_secret.len() != 32 {
        return Err(JsError::new("shared secret must be 32 bytes"));
    }
    let hk = Hkdf::<Sha256>::new(None, shared_secret);
    let mut key = [0u8; 32];
    hk.expand(context, &mut key)
        .map_err(|e| JsError::new(&format!("HKDF expand failed: {e}")))?;
    Ok(key.to_vec())
}

// ── Hashing ───────────────────────────────────────────────────────

/// SHA-256 hash of arbitrary data.
#[wasm_bindgen(js_name = sha256)]
pub fn sha256_hash(data: &[u8]) -> Vec<u8> {
    Sha256::digest(data).to_vec()
}

// ── DID Utilities ─────────────────────────────────────────────────

/// Convert a 32-byte Ed25519 public key to a DID:key string.
#[wasm_bindgen(js_name = publicKeyToDid)]
pub fn public_key_to_did_js(public_key: &[u8]) -> Result<String, JsError> {
    if public_key.len() != 32 {
        return Err(JsError::new("public key must be 32 bytes"));
    }
    let mut bytes = [0u8; 32];
    bytes.copy_from_slice(public_key);
    Ok(public_key_to_did(&bytes))
}

/// Extract the 32-byte Ed25519 public key from a DID:key string.
#[wasm_bindgen(js_name = didToPublicKey)]
pub fn did_to_public_key_js(did: &str) -> Result<Vec<u8>, JsError> {
    let z_part = did
        .strip_prefix("did:key:z")
        .ok_or_else(|| JsError::new("invalid DID:key format"))?;
    let decoded = bs58::decode(z_part)
        .into_vec()
        .map_err(|e| JsError::new(&format!("base58 decode failed: {e}")))?;
    if decoded.len() < 34 || decoded[0] != 0xed || decoded[1] != 0x01 {
        return Err(JsError::new("invalid multicodec prefix for ed25519"));
    }
    Ok(decoded[2..34].to_vec())
}

// ── Schnorr Proofs ────────────────────────────────────────────────

/// Non-interactive Schnorr proof of knowledge of a discrete logarithm.
#[wasm_bindgen]
pub struct WasmSchnorrProof {
    commitment: [u8; 32],
    response: [u8; 32],
}

fn challenge_scalar(data: &[u8]) -> Scalar {
    let hash: [u8; 64] = Sha512::digest(data).into();
    Scalar::from_bytes_mod_order_wide(&hash)
}

#[wasm_bindgen]
impl WasmSchnorrProof {
    /// Generate a Schnorr proof-of-knowledge for the given secret/public pair.
    pub fn prove(
        secret: &[u8],
        public: &[u8],
        message: &[u8],
    ) -> Result<WasmSchnorrProof, JsError> {
        if secret.len() != 32 || public.len() != 32 {
            return Err(JsError::new("secret and public must be 32 bytes each"));
        }
        let mut sec = [0u8; 32];
        let mut pub_bytes = [0u8; 32];
        sec.copy_from_slice(secret);
        pub_bytes.copy_from_slice(public);

        let x = Scalar::from_bytes_mod_order(sec);
        let k = Scalar::random(&mut OsRng);
        let r = (k * RISTRETTO_BASEPOINT_POINT).compress().to_bytes();

        let mut input = Vec::with_capacity(64 + message.len());
        input.extend_from_slice(&r);
        input.extend_from_slice(&pub_bytes);
        input.extend_from_slice(message);
        let c = challenge_scalar(&input);
        let s = k + c * x;

        Ok(Self {
            commitment: r,
            response: s.to_bytes(),
        })
    }

    /// Verify a Schnorr proof against a public key and message.
    pub fn verify(&self, public: &[u8], message: &[u8]) -> Result<bool, JsError> {
        if public.len() != 32 {
            return Err(JsError::new("public key must be 32 bytes"));
        }
        let mut pub_bytes = [0u8; 32];
        pub_bytes.copy_from_slice(public);

        let y = match CompressedRistretto(pub_bytes).decompress() {
            Some(p) => p,
            None => return Ok(false),
        };
        let r = match CompressedRistretto(self.commitment).decompress() {
            Some(p) => p,
            None => return Ok(false),
        };
        let s = Scalar::from_bytes_mod_order(self.response);

        let mut input = Vec::with_capacity(64 + message.len());
        input.extend_from_slice(&self.commitment);
        input.extend_from_slice(&pub_bytes);
        input.extend_from_slice(message);
        let c = challenge_scalar(&input);

        Ok(s * RISTRETTO_BASEPOINT_POINT == r + c * y)
    }

    /// The 32-byte commitment (R point).
    #[wasm_bindgen(getter)]
    pub fn commitment(&self) -> Vec<u8> {
        self.commitment.to_vec()
    }

    /// The 32-byte response scalar.
    #[wasm_bindgen(getter)]
    pub fn response(&self) -> Vec<u8> {
        self.response.to_vec()
    }
}

/// Generate a random Ristretto keypair for Schnorr proofs.
/// Returns `[secret_32_bytes, public_32_bytes]` concatenated (64 bytes).
#[wasm_bindgen(js_name = schnorrKeygen)]
pub fn schnorr_keygen() -> Vec<u8> {
    let secret = Scalar::random(&mut OsRng);
    let public = (secret * RISTRETTO_BASEPOINT_POINT).compress().to_bytes();
    let mut out = Vec::with_capacity(64);
    out.extend_from_slice(&secret.to_bytes());
    out.extend_from_slice(&public);
    out
}

// ── Internal helpers ──────────────────────────────────────────────

fn public_key_to_did(verifying_key_bytes: &[u8; 32]) -> String {
    let mut multicodec = vec![0xed, 0x01];
    multicodec.extend_from_slice(verifying_key_bytes);
    format!("did:key:z{}", bs58::encode(&multicodec).into_string())
}

// ── Native tests (run with `cargo test`) ──────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn identity_generate_has_valid_did() {
        let id = WasmIdentity::new();
        assert!(id.did().starts_with("did:key:z"));
    }

    #[test]
    fn identity_sign_and_verify() {
        let id = WasmIdentity::new();
        let msg = b"the will to power";
        let sig = id.sign(msg);
        assert_eq!(sig.len(), 64);
        assert!(id.verify(msg, &sig).unwrap());
    }

    #[test]
    fn identity_verify_rejects_wrong_message() {
        let id = WasmIdentity::new();
        let sig = id.sign(b"original");
        assert!(!id.verify(b"tampered", &sig).unwrap());
    }

    #[test]
    fn identity_restore_roundtrip() {
        let id = WasmIdentity::new();
        let did = id.did();
        let key = id.export_signing_key();
        let restored = WasmIdentity::from_signing_key(&key).unwrap();
        assert_eq!(restored.did(), did);
    }

    #[test]
    fn identity_restored_can_sign() {
        let original = WasmIdentity::new();
        let key = original.export_signing_key();
        let restored = WasmIdentity::from_signing_key(&key).unwrap();
        let msg = b"persistence";
        let sig = restored.sign(msg);
        assert!(original.verify(msg, &sig).unwrap());
    }

    #[test]
    fn identity_key_exchange_symmetric() {
        let alice = WasmIdentity::new();
        let bob = WasmIdentity::new();
        let alice_shared = alice.key_exchange(&bob.exchange_public_key()).unwrap();
        let bob_shared = bob.key_exchange(&alice.exchange_public_key()).unwrap();
        assert_eq!(alice_shared, bob_shared);
    }

    #[test]
    fn identity_did_document_is_valid_json() {
        let id = WasmIdentity::new();
        let doc = id.did_document();
        let v: serde_json::Value = serde_json::from_str(&doc).unwrap();
        assert!(v["id"].as_str().unwrap().starts_with("did:key:z"));
        assert_eq!(v["verificationMethod"].as_array().unwrap().len(), 2);
    }

    #[test]
    fn standalone_verify_signature() {
        let id = WasmIdentity::new();
        let msg = b"standalone test";
        let sig = id.sign(msg);
        let pk = id.signing_public_key();
        assert!(verify_signature(&pk, msg, &sig).unwrap());
    }

    #[test]
    fn standalone_verify_rejects_wrong_key() {
        let id1 = WasmIdentity::new();
        let id2 = WasmIdentity::new();
        let sig = id1.sign(b"test");
        assert!(!verify_signature(&id2.signing_public_key(), b"test", &sig).unwrap());
    }

    #[test]
    fn encrypt_decrypt_roundtrip() {
        let mut key = [0u8; 32];
        OsRng.fill_bytes(&mut key);
        let plaintext = b"amor fati";
        let encrypted = encrypt(&key, plaintext).unwrap();
        let decrypted = decrypt(&key, &encrypted).unwrap();
        assert_eq!(decrypted, plaintext);
    }

    #[test]
    fn encrypt_unique_nonces() {
        let mut key = [0u8; 32];
        OsRng.fill_bytes(&mut key);
        let a = encrypt(&key, b"same").unwrap();
        let b = encrypt(&key, b"same").unwrap();
        assert_ne!(a.nonce, b.nonce);
    }

    #[test]
    fn decrypt_rejects_wrong_key() {
        let mut key1 = [0u8; 32];
        let mut key2 = [0u8; 32];
        OsRng.fill_bytes(&mut key1);
        OsRng.fill_bytes(&mut key2);
        let encrypted = encrypt(&key1, b"secret").unwrap();
        // On native, JsError triggers a panic, so we catch it
        let result = std::panic::catch_unwind(|| decrypt(&key2, &encrypted));
        assert!(result.is_err() || result.unwrap().is_err());
    }

    #[test]
    fn encrypted_json_roundtrip() {
        let mut key = [0u8; 32];
        OsRng.fill_bytes(&mut key);
        let encrypted = encrypt(&key, b"json test").unwrap();
        let json = encrypted.to_json();
        let restored = WasmEncrypted::from_json(&json).unwrap();
        let decrypted = decrypt(&key, &restored).unwrap();
        assert_eq!(decrypted, b"json test");
    }

    #[test]
    fn derive_key_deterministic() {
        let mut secret = [0u8; 32];
        OsRng.fill_bytes(&mut secret);
        let k1 = derive_key(&secret, b"context-a").unwrap();
        let k2 = derive_key(&secret, b"context-a").unwrap();
        assert_eq!(k1, k2);
    }

    #[test]
    fn derive_key_different_contexts() {
        let mut secret = [0u8; 32];
        OsRng.fill_bytes(&mut secret);
        let k1 = derive_key(&secret, b"context-a").unwrap();
        let k2 = derive_key(&secret, b"context-b").unwrap();
        assert_ne!(k1, k2);
    }

    #[test]
    fn sha256_known_value() {
        let hash = sha256_hash(b"");
        assert_eq!(hash.len(), 32);
        assert_eq!(
            hex::encode(&hash),
            "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
        );
    }

    #[test]
    fn did_roundtrip() {
        let id = WasmIdentity::new();
        let pk = id.signing_public_key();
        let did = public_key_to_did_js(&pk).unwrap();
        assert_eq!(did, id.did());
        let recovered = did_to_public_key_js(&did).unwrap();
        assert_eq!(recovered, pk);
    }

    #[test]
    fn did_rejects_invalid_prefix() {
        let result = std::panic::catch_unwind(|| did_to_public_key_js("did:web:example.com"));
        assert!(result.is_err() || result.unwrap().is_err());
    }

    #[test]
    fn schnorr_prove_and_verify() {
        let keys = schnorr_keygen();
        let secret = &keys[..32];
        let public = &keys[32..];
        let proof = WasmSchnorrProof::prove(secret, public, b"test").unwrap();
        assert!(proof.verify(public, b"test").unwrap());
    }

    #[test]
    fn schnorr_rejects_wrong_message() {
        let keys = schnorr_keygen();
        let secret = &keys[..32];
        let public = &keys[32..];
        let proof = WasmSchnorrProof::prove(secret, public, b"original").unwrap();
        assert!(!proof.verify(public, b"tampered").unwrap());
    }

    #[test]
    fn schnorr_rejects_wrong_key() {
        let keys1 = schnorr_keygen();
        let keys2 = schnorr_keygen();
        let proof = WasmSchnorrProof::prove(&keys1[..32], &keys1[32..], b"test").unwrap();
        assert!(!proof.verify(&keys2[32..], b"test").unwrap());
    }

    // Error-path tests use JsError which only works on wasm targets.
    // On native, we test the same logic through the successful paths above.
    #[cfg(target_arch = "wasm32")]
    mod wasm_error_tests {
        use super::*;
        use wasm_bindgen_test::*;

        #[wasm_bindgen_test]
        fn identity_rejects_wrong_key_length() {
            assert!(WasmIdentity::from_signing_key(&[0u8; 16]).is_err());
        }

        #[wasm_bindgen_test]
        fn verify_rejects_wrong_sig_length() {
            let id = WasmIdentity::new();
            assert!(id.verify(b"test", &[0u8; 32]).is_err());
        }

        #[wasm_bindgen_test]
        fn encrypt_rejects_wrong_key_length() {
            assert!(encrypt(&[0u8; 16], b"test").is_err());
        }

        #[wasm_bindgen_test]
        fn key_exchange_rejects_wrong_length() {
            let id = WasmIdentity::new();
            assert!(id.key_exchange(&[0u8; 16]).is_err());
        }
    }
}
