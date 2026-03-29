//! NIP-04 encrypted direct messages.
//!
//! Uses x25519 ECDH key exchange (ed25519 keys converted to x25519) and
//! AES-256-GCM for authenticated encryption. The shared secret is derived
//! via HKDF-SHA256.

use aes_gcm::aead::{Aead, KeyInit};
use aes_gcm::{Aes256Gcm, Nonce};
use base64::engine::general_purpose::STANDARD as BASE64;
use base64::Engine;
use ed25519_dalek::SigningKey;
use hkdf::Hkdf;
use rand::RngCore;
use sha2::Sha256;
use x25519_dalek::{PublicKey as X25519Public, StaticSecret};

/// Convert an ed25519 signing key to an x25519 static secret.
fn ed25519_to_x25519_secret(signing_key: &SigningKey) -> StaticSecret {
    let mut hash = sha2::Sha512::default();
    sha2::Digest::update(&mut hash, signing_key.to_bytes());
    let result = sha2::Digest::finalize(hash);
    let mut secret = [0u8; 32];
    secret.copy_from_slice(&result[..32]);
    // Clamp per curve25519 convention
    secret[0] &= 248;
    secret[31] &= 127;
    secret[31] |= 64;
    StaticSecret::from(secret)
}

/// Convert an ed25519 public key (32 bytes hex) to an x25519 public key.
fn ed25519_pubkey_to_x25519(pubkey_hex: &str) -> Result<X25519Public, String> {
    let pubkey_bytes = hex::decode(pubkey_hex).map_err(|e| format!("invalid pubkey hex: {e}"))?;
    let pubkey_arr: [u8; 32] = pubkey_bytes
        .try_into()
        .map_err(|_| "pubkey must be 32 bytes".to_string())?;

    // Use curve25519-dalek's conversion from Edwards to Montgomery form
    let ed_point = curve25519_dalek::edwards::CompressedEdwardsY(pubkey_arr);
    let mont = ed_point
        .decompress()
        .ok_or("invalid ed25519 point")?
        .to_montgomery();
    Ok(X25519Public::from(mont.to_bytes()))
}

/// Derive a shared AES-256 key from the x25519 shared secret.
fn derive_aes_key(shared_secret: &[u8]) -> [u8; 32] {
    let hk = Hkdf::<Sha256>::new(Some(b"nous-nip04"), shared_secret);
    let mut key = [0u8; 32];
    hk.expand(b"aes-256-gcm", &mut key)
        .expect("32 bytes is a valid length for HKDF-SHA256");
    key
}

/// Encrypt a plaintext message for a recipient.
///
/// Returns a string in the format: `base64(nonce)||?||base64(ciphertext)`
/// (using `?` as the separator, following NIP-04 conventions with `?iv=`).
pub fn encrypt(
    plaintext: &str,
    recipient_pubkey_hex: &str,
    sender_signing_key: &SigningKey,
) -> Result<String, String> {
    let sender_x25519 = ed25519_to_x25519_secret(sender_signing_key);
    let recipient_x25519 = ed25519_pubkey_to_x25519(recipient_pubkey_hex)?;
    let shared = sender_x25519.diffie_hellman(&recipient_x25519);
    let aes_key = derive_aes_key(shared.as_bytes());

    let cipher = Aes256Gcm::new_from_slice(&aes_key)
        .map_err(|e| format!("cipher init failed: {e}"))?;

    let mut nonce_bytes = [0u8; 12];
    rand::thread_rng().fill_bytes(&mut nonce_bytes);
    let nonce = Nonce::from_slice(&nonce_bytes);

    let ciphertext = cipher
        .encrypt(nonce, plaintext.as_bytes())
        .map_err(|e| format!("encryption failed: {e}"))?;

    Ok(format!(
        "{}?iv={}",
        BASE64.encode(&ciphertext),
        BASE64.encode(nonce_bytes)
    ))
}

/// Decrypt an encrypted message from a sender.
///
/// `encrypted` should be in the format: `base64(ciphertext)?iv=base64(nonce)`.
pub fn decrypt(
    encrypted: &str,
    sender_pubkey_hex: &str,
    recipient_signing_key: &SigningKey,
) -> Result<String, String> {
    let parts: Vec<&str> = encrypted.splitn(2, "?iv=").collect();
    if parts.len() != 2 {
        return Err("invalid encrypted format: expected 'ciphertext?iv=nonce'".into());
    }

    let ciphertext = BASE64
        .decode(parts[0])
        .map_err(|e| format!("invalid ciphertext base64: {e}"))?;
    let nonce_bytes = BASE64
        .decode(parts[1])
        .map_err(|e| format!("invalid nonce base64: {e}"))?;

    if nonce_bytes.len() != 12 {
        return Err("nonce must be 12 bytes".into());
    }

    let recipient_x25519 = ed25519_to_x25519_secret(recipient_signing_key);
    let sender_x25519 = ed25519_pubkey_to_x25519(sender_pubkey_hex)?;
    let shared = recipient_x25519.diffie_hellman(&sender_x25519);
    let aes_key = derive_aes_key(shared.as_bytes());

    let cipher = Aes256Gcm::new_from_slice(&aes_key)
        .map_err(|e| format!("cipher init failed: {e}"))?;
    let nonce = Nonce::from_slice(&nonce_bytes);

    let plaintext = cipher
        .decrypt(nonce, ciphertext.as_ref())
        .map_err(|e| format!("decryption failed: {e}"))?;

    String::from_utf8(plaintext).map_err(|e| format!("invalid utf-8: {e}"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use rand::rngs::OsRng;

    fn keypair() -> SigningKey {
        SigningKey::generate(&mut OsRng)
    }

    fn pubkey_hex(key: &SigningKey) -> String {
        hex::encode(key.verifying_key().as_bytes())
    }

    #[test]
    fn encrypt_decrypt_roundtrip() {
        let alice = keypair();
        let bob = keypair();

        let plaintext = "hello bob, this is a secret message";
        let encrypted = encrypt(plaintext, &pubkey_hex(&bob), &alice).unwrap();

        // Bob decrypts
        let decrypted = decrypt(&encrypted, &pubkey_hex(&alice), &bob).unwrap();
        assert_eq!(decrypted, plaintext);
    }

    #[test]
    fn different_messages_produce_different_ciphertexts() {
        let alice = keypair();
        let bob = keypair();

        let c1 = encrypt("message 1", &pubkey_hex(&bob), &alice).unwrap();
        let c2 = encrypt("message 2", &pubkey_hex(&bob), &alice).unwrap();
        assert_ne!(c1, c2);
    }

    #[test]
    fn same_message_produces_different_ciphertexts() {
        let alice = keypair();
        let bob = keypair();

        let c1 = encrypt("same message", &pubkey_hex(&bob), &alice).unwrap();
        let c2 = encrypt("same message", &pubkey_hex(&bob), &alice).unwrap();
        // Random nonce ensures different ciphertexts
        assert_ne!(c1, c2);
    }

    #[test]
    fn wrong_recipient_cannot_decrypt() {
        let alice = keypair();
        let bob = keypair();
        let eve = keypair();

        let encrypted = encrypt("secret", &pubkey_hex(&bob), &alice).unwrap();

        // Eve tries to decrypt — should fail
        let result = decrypt(&encrypted, &pubkey_hex(&alice), &eve);
        assert!(result.is_err());
    }

    #[test]
    fn invalid_format_rejected() {
        let bob = keypair();
        let result = decrypt("not_valid_format", "deadbeef".repeat(4).as_str(), &bob);
        assert!(result.is_err());
    }

    #[test]
    fn empty_message_roundtrip() {
        let alice = keypair();
        let bob = keypair();

        let encrypted = encrypt("", &pubkey_hex(&bob), &alice).unwrap();
        let decrypted = decrypt(&encrypted, &pubkey_hex(&alice), &bob).unwrap();
        assert_eq!(decrypted, "");
    }

    #[test]
    fn long_message_roundtrip() {
        let alice = keypair();
        let bob = keypair();

        let long_msg = "x".repeat(10_000);
        let encrypted = encrypt(&long_msg, &pubkey_hex(&bob), &alice).unwrap();
        let decrypted = decrypt(&encrypted, &pubkey_hex(&alice), &bob).unwrap();
        assert_eq!(decrypted, long_msg);
    }

    #[test]
    fn unicode_message_roundtrip() {
        let alice = keypair();
        let bob = keypair();

        let msg = "hello 🌍 welt 世界 мир";
        let encrypted = encrypt(msg, &pubkey_hex(&bob), &alice).unwrap();
        let decrypted = decrypt(&encrypted, &pubkey_hex(&alice), &bob).unwrap();
        assert_eq!(decrypted, msg);
    }
}
