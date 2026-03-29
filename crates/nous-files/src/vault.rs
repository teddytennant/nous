use aes_gcm::{
    Aes256Gcm, Nonce,
    aead::{Aead, KeyInit},
};
use argon2::Argon2;
use rand::RngCore;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use zeroize::Zeroize;

use nous_core::{Error, Result};

const NONCE_SIZE: usize = 12;
const SALT_SIZE: usize = 32;
const KEY_SIZE: usize = 32;

/// An encrypted vault that protects files with a password-derived key.
///
/// Vaults use Argon2id for key derivation (memory-hard, resistant to GPU/ASIC attacks)
/// and AES-256-GCM for authenticated encryption.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Vault {
    /// Unique vault identifier.
    pub id: String,
    /// Human-readable name.
    pub name: String,
    /// Argon2id salt for key derivation.
    salt: Vec<u8>,
    /// Encrypted vault key — the actual encryption key is wrapped with the password-derived key.
    /// This allows password changes without re-encrypting all files.
    encrypted_master_key: EncryptedBlob,
    /// Metadata: creation timestamp.
    pub created_at: chrono::DateTime<chrono::Utc>,
    /// Metadata: last modified timestamp.
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

/// An encrypted blob with its nonce.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EncryptedBlob {
    pub nonce: Vec<u8>,
    pub ciphertext: Vec<u8>,
}

/// A file entry inside a vault.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VaultEntry {
    /// Original filename.
    pub name: String,
    /// MIME type.
    pub mime_type: String,
    /// Size of the plaintext file in bytes.
    pub size: u64,
    /// SHA-256 hash of the plaintext content.
    pub content_hash: String,
    /// The encrypted file data.
    pub encrypted: EncryptedBlob,
    /// When this entry was added.
    pub created_at: chrono::DateTime<chrono::Utc>,
}

impl Vault {
    /// Create a new vault protected by a password.
    pub fn create(name: &str, password: &[u8]) -> Result<Self> {
        if password.is_empty() {
            return Err(Error::InvalidInput("vault password cannot be empty".into()));
        }

        let id = uuid::Uuid::new_v4().to_string();

        // Generate random salt for Argon2id.
        let mut salt = vec![0u8; SALT_SIZE];
        rand::rngs::OsRng.fill_bytes(&mut salt);

        // Derive key from password.
        let mut password_key = derive_key_from_password(password, &salt)?;

        // Generate a random master key — this is what actually encrypts files.
        let mut master_key = [0u8; KEY_SIZE];
        rand::rngs::OsRng.fill_bytes(&mut master_key);

        // Wrap the master key with the password-derived key.
        let encrypted_master_key = encrypt_blob(&password_key, &master_key)?;

        // Zeroize sensitive material.
        password_key.zeroize();
        master_key.zeroize();

        let now = chrono::Utc::now();

        Ok(Self {
            id,
            name: name.to_string(),
            salt,
            encrypted_master_key,
            created_at: now,
            updated_at: now,
        })
    }

    /// Unlock the vault and return the master key.
    pub fn unlock(&self, password: &[u8]) -> Result<VaultKey> {
        let mut password_key = derive_key_from_password(password, &self.salt)?;
        let master_key_bytes = decrypt_blob(&password_key, &self.encrypted_master_key)?;
        password_key.zeroize();

        let master_key: [u8; KEY_SIZE] = master_key_bytes
            .try_into()
            .map_err(|_| Error::Crypto("corrupted master key".into()))?;

        Ok(VaultKey(master_key))
    }

    /// Change the vault password without re-encrypting files.
    pub fn change_password(&mut self, old_password: &[u8], new_password: &[u8]) -> Result<()> {
        if new_password.is_empty() {
            return Err(Error::InvalidInput("new password cannot be empty".into()));
        }

        // Decrypt master key with old password.
        let mut vault_key = self.unlock(old_password)?;

        // Generate new salt and re-wrap with new password.
        let mut new_salt = vec![0u8; SALT_SIZE];
        rand::rngs::OsRng.fill_bytes(&mut new_salt);

        let mut new_password_key = derive_key_from_password(new_password, &new_salt)?;
        let new_encrypted_master = encrypt_blob(&new_password_key, &vault_key.0)?;

        new_password_key.zeroize();
        vault_key.0.zeroize();

        self.salt = new_salt;
        self.encrypted_master_key = new_encrypted_master;
        self.updated_at = chrono::Utc::now();

        Ok(())
    }

    /// Encrypt a file for storage in this vault.
    pub fn encrypt_file(&self, key: &VaultKey, name: &str, mime_type: &str, data: &[u8]) -> Result<VaultEntry> {
        let content_hash = sha256_hex(data);
        let encrypted = encrypt_blob(&key.0, data)?;

        Ok(VaultEntry {
            name: name.to_string(),
            mime_type: mime_type.to_string(),
            size: data.len() as u64,
            content_hash,
            encrypted,
            created_at: chrono::Utc::now(),
        })
    }

    /// Decrypt a file from this vault.
    pub fn decrypt_file(&self, key: &VaultKey, entry: &VaultEntry) -> Result<Vec<u8>> {
        let plaintext = decrypt_blob(&key.0, &entry.encrypted)?;

        // Verify integrity.
        let actual_hash = sha256_hex(&plaintext);
        if actual_hash != entry.content_hash {
            return Err(Error::Crypto("file integrity check failed — hash mismatch".into()));
        }

        Ok(plaintext)
    }
}

/// The decrypted master key for a vault. Zeroized on drop.
pub struct VaultKey(pub(crate) [u8; KEY_SIZE]);

impl Drop for VaultKey {
    fn drop(&mut self) {
        self.0.zeroize();
    }
}

impl std::fmt::Debug for VaultKey {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("VaultKey(***)")
    }
}

fn derive_key_from_password(password: &[u8], salt: &[u8]) -> Result<[u8; KEY_SIZE]> {
    let mut key = [0u8; KEY_SIZE];
    Argon2::default()
        .hash_password_into(password, salt, &mut key)
        .map_err(|e| Error::Crypto(format!("argon2id key derivation failed: {e}")))?;
    Ok(key)
}

fn encrypt_blob(key: &[u8; KEY_SIZE], plaintext: &[u8]) -> Result<EncryptedBlob> {
    let cipher = Aes256Gcm::new_from_slice(key)
        .map_err(|e| Error::Crypto(format!("cipher init failed: {e}")))?;

    let mut nonce_bytes = [0u8; NONCE_SIZE];
    rand::rngs::OsRng.fill_bytes(&mut nonce_bytes);
    let nonce = Nonce::from_slice(&nonce_bytes);

    let ciphertext = cipher
        .encrypt(nonce, plaintext)
        .map_err(|e| Error::Crypto(format!("encryption failed: {e}")))?;

    Ok(EncryptedBlob {
        nonce: nonce_bytes.to_vec(),
        ciphertext,
    })
}

fn decrypt_blob(key: &[u8; KEY_SIZE], blob: &EncryptedBlob) -> Result<Vec<u8>> {
    let cipher = Aes256Gcm::new_from_slice(key)
        .map_err(|e| Error::Crypto(format!("cipher init failed: {e}")))?;

    let nonce_bytes: [u8; NONCE_SIZE] = blob
        .nonce
        .as_slice()
        .try_into()
        .map_err(|_| Error::Crypto(format!("nonce must be {NONCE_SIZE} bytes")))?;
    let nonce = Nonce::from_slice(&nonce_bytes);

    cipher
        .decrypt(nonce, blob.ciphertext.as_ref())
        .map_err(|e| Error::Crypto(format!("decryption failed: {e}")))
}

fn sha256_hex(data: &[u8]) -> String {
    let hash = Sha256::digest(data);
    hash.iter().fold(String::new(), |mut s, b| {
        use std::fmt::Write;
        let _ = write!(s, "{b:02x}");
        s
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn create_vault() {
        let vault = Vault::create("test-vault", b"strong-password").unwrap();
        assert_eq!(vault.name, "test-vault");
        assert!(!vault.id.is_empty());
    }

    #[test]
    fn create_vault_rejects_empty_password() {
        assert!(Vault::create("v", b"").is_err());
    }

    #[test]
    fn unlock_vault_correct_password() {
        let vault = Vault::create("v", b"password123").unwrap();
        let key = vault.unlock(b"password123");
        assert!(key.is_ok());
    }

    #[test]
    fn unlock_vault_wrong_password() {
        let vault = Vault::create("v", b"correct").unwrap();
        assert!(vault.unlock(b"incorrect").is_err());
    }

    #[test]
    fn encrypt_decrypt_file_roundtrip() {
        let vault = Vault::create("v", b"password").unwrap();
        let key = vault.unlock(b"password").unwrap();

        let data = b"the sovereign individual needs sovereign storage";
        let entry = vault.encrypt_file(&key, "manifesto.txt", "text/plain", data).unwrap();

        assert_eq!(entry.name, "manifesto.txt");
        assert_eq!(entry.mime_type, "text/plain");
        assert_eq!(entry.size, data.len() as u64);

        let decrypted = vault.decrypt_file(&key, &entry).unwrap();
        assert_eq!(decrypted, data);
    }

    #[test]
    fn encrypt_file_produces_unique_nonces() {
        let vault = Vault::create("v", b"pass").unwrap();
        let key = vault.unlock(b"pass").unwrap();

        let e1 = vault.encrypt_file(&key, "a.txt", "text/plain", b"same data").unwrap();
        let e2 = vault.encrypt_file(&key, "a.txt", "text/plain", b"same data").unwrap();

        assert_ne!(e1.encrypted.nonce, e2.encrypted.nonce);
    }

    #[test]
    fn decrypt_detects_tampered_ciphertext() {
        let vault = Vault::create("v", b"pass").unwrap();
        let key = vault.unlock(b"pass").unwrap();

        let mut entry = vault.encrypt_file(&key, "a.txt", "text/plain", b"integrity test").unwrap();
        if let Some(byte) = entry.encrypted.ciphertext.last_mut() {
            *byte ^= 0xFF;
        }

        assert!(vault.decrypt_file(&key, &entry).is_err());
    }

    #[test]
    fn decrypt_detects_hash_mismatch() {
        let vault = Vault::create("v", b"pass").unwrap();
        let key = vault.unlock(b"pass").unwrap();

        let mut entry = vault.encrypt_file(&key, "a.txt", "text/plain", b"original").unwrap();
        // Swap encrypted data with encryption of different content but keep old hash
        let other_entry = vault.encrypt_file(&key, "b.txt", "text/plain", b"different").unwrap();
        entry.encrypted = other_entry.encrypted;

        assert!(vault.decrypt_file(&key, &entry).is_err());
    }

    #[test]
    fn change_password() {
        let mut vault = Vault::create("v", b"old-pass").unwrap();
        let key_before = vault.unlock(b"old-pass").unwrap();

        vault.change_password(b"old-pass", b"new-pass").unwrap();

        // Old password no longer works.
        assert!(vault.unlock(b"old-pass").is_err());

        // New password works and produces same master key.
        let key_after = vault.unlock(b"new-pass").unwrap();

        // Encrypt with old key, decrypt with new key — same master key.
        let entry = vault.encrypt_file(&key_before, "test.txt", "text/plain", b"data").unwrap();
        let decrypted = vault.decrypt_file(&key_after, &entry).unwrap();
        assert_eq!(decrypted, b"data");
    }

    #[test]
    fn change_password_rejects_empty() {
        let mut vault = Vault::create("v", b"pass").unwrap();
        assert!(vault.change_password(b"pass", b"").is_err());
    }

    #[test]
    fn change_password_rejects_wrong_old() {
        let mut vault = Vault::create("v", b"correct").unwrap();
        assert!(vault.change_password(b"wrong", b"new").is_err());
    }

    #[test]
    fn vault_serde_roundtrip() {
        let vault = Vault::create("serialization-test", b"pass").unwrap();
        let json = serde_json::to_string(&vault).unwrap();
        let deserialized: Vault = serde_json::from_str(&json).unwrap();

        assert_eq!(deserialized.name, vault.name);
        assert_eq!(deserialized.id, vault.id);

        // Should still be unlockable after deserialization.
        let key = deserialized.unlock(b"pass").unwrap();
        let entry = deserialized.encrypt_file(&key, "f.txt", "text/plain", b"test").unwrap();
        let decrypted = deserialized.decrypt_file(&key, &entry).unwrap();
        assert_eq!(decrypted, b"test");
    }

    #[test]
    fn encrypt_empty_file() {
        let vault = Vault::create("v", b"pass").unwrap();
        let key = vault.unlock(b"pass").unwrap();

        let entry = vault.encrypt_file(&key, "empty.bin", "application/octet-stream", b"").unwrap();
        assert_eq!(entry.size, 0);

        let decrypted = vault.decrypt_file(&key, &entry).unwrap();
        assert!(decrypted.is_empty());
    }

    #[test]
    fn encrypt_large_file() {
        let vault = Vault::create("v", b"pass").unwrap();
        let key = vault.unlock(b"pass").unwrap();

        let data = vec![0xABu8; 1_000_000]; // 1 MB
        let entry = vault.encrypt_file(&key, "large.bin", "application/octet-stream", &data).unwrap();
        let decrypted = vault.decrypt_file(&key, &entry).unwrap();
        assert_eq!(decrypted, data);
    }

    #[test]
    fn vault_key_debug_does_not_leak() {
        let vault = Vault::create("v", b"pass").unwrap();
        let key = vault.unlock(b"pass").unwrap();
        let debug = format!("{key:?}");
        assert!(debug.contains("***"));
        assert!(!debug.contains("pass"));
    }
}
