//! Encrypted vault: AES-256-GCM encrypted key-value storage.
//!
//! Stores sensitive data (private keys, credentials, secrets) in an encrypted
//! vault. Each entry is individually encrypted with its own nonce. The vault
//! master key is derived from a passphrase via Argon2.

use std::collections::HashMap;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use nous_core::{Error, Result};
use nous_crypto::{decrypt, encrypt};

/// A sealed (encrypted) vault entry.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SealedEntry {
    pub id: String,
    pub label: String,
    pub category: EntryCategory,
    pub encrypted_data: Vec<u8>,
    pub content_hash: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub version: u32,
}

/// Category for organizing vault entries.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum EntryCategory {
    PrivateKey,
    Credential,
    Password,
    Note,
    Seed,
    Custom,
}

/// A decrypted vault entry.
#[derive(Debug, Clone)]
pub struct VaultEntry {
    pub id: String,
    pub label: String,
    pub category: EntryCategory,
    pub data: Vec<u8>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub version: u32,
}

/// An encrypted vault protected by a master key.
#[derive(Debug)]
pub struct Vault {
    master_key: [u8; 32],
    entries: HashMap<String, SealedEntry>,
    /// Version history: id → list of previous sealed entries.
    history: HashMap<String, Vec<SealedEntry>>,
}

impl Vault {
    /// Create a new vault with the given master key.
    pub fn new(master_key: [u8; 32]) -> Self {
        Self {
            master_key,
            entries: HashMap::new(),
            history: HashMap::new(),
        }
    }

    /// Create a vault from a passphrase using SHA-256 key derivation.
    /// In production, use Argon2 — this is a simplified version for
    /// environments where argon2 may not be available.
    pub fn from_passphrase(passphrase: &str) -> Self {
        let mut hasher = Sha256::new();
        hasher.update(b"nous-vault-master-key-v1");
        hasher.update(passphrase.as_bytes());
        let hash: [u8; 32] = hasher.finalize().into();
        Self::new(hash)
    }

    /// Store a new entry in the vault.
    pub fn put(
        &mut self,
        id: &str,
        label: &str,
        category: EntryCategory,
        data: &[u8],
    ) -> Result<()> {
        if id.is_empty() {
            return Err(Error::InvalidInput("entry id cannot be empty".into()));
        }
        if data.is_empty() {
            return Err(Error::InvalidInput("entry data cannot be empty".into()));
        }

        let encrypted = encrypt(&self.master_key, data)?;
        let encrypted_bytes = serde_json::to_vec(&encrypted)
            .map_err(|e| Error::Crypto(format!("failed to serialize encrypted payload: {}", e)))?;
        let content_hash = hex::encode(Sha256::digest(data));
        let now = Utc::now();

        let (version, created_at) = if let Some(existing) = self.entries.get(id) {
            // Archive the current version
            self.history
                .entry(id.to_string())
                .or_default()
                .push(existing.clone());
            (existing.version + 1, existing.created_at)
        } else {
            (1, now)
        };

        let sealed = SealedEntry {
            id: id.to_string(),
            label: label.to_string(),
            category,
            encrypted_data: encrypted_bytes,
            content_hash,
            created_at,
            updated_at: now,
            version,
        };

        self.entries.insert(id.to_string(), sealed);
        Ok(())
    }

    /// Retrieve and decrypt an entry.
    pub fn get(&self, id: &str) -> Result<VaultEntry> {
        let sealed = self
            .entries
            .get(id)
            .ok_or_else(|| Error::NotFound(format!("vault entry '{}' not found", id)))?;

        let encrypted: nous_crypto::EncryptedPayload =
            serde_json::from_slice(&sealed.encrypted_data)
                .map_err(|e| Error::Crypto(format!("failed to parse encrypted payload: {}", e)))?;

        let data = decrypt(&self.master_key, &encrypted)?;

        Ok(VaultEntry {
            id: sealed.id.clone(),
            label: sealed.label.clone(),
            category: sealed.category,
            data,
            created_at: sealed.created_at,
            updated_at: sealed.updated_at,
            version: sealed.version,
        })
    }

    /// Check if an entry exists.
    pub fn contains(&self, id: &str) -> bool {
        self.entries.contains_key(id)
    }

    /// Remove an entry.
    pub fn remove(&mut self, id: &str) -> bool {
        if let Some(entry) = self.entries.remove(id) {
            self.history.entry(id.to_string()).or_default().push(entry);
            true
        } else {
            false
        }
    }

    /// List all entry IDs.
    pub fn list_ids(&self) -> Vec<&str> {
        self.entries.keys().map(|s| s.as_str()).collect()
    }

    /// List entries by category.
    pub fn list_by_category(&self, category: EntryCategory) -> Vec<&SealedEntry> {
        self.entries
            .values()
            .filter(|e| e.category == category)
            .collect()
    }

    /// Search entries by label (case-insensitive substring match).
    pub fn search(&self, query: &str) -> Vec<&SealedEntry> {
        let query_lower = query.to_lowercase();
        self.entries
            .values()
            .filter(|e| e.label.to_lowercase().contains(&query_lower))
            .collect()
    }

    /// Number of entries.
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// Get version history for an entry.
    pub fn history(&self, id: &str) -> &[SealedEntry] {
        self.history.get(id).map(|v| v.as_slice()).unwrap_or(&[])
    }

    /// Change the master key (re-encrypts all entries).
    pub fn rekey(&mut self, new_key: [u8; 32]) -> Result<()> {
        let mut new_entries = HashMap::new();

        for (id, sealed) in &self.entries {
            // Decrypt with old key
            let encrypted: nous_crypto::EncryptedPayload =
                serde_json::from_slice(&sealed.encrypted_data)
                    .map_err(|e| Error::Crypto(format!("failed to parse payload: {}", e)))?;
            let data = decrypt(&self.master_key, &encrypted)?;

            // Re-encrypt with new key
            let new_encrypted = encrypt(&new_key, &data)?;
            let new_bytes = serde_json::to_vec(&new_encrypted)
                .map_err(|e| Error::Crypto(format!("failed to serialize payload: {}", e)))?;

            let mut new_sealed = sealed.clone();
            new_sealed.encrypted_data = new_bytes;
            new_entries.insert(id.clone(), new_sealed);
        }

        self.entries = new_entries;
        self.master_key = new_key;
        Ok(())
    }

    /// Export the vault as serializable sealed entries (no key material).
    pub fn export(&self) -> Vec<SealedEntry> {
        self.entries.values().cloned().collect()
    }

    /// Import sealed entries into the vault.
    pub fn import(&mut self, entries: Vec<SealedEntry>) {
        for entry in entries {
            self.entries.insert(entry.id.clone(), entry);
        }
    }
}

impl Drop for Vault {
    fn drop(&mut self) {
        // Zeroize the master key
        self.master_key.fill(0);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_vault() -> Vault {
        Vault::from_passphrase("test-passphrase-42")
    }

    // ── Creation ───────────────────────────────────────────────

    #[test]
    fn create_from_passphrase() {
        let vault = Vault::from_passphrase("my-secret");
        assert!(vault.is_empty());
    }

    #[test]
    fn different_passphrases_different_keys() {
        let v1 = Vault::from_passphrase("password1");
        let v2 = Vault::from_passphrase("password2");
        assert_ne!(v1.master_key, v2.master_key);
    }

    #[test]
    fn same_passphrase_same_key() {
        let v1 = Vault::from_passphrase("consistent");
        let v2 = Vault::from_passphrase("consistent");
        assert_eq!(v1.master_key, v2.master_key);
    }

    // ── Put / Get ──────────────────────────────────────────────

    #[test]
    fn put_and_get() {
        let mut vault = test_vault();
        vault
            .put(
                "key1",
                "My Private Key",
                EntryCategory::PrivateKey,
                b"secret-key-data",
            )
            .unwrap();

        let entry = vault.get("key1").unwrap();
        assert_eq!(entry.data, b"secret-key-data");
        assert_eq!(entry.label, "My Private Key");
        assert_eq!(entry.category, EntryCategory::PrivateKey);
        assert_eq!(entry.version, 1);
    }

    #[test]
    fn put_rejects_empty_id() {
        let mut vault = test_vault();
        assert!(
            vault
                .put("", "label", EntryCategory::Note, b"data")
                .is_err()
        );
    }

    #[test]
    fn put_rejects_empty_data() {
        let mut vault = test_vault();
        assert!(vault.put("id", "label", EntryCategory::Note, b"").is_err());
    }

    #[test]
    fn get_nonexistent_fails() {
        let vault = test_vault();
        assert!(vault.get("nonexistent").is_err());
    }

    #[test]
    fn wrong_key_fails_decryption() {
        let mut vault1 = Vault::from_passphrase("correct");
        vault1
            .put("key1", "test", EntryCategory::Note, b"secret")
            .unwrap();

        // Export and import into vault with different key
        let exported = vault1.export();
        let mut vault2 = Vault::from_passphrase("wrong");
        vault2.import(exported);

        assert!(vault2.get("key1").is_err());
    }

    // ── Versioning ─────────────────────────────────────────────

    #[test]
    fn update_increments_version() {
        let mut vault = test_vault();
        vault
            .put("key1", "v1", EntryCategory::Note, b"version-1")
            .unwrap();
        vault
            .put("key1", "v2", EntryCategory::Note, b"version-2")
            .unwrap();

        let entry = vault.get("key1").unwrap();
        assert_eq!(entry.version, 2);
        assert_eq!(entry.data, b"version-2");
    }

    #[test]
    fn update_preserves_created_at() {
        let mut vault = test_vault();
        vault
            .put("key1", "first", EntryCategory::Note, b"data1")
            .unwrap();
        let created = vault.get("key1").unwrap().created_at;

        vault
            .put("key1", "second", EntryCategory::Note, b"data2")
            .unwrap();
        assert_eq!(vault.get("key1").unwrap().created_at, created);
    }

    #[test]
    fn history_tracks_previous_versions() {
        let mut vault = test_vault();
        vault
            .put("key1", "v1", EntryCategory::Note, b"data1")
            .unwrap();
        vault
            .put("key1", "v2", EntryCategory::Note, b"data2")
            .unwrap();
        vault
            .put("key1", "v3", EntryCategory::Note, b"data3")
            .unwrap();

        let history = vault.history("key1");
        assert_eq!(history.len(), 2); // v1 and v2 are in history, v3 is current
    }

    // ── Remove ─────────────────────────────────────────────────

    #[test]
    fn remove_entry() {
        let mut vault = test_vault();
        vault
            .put("key1", "test", EntryCategory::Note, b"data")
            .unwrap();
        assert!(vault.remove("key1"));
        assert!(!vault.contains("key1"));
        assert_eq!(vault.len(), 0);
    }

    #[test]
    fn remove_nonexistent() {
        let mut vault = test_vault();
        assert!(!vault.remove("nonexistent"));
    }

    #[test]
    fn remove_archives_to_history() {
        let mut vault = test_vault();
        vault
            .put("key1", "test", EntryCategory::Note, b"data")
            .unwrap();
        vault.remove("key1");
        assert_eq!(vault.history("key1").len(), 1);
    }

    // ── Search / Filter ────────────────────────────────────────

    #[test]
    fn list_by_category() {
        let mut vault = test_vault();
        vault
            .put("k1", "Key 1", EntryCategory::PrivateKey, b"data1")
            .unwrap();
        vault
            .put("k2", "Key 2", EntryCategory::PrivateKey, b"data2")
            .unwrap();
        vault
            .put("n1", "Note 1", EntryCategory::Note, b"data3")
            .unwrap();

        assert_eq!(vault.list_by_category(EntryCategory::PrivateKey).len(), 2);
        assert_eq!(vault.list_by_category(EntryCategory::Note).len(), 1);
        assert_eq!(vault.list_by_category(EntryCategory::Password).len(), 0);
    }

    #[test]
    fn search_by_label() {
        let mut vault = test_vault();
        vault
            .put(
                "k1",
                "Ethereum Private Key",
                EntryCategory::PrivateKey,
                b"data1",
            )
            .unwrap();
        vault
            .put(
                "k2",
                "Solana Private Key",
                EntryCategory::PrivateKey,
                b"data2",
            )
            .unwrap();
        vault
            .put("n1", "Recovery Seed", EntryCategory::Seed, b"data3")
            .unwrap();

        let results = vault.search("private key");
        assert_eq!(results.len(), 2);

        let results = vault.search("ethereum");
        assert_eq!(results.len(), 1);

        let results = vault.search("nonexistent");
        assert_eq!(results.len(), 0);
    }

    // ── Rekey ──────────────────────────────────────────────────

    #[test]
    fn rekey_preserves_data() {
        let mut vault = Vault::from_passphrase("old-pass");
        vault
            .put("key1", "test", EntryCategory::Note, b"secret-data")
            .unwrap();

        let new_key = {
            let mut hasher = Sha256::new();
            hasher.update(b"nous-vault-master-key-v1");
            hasher.update(b"new-pass");
            hasher.finalize().into()
        };

        vault.rekey(new_key).unwrap();

        // Data should still be readable with new key
        let entry = vault.get("key1").unwrap();
        assert_eq!(entry.data, b"secret-data");
    }

    // ── Export / Import ────────────────────────────────────────

    #[test]
    fn export_and_import() {
        let mut vault1 = test_vault();
        vault1
            .put("k1", "Key 1", EntryCategory::PrivateKey, b"data1")
            .unwrap();
        vault1
            .put("k2", "Key 2", EntryCategory::Note, b"data2")
            .unwrap();

        let exported = vault1.export();
        assert_eq!(exported.len(), 2);

        let mut vault2 = test_vault(); // Same passphrase
        vault2.import(exported);

        let entry = vault2.get("k1").unwrap();
        assert_eq!(entry.data, b"data1");
    }

    // ── Content Hash ───────────────────────────────────────────

    #[test]
    fn content_hash_is_deterministic() {
        let mut vault = test_vault();
        vault
            .put("k1", "test", EntryCategory::Note, b"hello world")
            .unwrap();

        let hash = &vault.entries["k1"].content_hash;
        let expected = hex::encode(Sha256::digest(b"hello world"));
        assert_eq!(hash, &expected);
    }

    // ── Serialization ──────────────────────────────────────────

    #[test]
    fn sealed_entry_serializes() {
        let mut vault = test_vault();
        vault
            .put("k1", "test", EntryCategory::PrivateKey, b"data")
            .unwrap();

        let sealed = &vault.entries["k1"];
        let json = serde_json::to_string(sealed).unwrap();
        let restored: SealedEntry = serde_json::from_str(&json).unwrap();
        assert_eq!(restored.id, "k1");
        assert_eq!(restored.category, EntryCategory::PrivateKey);
    }

    #[test]
    fn category_serializes() {
        let cat = EntryCategory::PrivateKey;
        let json = serde_json::to_string(&cat).unwrap();
        let restored: EntryCategory = serde_json::from_str(&json).unwrap();
        assert_eq!(restored, EntryCategory::PrivateKey);
    }
}
