use aes_gcm::{
    Aes256Gcm, Nonce,
    aead::{Aead, KeyInit},
};
use hkdf::Hkdf;
use rand::RngCore;
use serde::{Deserialize, Serialize};
use sha2::Sha256;
use zeroize::Zeroize;

use nous_core::{Error, Result};

const NONCE_SIZE: usize = 12;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EncryptedPayload {
    pub nonce: Vec<u8>,
    pub ciphertext: Vec<u8>,
}

pub fn encrypt(key: &[u8; 32], plaintext: &[u8]) -> Result<EncryptedPayload> {
    let cipher = Aes256Gcm::new_from_slice(key)
        .map_err(|e| Error::Crypto(format!("failed to create cipher: {e}")))?;

    let mut nonce_bytes = [0u8; NONCE_SIZE];
    rand::rngs::OsRng.fill_bytes(&mut nonce_bytes);
    let nonce = Nonce::from_slice(&nonce_bytes);

    let ciphertext = cipher
        .encrypt(nonce, plaintext)
        .map_err(|e| Error::Crypto(format!("encryption failed: {e}")))?;

    Ok(EncryptedPayload {
        nonce: nonce_bytes.to_vec(),
        ciphertext,
    })
}

pub fn decrypt(key: &[u8; 32], payload: &EncryptedPayload) -> Result<Vec<u8>> {
    let cipher = Aes256Gcm::new_from_slice(key)
        .map_err(|e| Error::Crypto(format!("failed to create cipher: {e}")))?;

    let nonce_bytes: [u8; NONCE_SIZE] = payload
        .nonce
        .as_slice()
        .try_into()
        .map_err(|_| Error::Crypto(format!("nonce must be {NONCE_SIZE} bytes")))?;
    let nonce = Nonce::from_slice(&nonce_bytes);

    cipher
        .decrypt(nonce, payload.ciphertext.as_ref())
        .map_err(|e| Error::Crypto(format!("decryption failed: {e}")))
}

pub fn derive_key(shared_secret: &[u8; 32], context: &[u8]) -> [u8; 32] {
    let hk = Hkdf::<Sha256>::new(None, shared_secret);
    let mut key = [0u8; 32];
    hk.expand(context, &mut key)
        .expect("HKDF-SHA256 expand to 32 bytes always succeeds");
    key
}

pub fn encrypt_for_shared_secret(
    shared_secret: &[u8; 32],
    context: &[u8],
    plaintext: &[u8],
) -> Result<EncryptedPayload> {
    let mut key = derive_key(shared_secret, context);
    let result = encrypt(&key, plaintext);
    key.zeroize();
    result
}

pub fn decrypt_for_shared_secret(
    shared_secret: &[u8; 32],
    context: &[u8],
    payload: &EncryptedPayload,
) -> Result<Vec<u8>> {
    let mut key = derive_key(shared_secret, context);
    let result = decrypt(&key, payload);
    key.zeroize();
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_key() -> [u8; 32] {
        let mut key = [0u8; 32];
        rand::rngs::OsRng.fill_bytes(&mut key);
        key
    }

    #[test]
    fn encrypt_decrypt_roundtrip() {
        let key = test_key();
        let plaintext = b"amor fati - love your fate";

        let encrypted = encrypt(&key, plaintext).unwrap();
        let decrypted = decrypt(&key, &encrypted).unwrap();

        assert_eq!(decrypted, plaintext);
    }

    #[test]
    fn encrypt_produces_unique_nonces() {
        let key = test_key();
        let plaintext = b"same message";

        let a = encrypt(&key, plaintext).unwrap();
        let b = encrypt(&key, plaintext).unwrap();

        assert_ne!(a.nonce, b.nonce);
        assert_ne!(a.ciphertext, b.ciphertext);
    }

    #[test]
    fn decrypt_rejects_wrong_key() {
        let key1 = test_key();
        let key2 = test_key();

        let encrypted = encrypt(&key1, b"secret").unwrap();
        assert!(decrypt(&key2, &encrypted).is_err());
    }

    #[test]
    fn decrypt_rejects_tampered_ciphertext() {
        let key = test_key();
        let mut encrypted = encrypt(&key, b"integrity test").unwrap();

        if let Some(byte) = encrypted.ciphertext.last_mut() {
            *byte ^= 0xff;
        }

        assert!(decrypt(&key, &encrypted).is_err());
    }

    #[test]
    fn decrypt_rejects_tampered_nonce() {
        let key = test_key();
        let mut encrypted = encrypt(&key, b"nonce test").unwrap();

        encrypted.nonce[0] ^= 0xff;

        assert!(decrypt(&key, &encrypted).is_err());
    }

    #[test]
    fn decrypt_rejects_wrong_nonce_length() {
        let key = test_key();
        let bad_payload = EncryptedPayload {
            nonce: vec![0u8; 8], // wrong size
            ciphertext: vec![1, 2, 3],
        };

        assert!(decrypt(&key, &bad_payload).is_err());
    }

    #[test]
    fn encrypt_empty_plaintext() {
        let key = test_key();
        let encrypted = encrypt(&key, b"").unwrap();
        let decrypted = decrypt(&key, &encrypted).unwrap();
        assert!(decrypted.is_empty());
    }

    #[test]
    fn encrypt_large_plaintext() {
        let key = test_key();
        let plaintext = vec![0xABu8; 1_000_000]; // 1MB

        let encrypted = encrypt(&key, &plaintext).unwrap();
        let decrypted = decrypt(&key, &encrypted).unwrap();

        assert_eq!(decrypted, plaintext);
    }

    #[test]
    fn derive_key_deterministic() {
        let secret = test_key();
        let context = b"nous-messaging-v1";

        let k1 = derive_key(&secret, context);
        let k2 = derive_key(&secret, context);

        assert_eq!(k1, k2);
    }

    #[test]
    fn derive_key_different_contexts_produce_different_keys() {
        let secret = test_key();

        let k1 = derive_key(&secret, b"context-a");
        let k2 = derive_key(&secret, b"context-b");

        assert_ne!(k1, k2);
    }

    #[test]
    fn shared_secret_encrypt_decrypt_roundtrip() {
        let secret = test_key();
        let context = b"nous-test";
        let plaintext = b"encrypted via shared secret";

        let encrypted = encrypt_for_shared_secret(&secret, context, plaintext).unwrap();
        let decrypted = decrypt_for_shared_secret(&secret, context, &encrypted).unwrap();

        assert_eq!(decrypted, plaintext);
    }

    #[test]
    fn encrypted_payload_serde_roundtrip() {
        let key = test_key();
        let encrypted = encrypt(&key, b"serde test").unwrap();

        let json = serde_json::to_string(&encrypted).unwrap();
        let deserialized: EncryptedPayload = serde_json::from_str(&json).unwrap();

        let decrypted = decrypt(&key, &deserialized).unwrap();
        assert_eq!(decrypted, b"serde test");
    }
}
