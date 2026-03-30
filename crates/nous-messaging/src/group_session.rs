//! Group session manager: orchestrates E2E encrypted group messaging.
//!
//! Combines pairwise Double Ratchet sessions (for secure key distribution)
//! with Sender Keys (for efficient O(1) group encryption). Each member
//! maintains:
//! - Their own sender key (for encrypting outgoing messages)
//! - A store of other members' sender keys (for decrypting incoming)
//! - Pairwise ratchet sessions for distributing key material
//!
//! Flow:
//! 1. Member joins group → generates sender key
//! 2. Distributes sender key to all members via pairwise ratchet sessions
//! 3. Encrypts group messages with sender key (O(1) per message)
//! 4. On member removal → all remaining members rotate sender keys

use std::collections::HashSet;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use nous_core::{Error, Result};

use crate::sender_key::{SenderKey, SenderKeyDistribution, SenderKeyMessage, SenderKeyStore};

/// Tracks the state of a group encryption session for one member.
pub struct GroupSession {
    pub group_id: String,
    pub self_did: String,
    pub sender_key: SenderKey,
    pub key_store: SenderKeyStore,
    pub members: HashSet<String>,
    pub created_at: DateTime<Utc>,
    /// DIDs that need to receive our current sender key distribution
    pending_distributions: HashSet<String>,
    /// History of key rotations
    rotation_count: u32,
}

/// A pending key distribution: the encrypted sender key material
/// that needs to be sent to a specific member via their pairwise channel.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PendingDistribution {
    pub target_did: String,
    pub distribution: SenderKeyDistribution,
}

/// Result of processing an incoming group message.
#[derive(Debug)]
pub struct DecryptedGroupMessage {
    pub sender_did: String,
    pub plaintext: Vec<u8>,
    pub generation: u32,
    pub iteration: u32,
}

impl GroupSession {
    /// Create a new group session. The creator is the first member.
    pub fn create(self_did: &str, group_id: &str) -> Self {
        let sender_key = SenderKey::generate(self_did, group_id);
        let mut members = HashSet::new();
        members.insert(self_did.to_string());

        Self {
            group_id: group_id.into(),
            self_did: self_did.into(),
            sender_key,
            key_store: SenderKeyStore::new(),
            members,
            created_at: Utc::now(),
            pending_distributions: HashSet::new(),
            rotation_count: 0,
        }
    }

    /// Add a member to the group. Queues a sender key distribution for them.
    pub fn add_member(&mut self, did: &str) -> Result<()> {
        if self.members.contains(did) {
            return Err(Error::InvalidInput("member already in group".into()));
        }
        self.members.insert(did.to_string());
        self.pending_distributions.insert(did.to_string());
        Ok(())
    }

    /// Remove a member from the group. Triggers sender key rotation for all remaining members.
    pub fn remove_member(&mut self, did: &str) -> Result<()> {
        if !self.members.contains(did) {
            return Err(Error::NotFound("member not in group".into()));
        }
        if did == self.self_did {
            return Err(Error::InvalidInput("cannot remove self".into()));
        }

        self.members.remove(did);
        self.key_store.remove_sender(did, &self.group_id);
        self.pending_distributions.remove(did);

        // Rotate our sender key (post-compromise security)
        self.sender_key.rotate();
        self.rotation_count += 1;

        // Queue redistribution to all remaining members
        for member in &self.members {
            if member != &self.self_did {
                self.pending_distributions.insert(member.clone());
            }
        }

        Ok(())
    }

    /// Get pending sender key distributions that need to be sent.
    /// In production, each distribution would be encrypted with the
    /// pairwise Double Ratchet session to that member.
    pub fn drain_pending_distributions(&mut self) -> Vec<PendingDistribution> {
        let dist = self.sender_key.to_distribution();
        let pending: Vec<PendingDistribution> = self
            .pending_distributions
            .drain()
            .map(|did| PendingDistribution {
                target_did: did,
                distribution: dist.clone(),
            })
            .collect();
        pending
    }

    /// Check if there are pending distributions to send.
    pub fn has_pending_distributions(&self) -> bool {
        !self.pending_distributions.is_empty()
    }

    /// Process an incoming sender key distribution from another member.
    pub fn receive_distribution(&mut self, dist: &SenderKeyDistribution) -> Result<()> {
        if !self.members.contains(&dist.sender_did) {
            return Err(Error::PermissionDenied(
                "distribution from non-member".into(),
            ));
        }
        if dist.group_id != self.group_id {
            return Err(Error::InvalidInput("group ID mismatch".into()));
        }
        self.key_store.process_distribution(dist);
        Ok(())
    }

    /// Encrypt a message for the group.
    pub fn encrypt(&mut self, plaintext: &[u8]) -> Result<SenderKeyMessage> {
        self.sender_key.encrypt(plaintext)
    }

    /// Decrypt an incoming group message.
    pub fn decrypt(&mut self, msg: &SenderKeyMessage) -> Result<DecryptedGroupMessage> {
        if msg.group_id != self.group_id {
            return Err(Error::InvalidInput("group ID mismatch".into()));
        }
        if !self.members.contains(&msg.sender_did) {
            return Err(Error::PermissionDenied("message from non-member".into()));
        }

        let plaintext = self.key_store.decrypt(msg)?;

        Ok(DecryptedGroupMessage {
            sender_did: msg.sender_did.clone(),
            plaintext,
            generation: msg.generation,
            iteration: msg.iteration,
        })
    }

    /// Check if we have a sender key for a specific member.
    pub fn has_key_for(&self, did: &str) -> bool {
        self.key_store.has_key(did, &self.group_id)
    }

    /// Number of members in the group.
    pub fn member_count(&self) -> usize {
        self.members.len()
    }

    /// Number of sender key rotations that have occurred.
    pub fn rotation_count(&self) -> u32 {
        self.rotation_count
    }

    /// List all member DIDs.
    pub fn member_dids(&self) -> Vec<&str> {
        self.members.iter().map(|s| s.as_str()).collect()
    }

    /// Check if a DID is a member.
    pub fn is_member(&self, did: &str) -> bool {
        self.members.contains(did)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn create_group_session() {
        let session = GroupSession::create("did:key:alice", "group:eng");
        assert_eq!(session.member_count(), 1);
        assert!(session.is_member("did:key:alice"));
        assert_eq!(session.rotation_count(), 0);
    }

    #[test]
    fn add_member_queues_distribution() {
        let mut session = GroupSession::create("did:key:alice", "group:eng");
        session.add_member("did:key:bob").unwrap();
        assert_eq!(session.member_count(), 2);
        assert!(session.has_pending_distributions());

        let pending = session.drain_pending_distributions();
        assert_eq!(pending.len(), 1);
        assert_eq!(pending[0].target_did, "did:key:bob");
        assert!(!session.has_pending_distributions());
    }

    #[test]
    fn add_duplicate_member_fails() {
        let mut session = GroupSession::create("did:key:alice", "group:eng");
        assert!(session.add_member("did:key:alice").is_err());
    }

    #[test]
    fn remove_member_rotates_key() {
        let mut session = GroupSession::create("did:key:alice", "group:eng");
        session.add_member("did:key:bob").unwrap();
        session.add_member("did:key:carol").unwrap();
        session.drain_pending_distributions(); // clear

        session.remove_member("did:key:carol").unwrap();
        assert_eq!(session.member_count(), 2);
        assert_eq!(session.rotation_count(), 1);
        assert!(!session.is_member("did:key:carol"));

        // Should have pending distribution to bob (rotated key)
        let pending = session.drain_pending_distributions();
        assert_eq!(pending.len(), 1);
        assert_eq!(pending[0].target_did, "did:key:bob");
    }

    #[test]
    fn cannot_remove_self() {
        let mut session = GroupSession::create("did:key:alice", "group:eng");
        assert!(session.remove_member("did:key:alice").is_err());
    }

    #[test]
    fn remove_non_member_fails() {
        let mut session = GroupSession::create("did:key:alice", "group:eng");
        assert!(session.remove_member("did:key:bob").is_err());
    }

    #[test]
    fn encrypt_and_decrypt_full_flow() {
        let group = "group:eng";

        // Alice creates the group
        let mut alice = GroupSession::create("did:key:alice", group);
        alice.add_member("did:key:bob").unwrap();

        // Alice distributes her sender key to Bob
        let alice_distributions = alice.drain_pending_distributions();
        assert_eq!(alice_distributions.len(), 1);

        // Bob creates his session and receives Alice's key
        let mut bob = GroupSession::create("did:key:bob", group);
        bob.add_member("did:key:alice").unwrap();
        bob.receive_distribution(&alice_distributions[0].distribution)
            .unwrap();

        // Bob distributes his key to Alice
        let bob_distributions = bob.drain_pending_distributions();
        alice
            .receive_distribution(&bob_distributions[0].distribution)
            .unwrap();

        // Alice sends an encrypted message
        let encrypted = alice.encrypt(b"hello team").unwrap();
        let decrypted = bob.decrypt(&encrypted).unwrap();
        assert_eq!(decrypted.plaintext, b"hello team");
        assert_eq!(decrypted.sender_did, "did:key:alice");

        // Bob sends an encrypted message
        let encrypted2 = bob.encrypt(b"hey alice").unwrap();
        let decrypted2 = alice.decrypt(&encrypted2).unwrap();
        assert_eq!(decrypted2.plaintext, b"hey alice");
        assert_eq!(decrypted2.sender_did, "did:key:bob");
    }

    #[test]
    fn three_member_group() {
        let group = "group:eng";

        let mut alice = GroupSession::create("did:key:alice", group);
        let mut bob = GroupSession::create("did:key:bob", group);
        let mut carol = GroupSession::create("did:key:carol", group);

        // Add all members
        alice.add_member("did:key:bob").unwrap();
        alice.add_member("did:key:carol").unwrap();
        bob.add_member("did:key:alice").unwrap();
        bob.add_member("did:key:carol").unwrap();
        carol.add_member("did:key:alice").unwrap();
        carol.add_member("did:key:bob").unwrap();

        // Distribute all keys
        let alice_dists = alice.drain_pending_distributions();
        let bob_dists = bob.drain_pending_distributions();
        let carol_dists = carol.drain_pending_distributions();

        for d in &alice_dists {
            if d.target_did == "did:key:bob" {
                bob.receive_distribution(&d.distribution).unwrap();
            } else {
                carol.receive_distribution(&d.distribution).unwrap();
            }
        }
        for d in &bob_dists {
            if d.target_did == "did:key:alice" {
                alice.receive_distribution(&d.distribution).unwrap();
            } else {
                carol.receive_distribution(&d.distribution).unwrap();
            }
        }
        for d in &carol_dists {
            if d.target_did == "did:key:alice" {
                alice.receive_distribution(&d.distribution).unwrap();
            } else {
                bob.receive_distribution(&d.distribution).unwrap();
            }
        }

        // Alice sends to the group
        let msg = alice.encrypt(b"standup time").unwrap();
        assert_eq!(bob.decrypt(&msg).unwrap().plaintext, b"standup time");
        assert_eq!(carol.decrypt(&msg).unwrap().plaintext, b"standup time");

        // Carol sends to the group
        let msg2 = carol.encrypt(b"all clear").unwrap();
        assert_eq!(alice.decrypt(&msg2).unwrap().plaintext, b"all clear");
        assert_eq!(bob.decrypt(&msg2).unwrap().plaintext, b"all clear");
    }

    #[test]
    fn member_removal_prevents_decryption() {
        let group = "group:eng";

        let mut alice = GroupSession::create("did:key:alice", group);
        let mut bob = GroupSession::create("did:key:bob", group);
        let mut carol = GroupSession::create("did:key:carol", group);

        // Setup (same as above, abbreviated)
        alice.add_member("did:key:bob").unwrap();
        alice.add_member("did:key:carol").unwrap();
        bob.add_member("did:key:alice").unwrap();
        bob.add_member("did:key:carol").unwrap();
        carol.add_member("did:key:alice").unwrap();
        carol.add_member("did:key:bob").unwrap();

        let alice_dists = alice.drain_pending_distributions();
        let bob_dists = bob.drain_pending_distributions();
        let carol_dists = carol.drain_pending_distributions();

        for d in &alice_dists {
            if d.target_did == "did:key:bob" {
                bob.receive_distribution(&d.distribution).unwrap();
            } else {
                carol.receive_distribution(&d.distribution).unwrap();
            }
        }
        for d in &bob_dists {
            if d.target_did == "did:key:alice" {
                alice.receive_distribution(&d.distribution).unwrap();
            } else {
                carol.receive_distribution(&d.distribution).unwrap();
            }
        }
        for d in &carol_dists {
            if d.target_did == "did:key:alice" {
                alice.receive_distribution(&d.distribution).unwrap();
            } else {
                bob.receive_distribution(&d.distribution).unwrap();
            }
        }

        // Verify Carol can decrypt before removal
        let pre_removal = alice.encrypt(b"before removal").unwrap();
        assert!(carol.decrypt(&pre_removal).is_ok());

        // Remove Carol
        alice.remove_member("did:key:carol").unwrap();
        bob.remove_member("did:key:carol").unwrap();

        // Distribute rotated keys between Alice and Bob
        let alice_new = alice.drain_pending_distributions();
        let bob_new = bob.drain_pending_distributions();
        for d in &alice_new {
            bob.receive_distribution(&d.distribution).unwrap();
        }
        for d in &bob_new {
            alice.receive_distribution(&d.distribution).unwrap();
        }

        // Alice sends post-removal
        let post_removal = alice.encrypt(b"carol is gone").unwrap();
        assert_eq!(
            bob.decrypt(&post_removal).unwrap().plaintext,
            b"carol is gone"
        );

        // Carol cannot decrypt post-removal messages (generation mismatch)
        assert!(carol.decrypt(&post_removal).is_err());
    }

    #[test]
    fn receive_distribution_from_non_member_fails() {
        let mut session = GroupSession::create("did:key:alice", "group:eng");
        let stranger_dist = SenderKeyDistribution {
            sender_did: "did:key:stranger".into(),
            group_id: "group:eng".into(),
            chain_key: [0u8; 32],
            generation: 0,
        };
        assert!(session.receive_distribution(&stranger_dist).is_err());
    }

    #[test]
    fn receive_distribution_wrong_group_fails() {
        let mut session = GroupSession::create("did:key:alice", "group:eng");
        session.add_member("did:key:bob").unwrap();
        let wrong_group_dist = SenderKeyDistribution {
            sender_did: "did:key:bob".into(),
            group_id: "group:other".into(),
            chain_key: [0u8; 32],
            generation: 0,
        };
        assert!(session.receive_distribution(&wrong_group_dist).is_err());
    }

    #[test]
    fn decrypt_from_non_member_fails() {
        let group = "group:eng";
        let mut alice = GroupSession::create("did:key:alice", group);
        let mut stranger = SenderKey::generate("did:key:stranger", group);
        let msg = stranger.encrypt(b"hacked").unwrap();
        assert!(alice.decrypt(&msg).is_err());
    }

    #[test]
    fn decrypt_wrong_group_fails() {
        let mut alice = GroupSession::create("did:key:alice", "group:eng");
        alice.add_member("did:key:bob").unwrap();

        let mut bob_wrong = SenderKey::generate("did:key:bob", "group:other");
        let msg = bob_wrong.encrypt(b"wrong group").unwrap();
        assert!(alice.decrypt(&msg).is_err());
    }

    #[test]
    fn member_dids_list() {
        let mut session = GroupSession::create("did:key:alice", "group:eng");
        session.add_member("did:key:bob").unwrap();
        session.add_member("did:key:carol").unwrap();

        let mut dids = session.member_dids();
        dids.sort();
        assert_eq!(dids, vec!["did:key:alice", "did:key:bob", "did:key:carol"]);
    }

    #[test]
    fn multiple_rotations() {
        let mut session = GroupSession::create("did:key:alice", "group:eng");
        session.add_member("did:key:bob").unwrap();
        session.add_member("did:key:carol").unwrap();
        session.add_member("did:key:dave").unwrap();
        session.drain_pending_distributions();

        session.remove_member("did:key:carol").unwrap();
        assert_eq!(session.rotation_count(), 1);
        session.drain_pending_distributions();

        session.remove_member("did:key:dave").unwrap();
        assert_eq!(session.rotation_count(), 2);
    }

    #[test]
    fn pending_distribution_serializes() {
        let dist = PendingDistribution {
            target_did: "did:key:bob".into(),
            distribution: SenderKeyDistribution {
                sender_did: "did:key:alice".into(),
                group_id: "group:eng".into(),
                chain_key: [42u8; 32],
                generation: 0,
            },
        };
        let json = serde_json::to_string(&dist).unwrap();
        let restored: PendingDistribution = serde_json::from_str(&json).unwrap();
        assert_eq!(restored.target_did, "did:key:bob");
        assert_eq!(restored.distribution.sender_did, "did:key:alice");
    }
}
