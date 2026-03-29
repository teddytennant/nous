//! Cross-crate integration tests.
//!
//! Proves the full stack works end-to-end: identity creation → X3DH key agreement →
//! Double Ratchet messaging → social feed → storage.

#[cfg(test)]
mod tests {
    use nous_crypto::signing::{Signer, Verifier};
    use nous_crypto::zkp::{schnorr_keygen, PedersenCommitment, SchnorrProof};
    use nous_identity::Identity;
    use nous_messaging::message::MessageBuilder;
    use nous_messaging::ratchet::DoubleRatchet;
    use nous_messaging::x3dh::{self, PreKeyBundle};
    use nous_messaging::Channel;
    use nous_social::{Feed, FollowGraph, PostBuilder};
    use nous_storage::crdt::GCounter;
    use nous_storage::sqlite::Database;
    use rand::rngs::OsRng;
    use x25519_dalek::StaticSecret;

    // ── Identity → Crypto ──────────────────────────────────────

    #[test]
    fn identity_signs_and_verifies() {
        let alice = Identity::generate();
        let signer = Signer::new(alice.keypair());
        let sig = signer.sign(b"cross-crate signing test");

        let vk = alice.keypair().verifying_key();
        assert!(Verifier::verify(&vk, b"cross-crate signing test", &sig).is_ok());
    }

    // ── X3DH → Double Ratchet → Encrypted Messaging ───────────

    #[test]
    fn full_x3dh_to_double_ratchet_session() {
        let alice = Identity::generate();
        let bob = Identity::generate();

        let bob_spk = StaticSecret::random_from_rng(OsRng);
        let bob_opk = StaticSecret::random_from_rng(OsRng);
        let bundle = PreKeyBundle::create(bob.keypair(), &bob_spk, Some((1, &bob_opk)));

        let alice_out = x3dh::initiate(alice.keypair(), &bundle).unwrap();
        let bob_secret = x3dh::accept(
            bob.keypair(),
            &bob_spk,
            Some(&bob_opk),
            &alice_out.identity_key,
            &alice_out.ephemeral_key,
        )
        .unwrap();

        assert_eq!(alice_out.shared_secret, bob_secret);

        let bob_spk_pub = x25519_dalek::PublicKey::from(&bob_spk).to_bytes();
        let mut alice_r = DoubleRatchet::init_initiator(alice_out.shared_secret, &bob_spk_pub);
        let mut bob_r = DoubleRatchet::init_responder(bob_secret, bob_spk);

        let msg = alice_r.encrypt(b"Hello Bob, from the sovereign web.").unwrap();
        assert_eq!(bob_r.decrypt(&msg).unwrap(), b"Hello Bob, from the sovereign web.");

        let msg = bob_r.encrypt(b"Received. The protocol works.").unwrap();
        assert_eq!(alice_r.decrypt(&msg).unwrap(), b"Received. The protocol works.");

        for i in 0..10 {
            let t = format!("Alice {i}");
            let m = alice_r.encrypt(t.as_bytes()).unwrap();
            assert_eq!(bob_r.decrypt(&m).unwrap(), t.as_bytes());

            let t = format!("Bob {i}");
            let m = bob_r.encrypt(t.as_bytes()).unwrap();
            assert_eq!(alice_r.decrypt(&m).unwrap(), t.as_bytes());
        }
    }

    #[test]
    fn x3dh_without_opk() {
        let alice = Identity::generate();
        let bob = Identity::generate();

        let bob_spk = StaticSecret::random_from_rng(OsRng);
        let bundle = PreKeyBundle::create(bob.keypair(), &bob_spk, None);

        let alice_out = x3dh::initiate(alice.keypair(), &bundle).unwrap();
        let bob_secret = x3dh::accept(
            bob.keypair(),
            &bob_spk,
            None,
            &alice_out.identity_key,
            &alice_out.ephemeral_key,
        )
        .unwrap();

        assert_eq!(alice_out.shared_secret, bob_secret);

        let bob_spk_pub = x25519_dalek::PublicKey::from(&bob_spk).to_bytes();
        let mut a = DoubleRatchet::init_initiator(alice_out.shared_secret, &bob_spk_pub);
        let mut b = DoubleRatchet::init_responder(bob_secret, bob_spk);

        let msg = a.encrypt(b"no OPK, still secure").unwrap();
        assert_eq!(b.decrypt(&msg).unwrap(), b"no OPK, still secure");
    }

    // ── Identity → Signed Messages ─────────────────────────────

    #[test]
    fn signed_messages_verify() {
        let sender = Identity::generate();
        let channel = Channel::direct(sender.did(), "did:key:zpeer");

        let msg = MessageBuilder::text(&channel.id, "cross-crate message test")
            .sign(&sender)
            .unwrap();

        assert!(msg.verify().is_ok());
        assert_eq!(msg.sender_did, sender.did());
    }

    // ── Social → Feed ──────────────────────────────────────────

    #[test]
    fn social_feed_with_follows() {
        let alice = Identity::generate();
        let bob = Identity::generate();

        let mut feed = Feed::new();
        let mut graph = FollowGraph::new();

        feed.insert(PostBuilder::new(bob.did(), "First post").build());
        feed.insert(
            PostBuilder::new(bob.did(), "Second post")
                .hashtag("nous")
                .build(),
        );

        graph.follow(alice.did(), bob.did());
        assert!(graph.is_following(alice.did(), bob.did()));

        let following: Vec<String> = graph
            .following_of(alice.did())
            .into_iter()
            .map(|s| s.to_string())
            .collect();
        let timeline = feed.timeline(&following, 10);
        assert_eq!(timeline.len(), 2);

        assert_eq!(feed.by_hashtag("nous").len(), 1);
    }

    // ── ZK Proofs ──────────────────────────────────────────────

    #[test]
    fn schnorr_proof_end_to_end() {
        let (secret, public) = schnorr_keygen();
        let proof = SchnorrProof::prove(&secret, &public, b"I control this identity");
        assert!(proof.verify(&public, b"I control this identity"));
        assert!(!proof.verify(&public, b"wrong"));
    }

    #[test]
    fn pedersen_commitment_private_voting() {
        let (c1, o1) = PedersenCommitment::commit(5);
        let (c2, o2) = PedersenCommitment::commit(3);

        assert!(c1.verify(&o1));
        assert!(c2.verify(&o2));

        let total = c1.add(&c2).unwrap();

        let sum_blinding = {
            use curve25519_dalek::scalar::Scalar;
            let a = Scalar::from_bytes_mod_order(o1.blinding);
            let b = Scalar::from_bytes_mod_order(o2.blinding);
            (a + b).to_bytes()
        };
        let expected = PedersenCommitment::commit_with(8, &sum_blinding);
        assert_eq!(total.commitment, expected.commitment);
    }

    // ── Storage CRDTs ──────────────────────────────────────────

    #[test]
    fn crdt_convergence() {
        let mut a = GCounter::new();
        let mut b = GCounter::new();

        a.increment("node_a");
        a.increment_by("node_a", 4);
        b.increment_by("node_b", 3);

        a.merge(&b);
        b.merge(&a);

        assert_eq!(a.value(), b.value());
        assert_eq!(a.value(), 8);
    }

    // ── KV Store ───────────────────────────────────────────────

    #[test]
    fn kv_store_persists() {
        let dir = tempfile::tempdir().unwrap();
        let db = Database::open(dir.path().join("test.db").as_path()).unwrap();

        db.put_kv("did", b"did:key:z6Mk...test").unwrap();
        db.put_kv("balance", b"1000").unwrap();

        assert_eq!(db.get_kv("did").unwrap().as_deref(), Some(b"did:key:z6Mk...test".as_slice()));
        assert_eq!(db.get_kv("balance").unwrap().as_deref(), Some(b"1000".as_slice()));
        assert!(db.get_kv("nonexistent").unwrap().is_none());
    }
}
