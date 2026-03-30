use criterion::{Criterion, black_box, criterion_group, criterion_main};

use nous_crypto::keys::KeyPair;
use nous_identity::Identity;
use nous_messaging::mention::extract_mentions;
use nous_messaging::ratchet::DoubleRatchet;
use nous_messaging::receipt::{Receipt, ReceiptKind, ReceiptTracker};
use nous_messaging::sender_key::{SenderKey, SenderKeyStore};
use nous_messaging::store::{MessageStore, StoredMessage};
use nous_messaging::x3dh::{self, PreKeyBundle};
use x25519_dalek::StaticSecret;

// ── X3DH Key Exchange ────────────────────────────────────────────

fn bench_x3dh_key_exchange(c: &mut Criterion) {
    c.bench_function("x3dh_key_exchange", |b| {
        let bob_kp = KeyPair::generate();
        let bob_spk = StaticSecret::random_from_rng(rand::rngs::OsRng);
        let bob_opk = StaticSecret::random_from_rng(rand::rngs::OsRng);
        let bundle = PreKeyBundle::create(&bob_kp, &bob_spk, Some((1, &bob_opk)));

        b.iter(|| {
            let alice_kp = KeyPair::generate();
            let _ = black_box(x3dh::initiate(&alice_kp, &bundle));
        });
    });
}

// ── Double Ratchet ──────────────────────────────────────────────

fn bench_double_ratchet_encrypt(c: &mut Criterion) {
    let bob_kp = KeyPair::generate();
    let bob_spk = StaticSecret::random_from_rng(rand::rngs::OsRng);
    let bundle = PreKeyBundle::create(&bob_kp, &bob_spk, None);

    let alice_kp = KeyPair::generate();
    let output = x3dh::initiate(&alice_kp, &bundle).unwrap();

    let plaintext = b"Hello, this is a test message for benchmarking the Double Ratchet.";

    c.bench_function("double_ratchet_encrypt", |b| {
        let mut ratchet =
            DoubleRatchet::init_initiator(output.shared_secret, &bundle.signed_pre_key.key);
        b.iter(|| {
            black_box(ratchet.encrypt(plaintext).unwrap());
        });
    });
}

fn bench_double_ratchet_roundtrip(c: &mut Criterion) {
    let bob_kp = KeyPair::generate();
    let bob_spk = StaticSecret::random_from_rng(rand::rngs::OsRng);
    let bob_spk_clone = bob_spk.clone();
    let bundle = PreKeyBundle::create(&bob_kp, &bob_spk, None);

    let alice_kp = KeyPair::generate();
    let output = x3dh::initiate(&alice_kp, &bundle).unwrap();

    let plaintext = b"Benchmark roundtrip message";

    c.bench_function("double_ratchet_encrypt_decrypt", |b| {
        b.iter(|| {
            let mut alice =
                DoubleRatchet::init_initiator(output.shared_secret, &bundle.signed_pre_key.key);
            let mut bob =
                DoubleRatchet::init_responder(output.shared_secret, bob_spk_clone.clone());

            let msg = alice.encrypt(plaintext).unwrap();
            let decrypted = bob.decrypt(&msg).unwrap();
            black_box(decrypted);
        });
    });
}

// ── Sender Key Group Encryption ─────────────────────────────────

fn bench_sender_key_encrypt(c: &mut Criterion) {
    let plaintext = b"Group message content for benchmarking sender key encryption";

    c.bench_function("sender_key_encrypt", |b| {
        let mut key = SenderKey::generate("did:key:alice", "group:bench");
        b.iter(|| {
            black_box(key.encrypt(plaintext).unwrap());
        });
    });
}

fn bench_sender_key_decrypt(c: &mut Criterion) {
    let plaintext = b"Group message for decryption benchmark";
    let mut sender = SenderKey::generate("did:key:alice", "group:bench");
    let dist = sender.to_distribution();

    c.bench_function("sender_key_decrypt", |b| {
        let mut store = SenderKeyStore::new();
        store.process_distribution(&dist);
        b.iter(|| {
            let msg = sender.encrypt(plaintext).unwrap();
            let decrypted = store.decrypt(&msg).unwrap();
            black_box(decrypted);
        });
    });
}

// ── Message Store ───────────────────────────────────────────────

fn bench_message_store_insert(c: &mut Criterion) {
    c.bench_function("message_store_insert_1000", |b| {
        b.iter(|| {
            let mut store = MessageStore::new();
            for i in 0..1000 {
                store
                    .insert(StoredMessage {
                        id: format!("msg:{i}"),
                        channel_id: "ch:bench".to_string(),
                        sender_did: "did:key:alice".to_string(),
                        body: format!("Message number {i}"),
                        timestamp: chrono::Utc::now(),
                        reply_to: None,
                        edited_at: None,
                        deleted: false,
                        pinned: false,
                    })
                    .unwrap();
            }
            black_box(&store);
        });
    });
}

fn bench_message_store_search(c: &mut Criterion) {
    let mut store = MessageStore::new();
    for i in 0..5000 {
        store
            .insert(StoredMessage {
                id: format!("msg:{i}"),
                channel_id: "ch:bench".to_string(),
                sender_did: "did:key:alice".to_string(),
                body: if i % 100 == 0 {
                    format!("IMPORTANT: critical update {i}")
                } else {
                    format!("Regular message number {i}")
                },
                timestamp: chrono::Utc::now(),
                reply_to: None,
                edited_at: None,
                deleted: false,
                pinned: false,
            })
            .unwrap();
    }

    c.bench_function("message_store_search_5000", |b| {
        b.iter(|| {
            black_box(store.search("ch:bench", "IMPORTANT", 100));
        });
    });
}

fn bench_message_store_paginate(c: &mut Criterion) {
    let mut store = MessageStore::new();
    for i in 0..5000 {
        store
            .insert(StoredMessage {
                id: format!("msg:{i:05}"),
                channel_id: "ch:bench".to_string(),
                sender_did: "did:key:alice".to_string(),
                body: format!("Message {i}"),
                timestamp: chrono::Utc::now(),
                reply_to: None,
                edited_at: None,
                deleted: false,
                pinned: false,
            })
            .unwrap();
    }

    c.bench_function("message_store_paginate_50", |b| {
        b.iter(|| {
            black_box(store.fetch("ch:bench", 50, None));
        });
    });
}

// ── Delivery Receipts ───────────────────────────────────────────

fn bench_receipt_create_and_verify(c: &mut Criterion) {
    let identity = Identity::generate();

    c.bench_function("receipt_create_and_verify", |b| {
        b.iter(|| {
            let receipt = Receipt::new("msg:bench", ReceiptKind::Delivered, &identity);
            black_box(receipt.verify().unwrap());
        });
    });
}

fn bench_receipt_tracker_100_messages(c: &mut Criterion) {
    c.bench_function("receipt_tracker_100_messages", |b| {
        let recipients: Vec<Identity> = (0..5).map(|_| Identity::generate()).collect();
        let now = chrono::Utc::now();

        b.iter(|| {
            let mut tracker = ReceiptTracker::new();
            let dids: Vec<&str> = recipients.iter().map(|id| id.did()).collect();

            for i in 0..100 {
                let msg_id = format!("msg:{i}");
                tracker.register_sent(&msg_id, &dids, now);

                for recipient in &recipients {
                    let receipt = Receipt::new(&msg_id, ReceiptKind::Delivered, recipient);
                    tracker.apply_receipt_unchecked(&receipt);
                }
            }
            black_box(tracker.tracked_message_count());
        });
    });
}

// ── Mention Extraction ──────────────────────────────────────────

fn bench_mention_extraction(c: &mut Criterion) {
    let text = "Hey @alice and @bob, check out @did:key:z6MkTest123 — @everyone should see this. \
                Also cc @carol @dave @eve. Not an email: user@example.com. @here please review.";

    c.bench_function("mention_extraction", |b| {
        b.iter(|| {
            black_box(extract_mentions(text));
        });
    });
}

criterion_group!(
    benches,
    bench_x3dh_key_exchange,
    bench_double_ratchet_encrypt,
    bench_double_ratchet_roundtrip,
    bench_sender_key_encrypt,
    bench_sender_key_decrypt,
    bench_message_store_insert,
    bench_message_store_search,
    bench_message_store_paginate,
    bench_receipt_create_and_verify,
    bench_receipt_tracker_100_messages,
    bench_mention_extraction,
);

criterion_main!(benches);
