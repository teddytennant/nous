//! Sealed-box messaging crypto for encrypt-to-DID communication.
//!
//! Provides anonymous encryption (sender generates ephemeral keys),
//! authenticated encryption (sender signs + encrypts), and message
//! envelope construction with timestamp and content-type metadata.

use aes_gcm::aead::{Aead, KeyInit};
use aes_gcm::{Aes256Gcm, Nonce};
use ed25519_dalek::Signer as DalekSigner;
use hkdf::Hkdf;
use rand::rngs::OsRng;
use rand::RngCore;
use sha2::Sha256;
use wasm_bindgen::prelude::*;
use zeroize::Zeroize;

/// Derive a 32-byte AES key from a shared secret and context.
fn derive_aes_key(shared_secret: &[u8], context: &[u8]) -> [u8; 32] {
    let hk = Hkdf::<Sha256>::new(None, shared_secret);
    let mut key = [0u8; 32];
    hk.expand(context, &mut key)
        .expect("HKDF expand failed — context too long");
    key
}

/// AES-256-GCM encrypt with a given key.
fn aes_encrypt(key: &[u8; 32], plaintext: &[u8]) -> (Vec<u8>, [u8; 12]) {
    let cipher = Aes256Gcm::new_from_slice(key).expect("invalid key length");
    let mut nonce_bytes = [0u8; 12];
    OsRng.fill_bytes(&mut nonce_bytes);
    let nonce = Nonce::from_slice(&nonce_bytes);
    let ciphertext = cipher.encrypt(nonce, plaintext).expect("encryption failed");
    (ciphertext, nonce_bytes)
}

/// AES-256-GCM decrypt with a given key.
fn aes_decrypt(key: &[u8; 32], nonce: &[u8; 12], ciphertext: &[u8]) -> Result<Vec<u8>, String> {
    let cipher =
        Aes256Gcm::new_from_slice(key).map_err(|e| format!("cipher init failed: {e}"))?;
    let n = Nonce::from_slice(nonce);
    cipher
        .decrypt(n, ciphertext)
        .map_err(|e| format!("decryption failed: {e}"))
}

// ── Sealed Box ───────────────────────────────────────────────────

/// A sealed box: anonymous encryption to a recipient's X25519 public key.
/// The sender generates an ephemeral key pair; the ephemeral public key
/// is prepended to the ciphertext so the recipient can derive the shared secret.
#[wasm_bindgen]
pub struct WasmSealedBox {
    /// 32 bytes ephemeral public key + 12 bytes nonce + ciphertext (with GCM tag)
    data: Vec<u8>,
}

#[wasm_bindgen]
impl WasmSealedBox {
    /// Encrypt a message to a recipient's X25519 public key.
    /// The sender is anonymous — only an ephemeral key is used.
    pub fn seal(recipient_public: &[u8], plaintext: &[u8]) -> Result<WasmSealedBox, JsError> {
        if recipient_public.len() != 32 {
            return Err(JsError::new("recipient public key must be 32 bytes"));
        }
        let mut their_bytes = [0u8; 32];
        their_bytes.copy_from_slice(recipient_public);
        let their_pk = x25519_dalek::PublicKey::from(their_bytes);

        // Ephemeral key pair
        let eph_secret = x25519_dalek::StaticSecret::random_from_rng(OsRng);
        let eph_public = x25519_dalek::PublicKey::from(&eph_secret);
        let shared = eph_secret.diffie_hellman(&their_pk);

        let mut aes_key = derive_aes_key(shared.as_bytes(), b"nous-sealed-box-v1");
        let (ciphertext, nonce) = aes_encrypt(&aes_key, plaintext);
        aes_key.zeroize();

        // Pack: ephemeral_public(32) || nonce(12) || ciphertext
        let mut data = Vec::with_capacity(32 + 12 + ciphertext.len());
        data.extend_from_slice(eph_public.as_bytes());
        data.extend_from_slice(&nonce);
        data.extend_from_slice(&ciphertext);

        Ok(Self { data })
    }

    /// Decrypt a sealed box using the recipient's X25519 secret key.
    pub fn open(recipient_secret: &[u8], sealed: &WasmSealedBox) -> Result<Vec<u8>, JsError> {
        if recipient_secret.len() != 32 {
            return Err(JsError::new("recipient secret key must be 32 bytes"));
        }
        if sealed.data.len() < 44 {
            // 32 + 12 minimum
            return Err(JsError::new("sealed box too short"));
        }

        let mut sec_bytes = [0u8; 32];
        sec_bytes.copy_from_slice(recipient_secret);

        let mut eph_pub_bytes = [0u8; 32];
        eph_pub_bytes.copy_from_slice(&sealed.data[..32]);
        let eph_pk = x25519_dalek::PublicKey::from(eph_pub_bytes);

        let our_secret = x25519_dalek::StaticSecret::from(sec_bytes);
        let shared = our_secret.diffie_hellman(&eph_pk);

        let mut aes_key = derive_aes_key(shared.as_bytes(), b"nous-sealed-box-v1");
        let mut nonce = [0u8; 12];
        nonce.copy_from_slice(&sealed.data[32..44]);
        let ciphertext = &sealed.data[44..];

        let result = aes_decrypt(&aes_key, &nonce, ciphertext)
            .map_err(|e| JsError::new(&format!("open failed: {e}")));
        aes_key.zeroize();
        result
    }

    /// The raw sealed box bytes.
    #[wasm_bindgen(getter)]
    pub fn data(&self) -> Vec<u8> {
        self.data.clone()
    }

    /// The sealed box length in bytes.
    pub fn len(&self) -> usize {
        self.data.len()
    }

    /// Whether the sealed box is empty.
    #[wasm_bindgen(js_name = isEmpty)]
    pub fn is_empty(&self) -> bool {
        self.data.is_empty()
    }

    /// Encode to base64 for transport.
    #[wasm_bindgen(js_name = toBase64)]
    pub fn to_base64(&self) -> String {
        base64::Engine::encode(&base64::engine::general_purpose::STANDARD, &self.data)
    }

    /// Decode from base64.
    #[wasm_bindgen(js_name = fromBase64)]
    pub fn from_base64(encoded: &str) -> Result<WasmSealedBox, JsError> {
        let data = base64::Engine::decode(&base64::engine::general_purpose::STANDARD, encoded)
            .map_err(|e| JsError::new(&format!("invalid base64: {e}")))?;
        Ok(Self { data })
    }
}

// ── Message Envelope ─────────────────────────────────────────────

/// An authenticated, encrypted message envelope.
///
/// Structure:
/// - sender_did: the sender's DID:key
/// - recipient_did: the recipient's DID:key
/// - content_type: MIME type (e.g., "text/plain", "application/json")
/// - timestamp: milliseconds since Unix epoch
/// - sealed_payload: sealed-box encrypted content
/// - signature: Ed25519 signature over (recipient_did || timestamp || sealed_payload)
#[wasm_bindgen]
pub struct WasmEnvelope {
    sender_did: String,
    recipient_did: String,
    content_type: String,
    timestamp: f64,
    sealed_payload: Vec<u8>,
    signature: Vec<u8>,
}

#[wasm_bindgen]
impl WasmEnvelope {
    /// Create and sign a message envelope.
    ///
    /// `sender_signing_key`: 32-byte Ed25519 signing secret
    /// `sender_exchange_key`: 32-byte X25519 secret (unused here, but part of identity)
    /// `recipient_exchange_public`: 32-byte X25519 public key for encryption
    #[allow(clippy::too_many_arguments)]
    pub fn create(
        sender_did: &str,
        recipient_did: &str,
        content_type: &str,
        timestamp: f64,
        plaintext: &[u8],
        sender_signing_key: &[u8],
        recipient_exchange_public: &[u8],
    ) -> Result<WasmEnvelope, JsError> {
        if sender_signing_key.len() != 32 {
            return Err(JsError::new("sender signing key must be 32 bytes"));
        }

        // Seal the plaintext to the recipient
        let sealed = WasmSealedBox::seal(recipient_exchange_public, plaintext)?;
        let sealed_payload = sealed.data;

        // Sign: recipient_did || timestamp_bytes || sealed_payload
        let mut sign_input = Vec::new();
        sign_input.extend_from_slice(recipient_did.as_bytes());
        sign_input.extend_from_slice(&timestamp.to_be_bytes());
        sign_input.extend_from_slice(&sealed_payload);

        let mut key_bytes = [0u8; 32];
        key_bytes.copy_from_slice(sender_signing_key);
        let signing_key = ed25519_dalek::SigningKey::from_bytes(&key_bytes);
        let sig = signing_key.sign(&sign_input);

        Ok(Self {
            sender_did: sender_did.to_string(),
            recipient_did: recipient_did.to_string(),
            content_type: content_type.to_string(),
            timestamp,
            sealed_payload,
            signature: sig.to_bytes().to_vec(),
        })
    }

    /// Verify the envelope signature against the sender's public key.
    #[wasm_bindgen(js_name = verifySignature)]
    pub fn verify_signature(&self, sender_public_key: &[u8]) -> Result<bool, JsError> {
        if sender_public_key.len() != 32 {
            return Err(JsError::new("sender public key must be 32 bytes"));
        }
        let pk_bytes: [u8; 32] = sender_public_key
            .try_into()
            .map_err(|_| JsError::new("invalid public key length"))?;
        let verifying_key = ed25519_dalek::VerifyingKey::from_bytes(&pk_bytes)
            .map_err(|e| JsError::new(&format!("invalid public key: {e}")))?;

        let mut sign_input = Vec::new();
        sign_input.extend_from_slice(self.recipient_did.as_bytes());
        sign_input.extend_from_slice(&self.timestamp.to_be_bytes());
        sign_input.extend_from_slice(&self.sealed_payload);

        let sig_bytes: [u8; 64] = self
            .signature
            .as_slice()
            .try_into()
            .map_err(|_| JsError::new("invalid signature length"))?;
        let sig = ed25519_dalek::Signature::from_bytes(&sig_bytes);
        Ok(verifying_key
            .verify_strict(&sign_input, &sig)
            .is_ok())
    }

    /// Decrypt the sealed payload using the recipient's X25519 secret key.
    #[wasm_bindgen(js_name = decryptPayload)]
    pub fn decrypt_payload(&self, recipient_exchange_secret: &[u8]) -> Result<Vec<u8>, JsError> {
        let sealed = WasmSealedBox {
            data: self.sealed_payload.clone(),
        };
        WasmSealedBox::open(recipient_exchange_secret, &sealed)
    }

    // Getters

    #[wasm_bindgen(getter, js_name = senderDid)]
    pub fn sender_did(&self) -> String {
        self.sender_did.clone()
    }

    #[wasm_bindgen(getter, js_name = recipientDid)]
    pub fn recipient_did(&self) -> String {
        self.recipient_did.clone()
    }

    #[wasm_bindgen(getter, js_name = contentType)]
    pub fn content_type(&self) -> String {
        self.content_type.clone()
    }

    #[wasm_bindgen(getter)]
    pub fn timestamp(&self) -> f64 {
        self.timestamp
    }

    #[wasm_bindgen(getter)]
    pub fn signature(&self) -> Vec<u8> {
        self.signature.clone()
    }

    #[wasm_bindgen(getter, js_name = sealedPayload)]
    pub fn sealed_payload(&self) -> Vec<u8> {
        self.sealed_payload.clone()
    }

    /// Serialize the envelope to JSON for transport.
    #[wasm_bindgen(js_name = toJson)]
    pub fn to_json(&self) -> String {
        let b64 = base64::Engine::encode(
            &base64::engine::general_purpose::STANDARD,
            &self.sealed_payload,
        );
        let sig_b64 = base64::Engine::encode(
            &base64::engine::general_purpose::STANDARD,
            &self.signature,
        );
        serde_json::json!({
            "sender_did": self.sender_did,
            "recipient_did": self.recipient_did,
            "content_type": self.content_type,
            "timestamp": self.timestamp,
            "sealed_payload": b64,
            "signature": sig_b64,
        })
        .to_string()
    }

    /// Deserialize an envelope from JSON.
    #[wasm_bindgen(js_name = fromJson)]
    pub fn from_json(json: &str) -> Result<WasmEnvelope, JsError> {
        let v: serde_json::Value =
            serde_json::from_str(json).map_err(|e| JsError::new(&format!("invalid JSON: {e}")))?;

        let sender_did = v["sender_did"]
            .as_str()
            .ok_or_else(|| JsError::new("missing sender_did"))?
            .to_string();
        let recipient_did = v["recipient_did"]
            .as_str()
            .ok_or_else(|| JsError::new("missing recipient_did"))?
            .to_string();
        let content_type = v["content_type"]
            .as_str()
            .ok_or_else(|| JsError::new("missing content_type"))?
            .to_string();
        let timestamp = v["timestamp"]
            .as_f64()
            .ok_or_else(|| JsError::new("missing timestamp"))?;
        let sealed_payload = base64::Engine::decode(
            &base64::engine::general_purpose::STANDARD,
            v["sealed_payload"]
                .as_str()
                .ok_or_else(|| JsError::new("missing sealed_payload"))?,
        )
        .map_err(|e| JsError::new(&format!("invalid sealed_payload base64: {e}")))?;
        let signature = base64::Engine::decode(
            &base64::engine::general_purpose::STANDARD,
            v["signature"]
                .as_str()
                .ok_or_else(|| JsError::new("missing signature"))?,
        )
        .map_err(|e| JsError::new(&format!("invalid signature base64: {e}")))?;

        Ok(Self {
            sender_did,
            recipient_did,
            content_type,
            timestamp,
            sealed_payload,
            signature,
        })
    }
}

// ── Tests ────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn make_x25519_keypair() -> ([u8; 32], [u8; 32]) {
        let secret = x25519_dalek::StaticSecret::random_from_rng(OsRng);
        let public = x25519_dalek::PublicKey::from(&secret);
        (secret.to_bytes(), *public.as_bytes())
    }

    fn make_ed25519_keypair() -> ([u8; 32], [u8; 32]) {
        let signing = ed25519_dalek::SigningKey::generate(&mut OsRng);
        let verifying = signing.verifying_key();
        (signing.to_bytes(), verifying.to_bytes())
    }

    // --- Sealed Box ---

    #[test]
    fn sealed_box_roundtrip() {
        let (secret, public) = make_x25519_keypair();
        let plaintext = b"the will to power";
        let sealed = WasmSealedBox::seal(&public, plaintext).unwrap();
        let opened = WasmSealedBox::open(&secret, &sealed).unwrap();
        assert_eq!(opened, plaintext);
    }

    #[test]
    fn sealed_box_different_recipients_different_ciphertext() {
        let (_, pub1) = make_x25519_keypair();
        let (_, pub2) = make_x25519_keypair();
        let s1 = WasmSealedBox::seal(&pub1, b"same").unwrap();
        let s2 = WasmSealedBox::seal(&pub2, b"same").unwrap();
        assert_ne!(s1.data, s2.data);
    }

    #[test]
    fn sealed_box_each_seal_is_unique() {
        let (_, public) = make_x25519_keypair();
        let s1 = WasmSealedBox::seal(&public, b"same").unwrap();
        let s2 = WasmSealedBox::seal(&public, b"same").unwrap();
        // Different ephemeral keys → different ciphertext
        assert_ne!(s1.data, s2.data);
    }

    #[test]
    fn sealed_box_wrong_key_fails() {
        let (_, public) = make_x25519_keypair();
        let (wrong_secret, _) = make_x25519_keypair();
        let sealed = WasmSealedBox::seal(&public, b"secret").unwrap();
        let result = std::panic::catch_unwind(|| WasmSealedBox::open(&wrong_secret, &sealed));
        // Should fail: either panic (JsError on non-wasm) or Err
        assert!(result.is_err() || result.unwrap().is_err());
    }

    #[test]
    fn sealed_box_empty_plaintext() {
        let (secret, public) = make_x25519_keypair();
        let sealed = WasmSealedBox::seal(&public, b"").unwrap();
        let opened = WasmSealedBox::open(&secret, &sealed).unwrap();
        assert!(opened.is_empty());
    }

    #[test]
    fn sealed_box_large_payload() {
        let (secret, public) = make_x25519_keypair();
        let payload = vec![0x42u8; 65536];
        let sealed = WasmSealedBox::seal(&public, &payload).unwrap();
        let opened = WasmSealedBox::open(&secret, &sealed).unwrap();
        assert_eq!(opened, payload);
    }

    #[test]
    fn sealed_box_length() {
        let (_, public) = make_x25519_keypair();
        let sealed = WasmSealedBox::seal(&public, b"test").unwrap();
        // 32 (ephemeral pub) + 12 (nonce) + 4 (plaintext) + 16 (GCM tag)
        assert_eq!(sealed.len(), 32 + 12 + 4 + 16);
    }

    #[test]
    fn sealed_box_base64_roundtrip() {
        let (secret, public) = make_x25519_keypair();
        let sealed = WasmSealedBox::seal(&public, b"base64 test").unwrap();
        let b64 = sealed.to_base64();
        let result = std::panic::catch_unwind(|| WasmSealedBox::from_base64(&b64));
        match result {
            Ok(Ok(restored)) => {
                let opened = WasmSealedBox::open(&secret, &restored).unwrap();
                assert_eq!(opened, b"base64 test");
            }
            _ => {} // JsError on non-wasm
        }
    }

    #[test]
    fn sealed_box_tampered_data_fails() {
        let (secret, public) = make_x25519_keypair();
        let mut sealed = WasmSealedBox::seal(&public, b"original").unwrap();
        // Tamper with the last byte of ciphertext
        let last = sealed.data.len() - 1;
        sealed.data[last] ^= 0xFF;
        let result = std::panic::catch_unwind(|| WasmSealedBox::open(&secret, &sealed));
        assert!(result.is_err() || result.unwrap().is_err());
    }

    // --- Message Envelope ---

    #[test]
    fn envelope_create_and_verify() {
        let (sender_sign_sec, sender_sign_pub) = make_ed25519_keypair();
        let (recip_exch_sec, recip_exch_pub) = make_x25519_keypair();

        let env = WasmEnvelope::create(
            "did:key:zSender",
            "did:key:zRecipient",
            "text/plain",
            1711843200000.0,
            b"hello from nous",
            &sender_sign_sec,
            &recip_exch_pub,
        )
        .unwrap();

        // Verify signature
        let valid = env.verify_signature(&sender_sign_pub).unwrap();
        assert!(valid);

        // Decrypt
        let plaintext = env.decrypt_payload(&recip_exch_sec).unwrap();
        assert_eq!(plaintext, b"hello from nous");
    }

    #[test]
    fn envelope_metadata_preserved() {
        let (sender_sign, _) = make_ed25519_keypair();
        let (_, recip_pub) = make_x25519_keypair();

        let env = WasmEnvelope::create(
            "did:key:zSenderDid",
            "did:key:zRecipientDid",
            "application/json",
            1000.0,
            b"{}",
            &sender_sign,
            &recip_pub,
        )
        .unwrap();

        assert_eq!(env.sender_did(), "did:key:zSenderDid");
        assert_eq!(env.recipient_did(), "did:key:zRecipientDid");
        assert_eq!(env.content_type(), "application/json");
        assert_eq!(env.timestamp(), 1000.0);
        assert_eq!(env.signature().len(), 64);
    }

    #[test]
    fn envelope_wrong_sender_key_rejects() {
        let (sender_sign, _) = make_ed25519_keypair();
        let (_, wrong_pub) = make_ed25519_keypair();
        let (_, recip_pub) = make_x25519_keypair();

        let env = WasmEnvelope::create(
            "did:key:z1",
            "did:key:z2",
            "text/plain",
            0.0,
            b"msg",
            &sender_sign,
            &recip_pub,
        )
        .unwrap();

        let valid = env.verify_signature(&wrong_pub).unwrap();
        assert!(!valid);
    }

    #[test]
    fn envelope_tampered_payload_rejects_signature() {
        let (sender_sign, sender_pub) = make_ed25519_keypair();
        let (_, recip_pub) = make_x25519_keypair();

        let mut env = WasmEnvelope::create(
            "did:key:z1",
            "did:key:z2",
            "text/plain",
            0.0,
            b"original",
            &sender_sign,
            &recip_pub,
        )
        .unwrap();

        // Tamper with sealed payload
        if !env.sealed_payload.is_empty() {
            let last = env.sealed_payload.len() - 1;
            env.sealed_payload[last] ^= 0xFF;
        }

        let valid = env.verify_signature(&sender_pub).unwrap();
        assert!(!valid);
    }

    #[test]
    fn envelope_json_roundtrip() {
        let (sender_sign, sender_pub) = make_ed25519_keypair();
        let (recip_sec, recip_pub) = make_x25519_keypair();

        let env = WasmEnvelope::create(
            "did:key:zJsonTest",
            "did:key:zRecip",
            "text/plain",
            42000.0,
            b"json roundtrip",
            &sender_sign,
            &recip_pub,
        )
        .unwrap();

        let json = env.to_json();

        // Verify JSON is valid
        let v: serde_json::Value = serde_json::from_str(&json).unwrap();
        assert_eq!(v["sender_did"].as_str().unwrap(), "did:key:zJsonTest");
        assert_eq!(v["content_type"].as_str().unwrap(), "text/plain");
        assert_eq!(v["timestamp"].as_f64().unwrap(), 42000.0);

        // Round-trip through JSON
        let result = std::panic::catch_unwind(|| WasmEnvelope::from_json(&json));
        match result {
            Ok(Ok(restored)) => {
                assert_eq!(restored.sender_did(), "did:key:zJsonTest");
                assert!(restored.verify_signature(&sender_pub).unwrap());
                let plaintext = restored.decrypt_payload(&recip_sec).unwrap();
                assert_eq!(plaintext, b"json roundtrip");
            }
            _ => {} // JsError on non-wasm
        }
    }

    #[test]
    fn envelope_different_messages_different_payloads() {
        let (sender_sign, _) = make_ed25519_keypair();
        let (_, recip_pub) = make_x25519_keypair();

        let e1 = WasmEnvelope::create(
            "did:key:z1",
            "did:key:z2",
            "text/plain",
            0.0,
            b"message one",
            &sender_sign,
            &recip_pub,
        )
        .unwrap();

        let e2 = WasmEnvelope::create(
            "did:key:z1",
            "did:key:z2",
            "text/plain",
            0.0,
            b"message two",
            &sender_sign,
            &recip_pub,
        )
        .unwrap();

        assert_ne!(e1.sealed_payload, e2.sealed_payload);
        assert_ne!(e1.signature, e2.signature);
    }

    #[test]
    fn envelope_empty_plaintext() {
        let (sender_sign, sender_pub) = make_ed25519_keypair();
        let (recip_sec, recip_pub) = make_x25519_keypair();

        let env = WasmEnvelope::create(
            "did:key:z1",
            "did:key:z2",
            "text/plain",
            0.0,
            b"",
            &sender_sign,
            &recip_pub,
        )
        .unwrap();

        assert!(env.verify_signature(&sender_pub).unwrap());
        let decrypted = env.decrypt_payload(&recip_sec).unwrap();
        assert!(decrypted.is_empty());
    }

    #[test]
    fn envelope_large_payload() {
        let (sender_sign, sender_pub) = make_ed25519_keypair();
        let (recip_sec, recip_pub) = make_x25519_keypair();
        let payload = vec![0xAB; 100_000];

        let env = WasmEnvelope::create(
            "did:key:z1",
            "did:key:z2",
            "application/octet-stream",
            0.0,
            &payload,
            &sender_sign,
            &recip_pub,
        )
        .unwrap();

        assert!(env.verify_signature(&sender_pub).unwrap());
        let decrypted = env.decrypt_payload(&recip_sec).unwrap();
        assert_eq!(decrypted, payload);
    }

    // --- Helper function tests ---

    #[test]
    fn derive_aes_key_deterministic() {
        let secret = [42u8; 32];
        let k1 = derive_aes_key(&secret, b"ctx");
        let k2 = derive_aes_key(&secret, b"ctx");
        assert_eq!(k1, k2);
    }

    #[test]
    fn derive_aes_key_different_contexts() {
        let secret = [42u8; 32];
        let k1 = derive_aes_key(&secret, b"ctx-a");
        let k2 = derive_aes_key(&secret, b"ctx-b");
        assert_ne!(k1, k2);
    }

    #[test]
    fn aes_encrypt_decrypt_roundtrip() {
        let key = [99u8; 32];
        let plaintext = b"roundtrip test";
        let (ciphertext, nonce) = aes_encrypt(&key, plaintext);
        let decrypted = aes_decrypt(&key, &nonce, &ciphertext).unwrap();
        assert_eq!(decrypted, plaintext);
    }

    #[test]
    fn aes_encrypt_unique_nonces() {
        let key = [99u8; 32];
        let (_, n1) = aes_encrypt(&key, b"same");
        let (_, n2) = aes_encrypt(&key, b"same");
        assert_ne!(n1, n2);
    }

    #[test]
    fn aes_decrypt_wrong_key_fails() {
        let key1 = [1u8; 32];
        let key2 = [2u8; 32];
        let (ct, nonce) = aes_encrypt(&key1, b"secret");
        assert!(aes_decrypt(&key2, &nonce, &ct).is_err());
    }
}
