//! Sender Keys for efficient group encryption.
//!
//! Each group member generates a sender key and distributes it to all other
//! members via their 1:1 encrypted channels. When sending a message, the
//! sender encrypts once with their key — all members who hold that key can
//! decrypt. Chain ratcheting provides forward secrecy within the group.
//!
//! Based on the Signal Sender Keys protocol:
//! - O(1) encryption per message (vs O(n) for pairwise)
//! - Forward secrecy via HMAC-based chain ratcheting
//! - Key rotation on member removal for post-compromise security

use std::collections::HashMap;

use hmac::{Hmac, Mac};
use rand::rngs::OsRng;
use rand::RngCore;
use serde::{Deserialize, Serialize};
use sha2::Sha256;
use zeroize::Zeroize;

use nous_core::{Error, Result};
use nous_crypto::encryption::{self, EncryptedPayload};

/// A sender key: a chain key + signing key pair for one group member.
#[derive(Clone, Serialize, Deserialize)]
pub struct SenderKey {
    pub sender_did: String,
    pub group_id: String,
    pub chain_key: [u8; 32],
    pub generation: u32,
    pub iteration: u32,
}

impl SenderKey {
    /// Generate a new sender key for the given group.
    pub fn generate(sender_did: &str, group_id: &str) -> Self {
        let mut chain_key = [0u8; 32];
        OsRng.fill_bytes(&mut chain_key);

        Self {
            sender_did: sender_did.into(),
            group_id: group_id.into(),
            chain_key,
            generation: 0,
            iteration: 0,
        }
    }

    /// Create a distribution message containing the key material
    /// that other members need to decrypt messages from this sender.
    pub fn to_distribution(&self) -> SenderKeyDistribution {
        SenderKeyDistribution {
            sender_did: self.sender_did.clone(),
            group_id: self.group_id.clone(),
            chain_key: self.chain_key,
            generation: self.generation,
        }
    }

    /// Derive the message key for the current iteration and advance the chain.
    fn advance(&mut self) -> [u8; 32] {
        let mk = derive_message_key(&self.chain_key);
        self.chain_key = derive_chain_key(&self.chain_key);
        self.iteration += 1;
        mk
    }

    /// Encrypt a plaintext message using this sender key.
    pub fn encrypt(&mut self, plaintext: &[u8]) -> Result<SenderKeyMessage> {
        let iteration = self.iteration;
        let mut mk = self.advance();

        let payload = encryption::encrypt(&mk, plaintext)?;
        mk.zeroize();

        Ok(SenderKeyMessage {
            sender_did: self.sender_did.clone(),
            group_id: self.group_id.clone(),
            generation: self.generation,
            iteration,
            payload,
        })
    }

    /// Rotate this sender key (new generation, new random chain key).
    /// Called after a member is removed from the group.
    pub fn rotate(&mut self) {
        OsRng.fill_bytes(&mut self.chain_key);
        self.generation += 1;
        self.iteration = 0;
    }
}

impl Drop for SenderKey {
    fn drop(&mut self) {
        self.chain_key.zeroize();
    }
}

/// Distribution message: contains the key material a recipient needs
/// to start decrypting messages from a sender.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SenderKeyDistribution {
    pub sender_did: String,
    pub group_id: String,
    pub chain_key: [u8; 32],
    pub generation: u32,
}

/// An encrypted group message produced by a sender key.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SenderKeyMessage {
    pub sender_did: String,
    pub group_id: String,
    pub generation: u32,
    pub iteration: u32,
    pub payload: EncryptedPayload,
}

/// Tracks sender keys received from other group members.
/// Each member maintains one of these per group they belong to.
pub struct SenderKeyStore {
    /// (sender_did, group_id) -> receiver state
    keys: HashMap<(String, String), ReceiverState>,
}

struct ReceiverState {
    chain_key: [u8; 32],
    generation: u32,
    iteration: u32,
    /// Cached message keys for out-of-order delivery.
    skipped: HashMap<(u32, u32), [u8; 32]>,
}

const MAX_SKIP: u32 = 256;

impl SenderKeyStore {
    pub fn new() -> Self {
        Self {
            keys: HashMap::new(),
        }
    }

    /// Process a sender key distribution message from another member.
    /// This installs or updates the receiver state for that sender.
    pub fn process_distribution(&mut self, dist: &SenderKeyDistribution) {
        let key = (dist.sender_did.clone(), dist.group_id.clone());
        self.keys.insert(
            key,
            ReceiverState {
                chain_key: dist.chain_key,
                generation: dist.generation,
                iteration: 0,
                skipped: HashMap::new(),
            },
        );
    }

    /// Check whether we have a sender key for the given sender in the given group.
    pub fn has_key(&self, sender_did: &str, group_id: &str) -> bool {
        self.keys
            .contains_key(&(sender_did.to_string(), group_id.to_string()))
    }

    /// Decrypt a sender key message.
    pub fn decrypt(&mut self, msg: &SenderKeyMessage) -> Result<Vec<u8>> {
        let key = (msg.sender_did.clone(), msg.group_id.clone());
        let state = self
            .keys
            .get_mut(&key)
            .ok_or_else(|| Error::Crypto("no sender key for this sender/group".into()))?;

        if msg.generation != state.generation {
            return Err(Error::Crypto(format!(
                "generation mismatch: expected {}, got {}",
                state.generation, msg.generation
            )));
        }

        // Check skipped keys first (out-of-order delivery)
        if let Some(mut mk) = state.skipped.remove(&(msg.generation, msg.iteration)) {
            let result = encryption::decrypt(&mk, &msg.payload);
            mk.zeroize();
            return result;
        }

        if msg.iteration < state.iteration {
            return Err(Error::Crypto("message iteration already consumed".into()));
        }

        if msg.iteration - state.iteration > MAX_SKIP {
            return Err(Error::Crypto(format!(
                "skip limit exceeded: {} > {MAX_SKIP}",
                msg.iteration - state.iteration
            )));
        }

        // Advance chain, storing skipped keys
        while state.iteration < msg.iteration {
            let mk = derive_message_key(&state.chain_key);
            state
                .skipped
                .insert((state.generation, state.iteration), mk);
            state.chain_key = derive_chain_key(&state.chain_key);
            state.iteration += 1;
        }

        // Derive the target message key
        let mut mk = derive_message_key(&state.chain_key);
        state.chain_key = derive_chain_key(&state.chain_key);
        state.iteration += 1;

        let result = encryption::decrypt(&mk, &msg.payload);
        mk.zeroize();
        result
    }

    /// Remove all sender keys for a given group.
    pub fn remove_group(&mut self, group_id: &str) {
        self.keys.retain(|(_, gid), _| gid != group_id);
    }

    /// Remove a specific sender's key from a group.
    pub fn remove_sender(&mut self, sender_did: &str, group_id: &str) {
        self.keys
            .remove(&(sender_did.to_string(), group_id.to_string()));
    }

    /// Number of sender keys currently stored.
    pub fn len(&self) -> usize {
        self.keys.len()
    }

    pub fn is_empty(&self) -> bool {
        self.keys.is_empty()
    }
}

impl Default for SenderKeyStore {
    fn default() -> Self {
        Self::new()
    }
}

impl Drop for SenderKeyStore {
    fn drop(&mut self) {
        for state in self.keys.values_mut() {
            state.chain_key.zeroize();
            for mk in state.skipped.values_mut() {
                mk.zeroize();
            }
        }
    }
}

/// Derive a message key from the current chain key.
/// mk = HMAC-SHA256(chain_key, 0x01)
fn derive_message_key(chain_key: &[u8; 32]) -> [u8; 32] {
    let mut mac = Hmac::<Sha256>::new_from_slice(chain_key).expect("HMAC accepts any key length");
    mac.update(&[0x01]);
    let out = mac.finalize().into_bytes();
    let mut mk = [0u8; 32];
    mk.copy_from_slice(&out);
    mk
}

/// Derive the next chain key from the current chain key.
/// next_ck = HMAC-SHA256(chain_key, 0x02)
fn derive_chain_key(chain_key: &[u8; 32]) -> [u8; 32] {
    let mut mac = Hmac::<Sha256>::new_from_slice(chain_key).expect("HMAC accepts any key length");
    mac.update(&[0x02]);
    let out = mac.finalize().into_bytes();
    let mut ck = [0u8; 32];
    ck.copy_from_slice(&out);
    ck
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn generate_sender_key() {
        let sk = SenderKey::generate("did:key:alice", "group:123");
        assert_eq!(sk.sender_did, "did:key:alice");
        assert_eq!(sk.group_id, "group:123");
        assert_eq!(sk.generation, 0);
        assert_eq!(sk.iteration, 0);
    }

    #[test]
    fn encrypt_and_decrypt() {
        let mut sender = SenderKey::generate("did:key:alice", "group:1");
        let dist = sender.to_distribution();

        let mut store = SenderKeyStore::new();
        store.process_distribution(&dist);

        let msg = sender.encrypt(b"hello group").unwrap();
        let plaintext = store.decrypt(&msg).unwrap();
        assert_eq!(plaintext, b"hello group");
    }

    #[test]
    fn multiple_messages() {
        let mut sender = SenderKey::generate("did:key:alice", "group:1");
        let dist = sender.to_distribution();

        let mut store = SenderKeyStore::new();
        store.process_distribution(&dist);

        for i in 0..10u32 {
            let text = format!("message {i}");
            let msg = sender.encrypt(text.as_bytes()).unwrap();
            assert_eq!(store.decrypt(&msg).unwrap(), text.as_bytes());
        }
    }

    #[test]
    fn multiple_senders_in_group() {
        let group = "group:multi";

        let mut alice_key = SenderKey::generate("did:key:alice", group);
        let mut bob_key = SenderKey::generate("did:key:bob", group);

        // Carol's store has both sender keys
        let mut carol_store = SenderKeyStore::new();
        carol_store.process_distribution(&alice_key.to_distribution());
        carol_store.process_distribution(&bob_key.to_distribution());

        let m1 = alice_key.encrypt(b"from alice").unwrap();
        let m2 = bob_key.encrypt(b"from bob").unwrap();

        assert_eq!(carol_store.decrypt(&m1).unwrap(), b"from alice");
        assert_eq!(carol_store.decrypt(&m2).unwrap(), b"from bob");
    }

    #[test]
    fn out_of_order_delivery() {
        let mut sender = SenderKey::generate("did:key:alice", "group:1");
        let dist = sender.to_distribution();

        let mut store = SenderKeyStore::new();
        store.process_distribution(&dist);

        let m0 = sender.encrypt(b"first").unwrap();
        let m1 = sender.encrypt(b"second").unwrap();
        let m2 = sender.encrypt(b"third").unwrap();

        // Deliver out of order: m0, m2, m1
        assert_eq!(store.decrypt(&m0).unwrap(), b"first");
        assert_eq!(store.decrypt(&m2).unwrap(), b"third");
        assert_eq!(store.decrypt(&m1).unwrap(), b"second");
    }

    #[test]
    fn replay_rejected() {
        let mut sender = SenderKey::generate("did:key:alice", "group:1");
        let dist = sender.to_distribution();

        let mut store = SenderKeyStore::new();
        store.process_distribution(&dist);

        let msg = sender.encrypt(b"once").unwrap();
        assert!(store.decrypt(&msg).is_ok());
        assert!(store.decrypt(&msg).is_err());
    }

    #[test]
    fn unknown_sender_fails() {
        let mut sender = SenderKey::generate("did:key:alice", "group:1");
        let msg = sender.encrypt(b"hello").unwrap();

        let mut store = SenderKeyStore::new();
        assert!(store.decrypt(&msg).is_err());
    }

    #[test]
    fn key_rotation() {
        let mut sender = SenderKey::generate("did:key:alice", "group:1");
        let dist1 = sender.to_distribution();

        let mut store = SenderKeyStore::new();
        store.process_distribution(&dist1);

        let msg1 = sender.encrypt(b"before rotation").unwrap();
        assert_eq!(store.decrypt(&msg1).unwrap(), b"before rotation");

        // Rotate the key
        sender.rotate();
        assert_eq!(sender.generation, 1);
        assert_eq!(sender.iteration, 0);

        // Distribute the new key
        let dist2 = sender.to_distribution();
        store.process_distribution(&dist2);

        let msg2 = sender.encrypt(b"after rotation").unwrap();
        assert_eq!(store.decrypt(&msg2).unwrap(), b"after rotation");
    }

    #[test]
    fn old_generation_rejected_after_rotation() {
        let mut sender = SenderKey::generate("did:key:alice", "group:1");
        let dist = sender.to_distribution();

        let mut store = SenderKeyStore::new();
        store.process_distribution(&dist);

        let old_msg = sender.encrypt(b"old").unwrap();

        // Rotate and redistribute
        sender.rotate();
        store.process_distribution(&sender.to_distribution());

        // Old message should fail (generation mismatch)
        assert!(store.decrypt(&old_msg).is_err());
    }

    #[test]
    fn skip_limit_exceeded() {
        let mut sender = SenderKey::generate("did:key:alice", "group:1");
        let dist = sender.to_distribution();

        let mut store = SenderKeyStore::new();
        store.process_distribution(&dist);

        // Skip way ahead
        for _ in 0..300 {
            sender.encrypt(b"skip").unwrap();
        }

        let msg = sender.encrypt(b"too far").unwrap();
        assert!(store.decrypt(&msg).is_err());
    }

    #[test]
    fn has_key() {
        let mut store = SenderKeyStore::new();
        assert!(!store.has_key("did:key:alice", "group:1"));

        let dist = SenderKey::generate("did:key:alice", "group:1").to_distribution();
        store.process_distribution(&dist);
        assert!(store.has_key("did:key:alice", "group:1"));
    }

    #[test]
    fn remove_sender() {
        let mut store = SenderKeyStore::new();
        let dist = SenderKey::generate("did:key:alice", "group:1").to_distribution();
        store.process_distribution(&dist);

        store.remove_sender("did:key:alice", "group:1");
        assert!(!store.has_key("did:key:alice", "group:1"));
    }

    #[test]
    fn remove_group() {
        let mut store = SenderKeyStore::new();
        store.process_distribution(&SenderKey::generate("did:key:alice", "group:1").to_distribution());
        store.process_distribution(&SenderKey::generate("did:key:bob", "group:1").to_distribution());
        store.process_distribution(&SenderKey::generate("did:key:carol", "group:2").to_distribution());

        assert_eq!(store.len(), 3);
        store.remove_group("group:1");
        assert_eq!(store.len(), 1);
        assert!(store.has_key("did:key:carol", "group:2"));
    }

    #[test]
    fn distribution_serializes() {
        let sk = SenderKey::generate("did:key:alice", "group:1");
        let dist = sk.to_distribution();
        let json = serde_json::to_string(&dist).unwrap();
        let restored: SenderKeyDistribution = serde_json::from_str(&json).unwrap();
        assert_eq!(restored.sender_did, "did:key:alice");
        assert_eq!(restored.group_id, "group:1");
    }

    #[test]
    fn message_serializes() {
        let mut sender = SenderKey::generate("did:key:alice", "group:1");
        let msg = sender.encrypt(b"serde test").unwrap();
        let json = serde_json::to_string(&msg).unwrap();
        let restored: SenderKeyMessage = serde_json::from_str(&json).unwrap();
        assert_eq!(restored.sender_did, "did:key:alice");
        assert_eq!(restored.iteration, 0);
    }

    #[test]
    fn forward_secrecy_chain_advances() {
        let mut sender = SenderKey::generate("did:key:alice", "group:1");
        let chain_before = sender.chain_key;
        sender.encrypt(b"msg").unwrap();
        assert_ne!(sender.chain_key, chain_before);
    }

    #[test]
    fn different_groups_isolated() {
        let mut sender1 = SenderKey::generate("did:key:alice", "group:1");
        let mut sender2 = SenderKey::generate("did:key:alice", "group:2");

        let mut store = SenderKeyStore::new();
        store.process_distribution(&sender1.to_distribution());
        store.process_distribution(&sender2.to_distribution());

        let msg1 = sender1.encrypt(b"group 1").unwrap();
        let msg2 = sender2.encrypt(b"group 2").unwrap();

        assert_eq!(store.decrypt(&msg1).unwrap(), b"group 1");
        assert_eq!(store.decrypt(&msg2).unwrap(), b"group 2");
    }

    #[test]
    fn empty_plaintext() {
        let mut sender = SenderKey::generate("did:key:alice", "group:1");
        let dist = sender.to_distribution();

        let mut store = SenderKeyStore::new();
        store.process_distribution(&dist);

        let msg = sender.encrypt(b"").unwrap();
        assert!(store.decrypt(&msg).unwrap().is_empty());
    }

    #[test]
    fn large_plaintext() {
        let mut sender = SenderKey::generate("did:key:alice", "group:1");
        let dist = sender.to_distribution();

        let mut store = SenderKeyStore::new();
        store.process_distribution(&dist);

        let data = vec![0xAB; 100_000];
        let msg = sender.encrypt(&data).unwrap();
        assert_eq!(store.decrypt(&msg).unwrap(), data);
    }

    #[test]
    fn store_len() {
        let mut store = SenderKeyStore::new();
        assert!(store.is_empty());
        assert_eq!(store.len(), 0);

        store.process_distribution(&SenderKey::generate("a", "g").to_distribution());
        assert_eq!(store.len(), 1);
        assert!(!store.is_empty());
    }
}
