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

    // ── libp2p Node + Wire Message Integration ────────────────

    #[tokio::test]
    async fn two_nodes_create_with_unique_peer_ids() {
        use nous_net::{NodeConfig, NousNode};

        let config = NodeConfig::default();
        let (node1, _rx1) = NousNode::new(&config).unwrap();
        let (node2, _rx2) = NousNode::new(&config).unwrap();

        assert_ne!(node1.local_peer_id(), node2.local_peer_id());
    }

    #[tokio::test]
    async fn nodes_subscribe_to_all_topics() {
        use nous_net::{NodeConfig, NousNode, NousTopic};

        let config = NodeConfig::default();
        let (mut node, _rx) = NousNode::new(&config).unwrap();
        node.subscribe_all_default().unwrap();

        let topics = NousTopic::all_default();
        assert_eq!(topics.len(), 7);
    }

    #[test]
    fn wire_message_signed_with_identity() {
        use nous_crypto::signing::Signer;
        use nous_net::{NousTopic, WireMessage};

        let identity = Identity::generate();
        let mut msg = WireMessage::new(
            NousTopic::Social,
            b"test message from sovereign web".to_vec(),
            identity.did().to_string(),
        );

        // Sign the message
        let signer = Signer::new(identity.keypair());
        let sig = signer.sign(&msg.signable_bytes());
        msg.signature = sig.as_bytes().to_vec();
        assert!(!msg.signature.is_empty());

        // Encode and decode
        let encoded = msg.encode().unwrap();
        let decoded = WireMessage::decode(&encoded).unwrap();
        assert_eq!(decoded.sender_did, identity.did());
        assert_eq!(decoded.payload, b"test message from sovereign web");
        assert_eq!(decoded.topic, NousTopic::Social);
        assert!(!decoded.signature.is_empty());
    }

    #[test]
    fn wire_messages_across_all_topics() {
        use nous_net::{NousTopic, WireMessage};

        for topic in NousTopic::all_default() {
            let msg = WireMessage::new(
                topic.clone(),
                format!("payload for {topic}").into_bytes(),
                "did:key:z6MkTest".to_string(),
            );

            let encoded = msg.encode().unwrap();
            let decoded = WireMessage::decode(&encoded).unwrap();
            assert_eq!(decoded.topic, topic);
        }
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

    // ── E2E: Identity → DAO → Proposal → Vote → Tally ────────

    #[test]
    fn governance_full_lifecycle() {
        use nous_governance::{
            Ballot, Dao, QuadraticVoting, VoteChoice, VoteTally,
            proposal::ProposalBuilder,
        };
        use chrono::Duration;

        // 1. Create identities
        let alice = Identity::generate().with_display_name("Alice");
        let bob = Identity::generate().with_display_name("Bob");
        let carol = Identity::generate().with_display_name("Carol");

        assert!(alice.did().starts_with("did:key:z"));
        assert!(bob.did().starts_with("did:key:z"));

        // 2. Create a DAO
        let mut dao = Dao::create(alice.did(), "Nous Governance", "Decentralized governance DAO");
        assert_eq!(dao.member_count(), 1);
        assert!(dao.is_member(alice.did()));

        // 3. Add members
        dao.add_member(bob.did()).unwrap();
        dao.add_member(carol.did()).unwrap();
        assert_eq!(dao.member_count(), 3);

        // 4. Grant credits for voting
        dao.grant_credits(alice.did(), 100).unwrap();
        dao.grant_credits(bob.did(), 50).unwrap();
        dao.grant_credits(carol.did(), 25).unwrap();

        // 5. Submit a proposal
        let proposal = ProposalBuilder::new(
            &dao.id,
            "Fund Protocol Development",
            "Allocate 10,000 NOUS tokens to core protocol development",
        )
        .voting_duration(Duration::days(1))
        .quorum(0.5)
        .threshold(0.6)
        .submit(&alice)
        .unwrap();

        assert!(proposal.verify().is_ok());
        assert_eq!(proposal.dao_id, dao.id);

        // 6. Cast votes (quadratic voting: votes = sqrt(credits))
        let alice_votes = QuadraticVoting::credits_to_votes(100);
        assert_eq!(alice_votes, 10); // sqrt(100) = 10

        let bob_votes = QuadraticVoting::credits_to_votes(50);
        assert_eq!(bob_votes, 7); // sqrt(50) ≈ 7

        let carol_votes = QuadraticVoting::credits_to_votes(25);
        assert_eq!(carol_votes, 5); // sqrt(25) = 5

        let mut tally = VoteTally::new(&proposal.id, 0.5, 0.6);

        let alice_ballot = Ballot::new(&proposal.id, &alice, VoteChoice::For, 100).unwrap();
        assert!(alice_ballot.verify().is_ok());
        tally.cast(alice_ballot).unwrap();

        let bob_ballot = Ballot::new(&proposal.id, &bob, VoteChoice::For, 50).unwrap();
        tally.cast(bob_ballot).unwrap();

        let carol_ballot = Ballot::new(&proposal.id, &carol, VoteChoice::Against, 25).unwrap();
        tally.cast(carol_ballot).unwrap();

        // 7. Tally results
        let result = tally.tally(3);
        assert_eq!(result.total_voters, 3);
        assert_eq!(result.votes_for, alice_votes + bob_votes);
        assert_eq!(result.votes_against, carol_votes);
        assert!(result.passed); // 17 for vs 5 against, well above 60% threshold
    }

    // ── E2E: Identity → Credential → Reputation ──────────────

    #[test]
    fn credential_and_reputation_lifecycle() {
        use nous_identity::{CredentialBuilder, Reputation};
        use nous_identity::reputation::ReputationCategory;

        // 1. Create issuer and subject
        let issuer = Identity::generate().with_display_name("University");
        let subject = Identity::generate().with_display_name("Student");

        // 2. Issue a credential
        let credential = CredentialBuilder::new(subject.did())
            .add_type("DegreeCredential")
            .claims(serde_json::json!({
                "degree": "Computer Science",
                "gpa": 3.9,
                "graduated": true
            }))
            .issue(&issuer)
            .unwrap();

        assert!(credential.verify().is_ok());
        assert_eq!(credential.subject_did(), subject.did());
        assert_eq!(credential.issuer_did(), issuer.did());
        assert!(!credential.is_expired());

        // 3. Build reputation
        let mut reputation = Reputation::new(subject.did());
        assert_eq!(reputation.total_score(), 0);

        let event = Reputation::issue_event(
            &issuer,
            subject.did(),
            ReputationCategory::Development,
            10,
            "excellent protocol contribution",
        )
        .unwrap();

        assert!(event.verify().is_ok());
        reputation.apply(&event).unwrap();
        assert_eq!(reputation.total_score(), 10);
        assert_eq!(reputation.score(ReputationCategory::Development), 10);

        // Add governance reputation
        let event = Reputation::issue_event(
            &issuer,
            subject.did(),
            ReputationCategory::Governance,
            5,
            "thoughtful proposal review",
        )
        .unwrap();
        reputation.apply(&event).unwrap();
        assert_eq!(reputation.total_score(), 15);
        assert_eq!(reputation.events().len(), 2);
    }

    // ── E2E: Marketplace listing → Review → Rating ────────────

    #[test]
    fn marketplace_full_lifecycle() {
        use nous_marketplace::{Listing, ListingCategory, Review, SellerRating, SearchQuery, search};

        let seller = Identity::generate();
        let buyer = Identity::generate();

        // 1. Create listing
        let listing = Listing::new(
            seller.did(),
            "Rust Programming Course",
            "Comprehensive async Rust course with exercises",
            ListingCategory::Digital,
            "USDC",
            5000,
        )
        .unwrap()
        .with_tag("rust")
        .with_tag("programming")
        .with_tag("async");

        assert!(listing.is_available());
        assert!(listing.matches_search("rust"));
        assert!(listing.matches_search("async"));
        assert!(!listing.matches_search("python"));

        // 2. Search for listing
        let listings = vec![listing.clone()];
        let query = SearchQuery::new().text("Rust");
        let results = search(&listings, &query);
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].title, "Rust Programming Course");

        // 3. Purchase
        let mut listing = listing;
        listing.purchase().unwrap();
        assert!(!listing.is_available());

        // 4. Leave a review
        let review = Review::new(
            &listing.id,
            buyer.did(),
            seller.did(),
            5,
            "Excellent course, learned so much about async Rust",
        )
        .unwrap();

        // 5. Compute seller rating
        let rating = SellerRating::compute(seller.did(), &[review]);
        assert_eq!(rating.total_reviews, 1);
        assert_eq!(rating.average_rating, 5.0);
    }

    // ── E2E: NIP-04 Encrypted DM Flow ────────────────────────

    #[test]
    fn nostr_encrypted_dm_flow() {
        use ed25519_dalek::SigningKey;
        use rand::rngs::OsRng;

        let alice_key = SigningKey::generate(&mut OsRng);
        let bob_key = SigningKey::generate(&mut OsRng);
        let alice_pubkey = hex::encode(alice_key.verifying_key().as_bytes());
        let bob_pubkey = hex::encode(bob_key.verifying_key().as_bytes());

        // 1. Alice creates an encrypted DM event
        let event = nous_nostr::EventBuilder::encrypted_dm(
            "hello bob, sovereign and private",
            &bob_pubkey,
            &alice_key,
        )
        .unwrap()
        .sign(&alice_key);

        assert_eq!(event.kind, nous_nostr::Kind::ENCRYPTED_DM);
        assert!(event.verify());
        assert!(event.has_tag("p", &bob_pubkey));

        // 2. Bob decrypts
        let plaintext =
            nous_nostr::nip04::decrypt(&event.content, &alice_pubkey, &bob_key).unwrap();
        assert_eq!(plaintext, "hello bob, sovereign and private");

        // 3. Bob replies with encrypted DM
        let reply = nous_nostr::EventBuilder::encrypted_dm(
            "received, the sovereign web works",
            &alice_pubkey,
            &bob_key,
        )
        .unwrap()
        .sign(&bob_key);

        let reply_plaintext =
            nous_nostr::nip04::decrypt(&reply.content, &bob_pubkey, &alice_key).unwrap();
        assert_eq!(reply_plaintext, "received, the sovereign web works");
    }

    // ── E2E: Payments Wallet → Transfer ──────────────────────

    #[test]
    fn payment_transfer_flow() {
        use nous_payments::{Wallet, transfer};

        let alice = Identity::generate();
        let bob = Identity::generate();

        let mut alice_wallet = Wallet::new(alice.did());
        let mut bob_wallet = Wallet::new(bob.did());

        // Credit alice
        alice_wallet.credit("NOUS", 1000);
        assert_eq!(alice_wallet.balance("NOUS"), 1000);

        // Transfer from alice to bob
        let tx = transfer(
            &mut alice_wallet,
            &mut bob_wallet,
            "NOUS",
            250,
        )
        .unwrap();

        assert_eq!(alice_wallet.balance("NOUS"), 750);
        assert_eq!(bob_wallet.balance("NOUS"), 250);
        assert_eq!(tx.from_did, alice.did());
        assert_eq!(tx.to_did, bob.did());
    }

    // ── E2E: NIP-02 Contact List + Social Follow ──────────────

    #[test]
    fn nostr_contact_list_with_social_follows() {
        use ed25519_dalek::SigningKey;
        use rand::rngs::OsRng;
        use nous_social::FollowGraph;

        let alice_key = SigningKey::generate(&mut OsRng);
        let alice_pub = hex::encode(alice_key.verifying_key().as_bytes());

        let bob_pub = hex::encode(SigningKey::generate(&mut OsRng).verifying_key().as_bytes());
        let carol_pub = hex::encode(SigningKey::generate(&mut OsRng).verifying_key().as_bytes());

        // 1. Create NIP-02 contact list
        let contacts = vec![
            (bob_pub.as_str(), "wss://relay.nous.dev", "bob"),
            (carol_pub.as_str(), "", "carol"),
        ];
        let event = nous_nostr::EventBuilder::contact_list(&contacts).sign(&alice_key);

        assert_eq!(event.kind, nous_nostr::Kind::CONTACTS);
        assert_eq!(event.tags.len(), 2);
        assert!(event.verify());

        // 2. Sync to social follow graph
        let mut graph = FollowGraph::new();
        for tag in &event.tags {
            if tag.tag_name() == Some("p") {
                if let Some(pubkey) = tag.value() {
                    graph.follow(&alice_pub, pubkey);
                }
            }
        }

        assert!(graph.is_following(&alice_pub, &bob_pub));
        assert!(graph.is_following(&alice_pub, &carol_pub));
        assert_eq!(graph.following_count(&alice_pub), 2);
    }
}
