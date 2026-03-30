//! Delivery receipts — track message lifecycle: sent → delivered → read.
//!
//! Each receipt is a signed attestation from a recipient that they have
//! received or read a particular message. Receipts are lightweight,
//! cryptographically verifiable, and aggregatable.

use std::collections::{BTreeMap, HashMap, HashSet};

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use nous_crypto::signing::{Signature, Verifier};
use nous_identity::Identity;

/// The lifecycle stage of a message from the sender's perspective.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum ReceiptKind {
    /// Message accepted by the local node / relay.
    Sent,
    /// Message arrived at the recipient's device.
    Delivered,
    /// Recipient opened / viewed the message.
    Read,
}

impl ReceiptKind {
    /// Returns `true` if `self` is at least as advanced as `other`.
    pub fn at_least(&self, other: ReceiptKind) -> bool {
        *self >= other
    }
}

/// A single signed receipt from one recipient for one message.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Receipt {
    /// ID of the message this receipt is for.
    pub message_id: String,
    /// DID of the recipient who generated this receipt.
    pub recipient_did: String,
    /// What stage this receipt attests to.
    pub kind: ReceiptKind,
    /// When this receipt was created.
    pub timestamp: DateTime<Utc>,
    /// Ed25519 signature over the receipt payload.
    pub signature: Signature,
}

impl Receipt {
    /// Create and sign a new receipt.
    pub fn new(
        message_id: impl Into<String>,
        kind: ReceiptKind,
        identity: &Identity,
    ) -> Receipt {
        let message_id = message_id.into();
        let timestamp = Utc::now();
        let payload = signable_bytes(&message_id, identity.did(), kind, timestamp);
        let signature = identity.sign(&payload);

        Receipt {
            message_id,
            recipient_did: identity.did().to_string(),
            kind,
            timestamp,
            signature,
        }
    }

    /// Verify the receipt signature against the recipient's DID.
    pub fn verify(&self) -> nous_core::Result<()> {
        let key = nous_crypto::keys::did_to_public_key(&self.recipient_did)?;
        let payload = signable_bytes(
            &self.message_id,
            &self.recipient_did,
            self.kind,
            self.timestamp,
        );
        Verifier::verify(&key, &payload, &self.signature)
    }
}

fn signable_bytes(
    message_id: &str,
    recipient_did: &str,
    kind: ReceiptKind,
    timestamp: DateTime<Utc>,
) -> Vec<u8> {
    let obj = serde_json::json!({
        "message_id": message_id,
        "recipient_did": recipient_did,
        "kind": kind,
        "timestamp": timestamp,
    });
    serde_json::to_vec(&obj).expect("receipt serialization cannot fail")
}

/// Aggregate status of a single message across all recipients.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MessageStatus {
    /// Message ID.
    pub message_id: String,
    /// Per-recipient status (highest receipt kind received).
    pub recipients: HashMap<String, ReceiptKind>,
    /// Timestamps of the most recent receipt per recipient.
    pub timestamps: HashMap<String, DateTime<Utc>>,
}

impl MessageStatus {
    fn new(message_id: impl Into<String>) -> Self {
        Self {
            message_id: message_id.into(),
            recipients: HashMap::new(),
            timestamps: HashMap::new(),
        }
    }

    /// Apply a receipt, advancing the recipient's status if appropriate.
    /// Returns `true` if the status was actually advanced.
    fn apply(&mut self, receipt: &Receipt) -> bool {
        let current = self.recipients.get(&receipt.recipient_did).copied();
        if current.is_some_and(|c| c >= receipt.kind) {
            return false;
        }
        self.recipients
            .insert(receipt.recipient_did.clone(), receipt.kind);
        self.timestamps
            .insert(receipt.recipient_did.clone(), receipt.timestamp);
        true
    }

    /// The overall status: the *minimum* status across all known recipients.
    /// If no recipients are tracked, returns `None`.
    pub fn overall(&self) -> Option<ReceiptKind> {
        self.recipients.values().copied().min()
    }

    /// Count of recipients at each status level.
    pub fn counts(&self) -> (usize, usize, usize) {
        let mut sent = 0usize;
        let mut delivered = 0usize;
        let mut read = 0usize;
        for kind in self.recipients.values() {
            match kind {
                ReceiptKind::Sent => sent += 1,
                ReceiptKind::Delivered => delivered += 1,
                ReceiptKind::Read => read += 1,
            }
        }
        (sent, delivered, read)
    }

    /// Set of recipients who have read the message.
    pub fn read_by(&self) -> HashSet<&str> {
        self.recipients
            .iter()
            .filter(|(_, k)| **k == ReceiptKind::Read)
            .map(|(did, _)| did.as_str())
            .collect()
    }

    /// Set of recipients who have at least received delivery confirmation.
    pub fn delivered_to(&self) -> HashSet<&str> {
        self.recipients
            .iter()
            .filter(|(_, k)| k.at_least(ReceiptKind::Delivered))
            .map(|(did, _)| did.as_str())
            .collect()
    }
}

/// Tracks delivery receipts for many messages, indexed for fast lookup.
pub struct ReceiptTracker {
    /// message_id → aggregate status
    statuses: BTreeMap<String, MessageStatus>,
    /// Total receipts processed.
    receipt_count: u64,
}

impl ReceiptTracker {
    pub fn new() -> Self {
        Self {
            statuses: BTreeMap::new(),
            receipt_count: 0,
        }
    }

    /// Register that a message was sent to a set of recipients.
    /// This initializes their status to `Sent`.
    pub fn register_sent(
        &mut self,
        message_id: &str,
        recipient_dids: &[&str],
        timestamp: DateTime<Utc>,
    ) {
        let status = self
            .statuses
            .entry(message_id.to_string())
            .or_insert_with(|| MessageStatus::new(message_id));

        for did in recipient_dids {
            status
                .recipients
                .entry(did.to_string())
                .or_insert(ReceiptKind::Sent);
            status
                .timestamps
                .entry(did.to_string())
                .or_insert(timestamp);
        }
    }

    /// Process a signed receipt. Verifies the signature, then applies it.
    /// Returns `Ok(true)` if the status was advanced, `Ok(false)` if it was
    /// a duplicate or older receipt, `Err` if verification failed.
    pub fn process_receipt(&mut self, receipt: &Receipt) -> nous_core::Result<bool> {
        receipt.verify()?;

        let status = self
            .statuses
            .entry(receipt.message_id.clone())
            .or_insert_with(|| MessageStatus::new(&receipt.message_id));

        let advanced = status.apply(receipt);
        if advanced {
            self.receipt_count += 1;
        }
        Ok(advanced)
    }

    /// Process a receipt without verifying its signature (for locally-generated
    /// or already-verified receipts).
    pub fn apply_receipt_unchecked(&mut self, receipt: &Receipt) -> bool {
        let status = self
            .statuses
            .entry(receipt.message_id.clone())
            .or_insert_with(|| MessageStatus::new(&receipt.message_id));

        let advanced = status.apply(receipt);
        if advanced {
            self.receipt_count += 1;
        }
        advanced
    }

    /// Get the aggregate status for a specific message.
    pub fn status(&self, message_id: &str) -> Option<&MessageStatus> {
        self.statuses.get(message_id)
    }

    /// Get the highest receipt kind for a specific (message, recipient) pair.
    pub fn recipient_status(
        &self,
        message_id: &str,
        recipient_did: &str,
    ) -> Option<ReceiptKind> {
        self.statuses
            .get(message_id)
            .and_then(|s| s.recipients.get(recipient_did).copied())
    }

    /// Total number of status-advancing receipts processed.
    pub fn receipt_count(&self) -> u64 {
        self.receipt_count
    }

    /// Number of messages being tracked.
    pub fn tracked_message_count(&self) -> usize {
        self.statuses.len()
    }

    /// Remove tracking data for a message (e.g., after it's been deleted).
    pub fn remove(&mut self, message_id: &str) -> bool {
        self.statuses.remove(message_id).is_some()
    }

    /// Batch-generate delivery receipts for multiple messages at once.
    /// Returns a vec of signed receipts.
    pub fn batch_deliver(
        message_ids: &[&str],
        identity: &Identity,
    ) -> Vec<Receipt> {
        message_ids
            .iter()
            .map(|id| Receipt::new(*id, ReceiptKind::Delivered, identity))
            .collect()
    }

    /// Batch-generate read receipts for multiple messages at once.
    pub fn batch_read(
        message_ids: &[&str],
        identity: &Identity,
    ) -> Vec<Receipt> {
        message_ids
            .iter()
            .map(|id| Receipt::new(*id, ReceiptKind::Read, identity))
            .collect()
    }

    /// Get all messages where every recipient has at least reached the given
    /// status level. Useful for UI indicators (e.g., show double-check when
    /// all recipients have `Delivered`).
    pub fn messages_fully_at(&self, kind: ReceiptKind) -> Vec<&str> {
        self.statuses
            .iter()
            .filter(|(_, status)| {
                !status.recipients.is_empty()
                    && status.recipients.values().all(|k| k.at_least(kind))
            })
            .map(|(id, _)| id.as_str())
            .collect()
    }
}

impl Default for ReceiptTracker {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use nous_identity::Identity;

    fn setup() -> (Identity, Identity, Identity) {
        (
            Identity::generate(),
            Identity::generate(),
            Identity::generate(),
        )
    }

    #[test]
    fn receipt_kind_ordering() {
        assert!(ReceiptKind::Sent < ReceiptKind::Delivered);
        assert!(ReceiptKind::Delivered < ReceiptKind::Read);
        assert!(ReceiptKind::Read.at_least(ReceiptKind::Sent));
        assert!(ReceiptKind::Delivered.at_least(ReceiptKind::Delivered));
        assert!(!ReceiptKind::Sent.at_least(ReceiptKind::Read));
    }

    #[test]
    fn create_and_verify_receipt() {
        let recipient = Identity::generate();
        let receipt = Receipt::new("msg:001", ReceiptKind::Delivered, &recipient);

        assert_eq!(receipt.message_id, "msg:001");
        assert_eq!(receipt.recipient_did, recipient.did());
        assert_eq!(receipt.kind, ReceiptKind::Delivered);
        assert!(receipt.verify().is_ok());
    }

    #[test]
    fn tampered_receipt_fails_verification() {
        let recipient = Identity::generate();
        let mut receipt = Receipt::new("msg:001", ReceiptKind::Read, &recipient);

        // Tamper with the message ID.
        receipt.message_id = "msg:002".to_string();
        assert!(receipt.verify().is_err());
    }

    #[test]
    fn receipt_serializes() {
        let recipient = Identity::generate();
        let receipt = Receipt::new("msg:001", ReceiptKind::Read, &recipient);

        let json = serde_json::to_string(&receipt).unwrap();
        let restored: Receipt = serde_json::from_str(&json).unwrap();

        assert_eq!(restored.message_id, receipt.message_id);
        assert_eq!(restored.kind, receipt.kind);
        assert!(restored.verify().is_ok());
    }

    #[test]
    fn register_sent_initializes_status() {
        let mut tracker = ReceiptTracker::new();
        let now = Utc::now();

        tracker.register_sent("msg:001", &["did:key:alice", "did:key:bob"], now);

        let status = tracker.status("msg:001").unwrap();
        assert_eq!(status.recipients.len(), 2);
        assert_eq!(
            status.recipients.get("did:key:alice"),
            Some(&ReceiptKind::Sent)
        );
        assert_eq!(status.overall(), Some(ReceiptKind::Sent));
    }

    #[test]
    fn process_receipt_advances_status() {
        let (_, alice, _) = setup();
        let mut tracker = ReceiptTracker::new();
        let now = Utc::now();

        tracker.register_sent("msg:001", &[alice.did()], now);

        let receipt = Receipt::new("msg:001", ReceiptKind::Delivered, &alice);
        assert!(tracker.process_receipt(&receipt).unwrap());

        assert_eq!(
            tracker.recipient_status("msg:001", alice.did()),
            Some(ReceiptKind::Delivered)
        );
    }

    #[test]
    fn duplicate_receipt_does_not_advance() {
        let alice = Identity::generate();
        let mut tracker = ReceiptTracker::new();
        let now = Utc::now();

        tracker.register_sent("msg:001", &[alice.did()], now);

        let r1 = Receipt::new("msg:001", ReceiptKind::Delivered, &alice);
        assert!(tracker.process_receipt(&r1).unwrap());

        let r2 = Receipt::new("msg:001", ReceiptKind::Delivered, &alice);
        assert!(!tracker.process_receipt(&r2).unwrap());

        assert_eq!(tracker.receipt_count(), 1);
    }

    #[test]
    fn older_receipt_does_not_regress() {
        let alice = Identity::generate();
        let mut tracker = ReceiptTracker::new();

        // Start by processing a Read receipt.
        let read = Receipt::new("msg:001", ReceiptKind::Read, &alice);
        assert!(tracker.process_receipt(&read).unwrap());

        // A Delivered receipt should not regress the status.
        let delivered = Receipt::new("msg:001", ReceiptKind::Delivered, &alice);
        assert!(!tracker.process_receipt(&delivered).unwrap());

        assert_eq!(
            tracker.recipient_status("msg:001", alice.did()),
            Some(ReceiptKind::Read)
        );
    }

    #[test]
    fn overall_status_is_minimum() {
        let (_, alice, bob) = setup();
        let mut tracker = ReceiptTracker::new();
        let now = Utc::now();

        tracker.register_sent("msg:001", &[alice.did(), bob.did()], now);

        // Alice reads, Bob only gets delivery.
        let r1 = Receipt::new("msg:001", ReceiptKind::Read, &alice);
        let r2 = Receipt::new("msg:001", ReceiptKind::Delivered, &bob);
        tracker.process_receipt(&r1).unwrap();
        tracker.process_receipt(&r2).unwrap();

        // Overall should be Delivered (the minimum).
        assert_eq!(
            tracker.status("msg:001").unwrap().overall(),
            Some(ReceiptKind::Delivered)
        );
    }

    #[test]
    fn counts_are_correct() {
        let (_, alice, bob) = setup();
        let mut tracker = ReceiptTracker::new();
        let now = Utc::now();

        let carol = Identity::generate();
        tracker.register_sent("msg:001", &[alice.did(), bob.did(), carol.did()], now);

        tracker
            .process_receipt(&Receipt::new("msg:001", ReceiptKind::Read, &alice))
            .unwrap();
        tracker
            .process_receipt(&Receipt::new("msg:001", ReceiptKind::Delivered, &bob))
            .unwrap();
        // Carol stays at Sent.

        let (sent, delivered, read) = tracker.status("msg:001").unwrap().counts();
        assert_eq!(sent, 1); // Carol
        assert_eq!(delivered, 1); // Bob
        assert_eq!(read, 1); // Alice
    }

    #[test]
    fn read_by_and_delivered_to() {
        let (_, alice, bob) = setup();
        let mut tracker = ReceiptTracker::new();
        let now = Utc::now();

        tracker.register_sent("msg:001", &[alice.did(), bob.did()], now);

        tracker
            .process_receipt(&Receipt::new("msg:001", ReceiptKind::Read, &alice))
            .unwrap();
        tracker
            .process_receipt(&Receipt::new("msg:001", ReceiptKind::Delivered, &bob))
            .unwrap();

        let status = tracker.status("msg:001").unwrap();
        assert!(status.read_by().contains(alice.did()));
        assert!(!status.read_by().contains(bob.did()));

        // delivered_to includes both Read and Delivered.
        assert!(status.delivered_to().contains(alice.did()));
        assert!(status.delivered_to().contains(bob.did()));
    }

    #[test]
    fn batch_deliver_receipts() {
        let alice = Identity::generate();
        let receipts = ReceiptTracker::batch_deliver(&["msg:001", "msg:002", "msg:003"], &alice);

        assert_eq!(receipts.len(), 3);
        for receipt in &receipts {
            assert_eq!(receipt.kind, ReceiptKind::Delivered);
            assert_eq!(receipt.recipient_did, alice.did());
            assert!(receipt.verify().is_ok());
        }
    }

    #[test]
    fn batch_read_receipts() {
        let alice = Identity::generate();
        let receipts = ReceiptTracker::batch_read(&["msg:001", "msg:002"], &alice);

        assert_eq!(receipts.len(), 2);
        for receipt in &receipts {
            assert_eq!(receipt.kind, ReceiptKind::Read);
            assert!(receipt.verify().is_ok());
        }
    }

    #[test]
    fn messages_fully_at_status() {
        let (_, alice, bob) = setup();
        let mut tracker = ReceiptTracker::new();
        let now = Utc::now();

        tracker.register_sent("msg:001", &[alice.did(), bob.did()], now);
        tracker.register_sent("msg:002", &[alice.did(), bob.did()], now);

        // msg:001 — both delivered
        tracker
            .process_receipt(&Receipt::new("msg:001", ReceiptKind::Delivered, &alice))
            .unwrap();
        tracker
            .process_receipt(&Receipt::new("msg:001", ReceiptKind::Delivered, &bob))
            .unwrap();

        // msg:002 — only alice delivered
        tracker
            .process_receipt(&Receipt::new("msg:002", ReceiptKind::Delivered, &alice))
            .unwrap();

        let fully_delivered = tracker.messages_fully_at(ReceiptKind::Delivered);
        assert!(fully_delivered.contains(&"msg:001"));
        assert!(!fully_delivered.contains(&"msg:002"));
    }

    #[test]
    fn remove_tracking() {
        let mut tracker = ReceiptTracker::new();
        let now = Utc::now();

        tracker.register_sent("msg:001", &["did:key:alice"], now);
        assert_eq!(tracker.tracked_message_count(), 1);

        assert!(tracker.remove("msg:001"));
        assert_eq!(tracker.tracked_message_count(), 0);
        assert!(tracker.status("msg:001").is_none());

        // Removing again returns false.
        assert!(!tracker.remove("msg:001"));
    }

    #[test]
    fn unknown_message_returns_none() {
        let tracker = ReceiptTracker::new();
        assert!(tracker.status("msg:nonexistent").is_none());
        assert!(tracker.recipient_status("msg:nonexistent", "did:key:x").is_none());
    }

    #[test]
    fn apply_unchecked_works() {
        let alice = Identity::generate();
        let mut tracker = ReceiptTracker::new();

        let receipt = Receipt::new("msg:001", ReceiptKind::Read, &alice);
        assert!(tracker.apply_receipt_unchecked(&receipt));

        assert_eq!(
            tracker.recipient_status("msg:001", alice.did()),
            Some(ReceiptKind::Read)
        );
    }

    #[test]
    fn receipt_for_unknown_message_creates_entry() {
        let alice = Identity::generate();
        let mut tracker = ReceiptTracker::new();

        let receipt = Receipt::new("msg:orphan", ReceiptKind::Delivered, &alice);
        assert!(tracker.process_receipt(&receipt).unwrap());
        assert_eq!(tracker.tracked_message_count(), 1);
    }

    #[test]
    fn multi_recipient_full_lifecycle() {
        let (_, alice, bob) = setup();
        let carol = Identity::generate();
        let mut tracker = ReceiptTracker::new();
        let now = Utc::now();

        tracker.register_sent("msg:001", &[alice.did(), bob.did(), carol.did()], now);

        // Phase 1: all delivered
        for id in [&alice, &bob, &carol] {
            let r = Receipt::new("msg:001", ReceiptKind::Delivered, id);
            tracker.process_receipt(&r).unwrap();
        }
        assert_eq!(
            tracker.status("msg:001").unwrap().overall(),
            Some(ReceiptKind::Delivered)
        );

        // Phase 2: all read
        for id in [&alice, &bob, &carol] {
            let r = Receipt::new("msg:001", ReceiptKind::Read, id);
            tracker.process_receipt(&r).unwrap();
        }
        assert_eq!(
            tracker.status("msg:001").unwrap().overall(),
            Some(ReceiptKind::Read)
        );

        let (sent, delivered, read) = tracker.status("msg:001").unwrap().counts();
        assert_eq!(sent, 0);
        assert_eq!(delivered, 0);
        assert_eq!(read, 3);
    }
}
