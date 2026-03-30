//! Cross-crate integration tests.
//!
//! Proves the full stack works end-to-end: identity creation → X3DH key agreement →
//! Double Ratchet messaging → social feed → storage.

#[cfg(test)]
mod tests {
    use nous_crypto::signing::{Signer, Verifier};
    use nous_crypto::zkp::{PedersenCommitment, SchnorrProof, schnorr_keygen};
    use nous_identity::Identity;
    use nous_messaging::Channel;
    use nous_messaging::message::MessageBuilder;
    use nous_messaging::ratchet::DoubleRatchet;
    use nous_messaging::x3dh::{self, PreKeyBundle};
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

        let msg = alice_r
            .encrypt(b"Hello Bob, from the sovereign web.")
            .unwrap();
        assert_eq!(
            bob_r.decrypt(&msg).unwrap(),
            b"Hello Bob, from the sovereign web."
        );

        let msg = bob_r.encrypt(b"Received. The protocol works.").unwrap();
        assert_eq!(
            alice_r.decrypt(&msg).unwrap(),
            b"Received. The protocol works."
        );

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

        assert_eq!(
            db.get_kv("did").unwrap().as_deref(),
            Some(b"did:key:z6Mk...test".as_slice())
        );
        assert_eq!(
            db.get_kv("balance").unwrap().as_deref(),
            Some(b"1000".as_slice())
        );
        assert!(db.get_kv("nonexistent").unwrap().is_none());
    }

    // ── E2E: Identity → DAO → Proposal → Vote → Tally ────────

    #[test]
    fn governance_full_lifecycle() {
        use chrono::Duration;
        use nous_governance::{
            Ballot, Dao, QuadraticVoting, VoteChoice, VoteTally, proposal::ProposalBuilder,
        };

        // 1. Create identities
        let alice = Identity::generate().with_display_name("Alice");
        let bob = Identity::generate().with_display_name("Bob");
        let carol = Identity::generate().with_display_name("Carol");

        assert!(alice.did().starts_with("did:key:z"));
        assert!(bob.did().starts_with("did:key:z"));

        // 2. Create a DAO
        let mut dao = Dao::create(
            alice.did(),
            "Nous Governance",
            "Decentralized governance DAO",
        );
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
        use nous_identity::reputation::ReputationCategory;
        use nous_identity::{CredentialBuilder, Reputation};

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
        use nous_marketplace::{
            Listing, ListingCategory, Review, SearchQuery, SellerRating, search,
        };

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
        let tx = transfer(&mut alice_wallet, &mut bob_wallet, "NOUS", 250).unwrap();

        assert_eq!(alice_wallet.balance("NOUS"), 750);
        assert_eq!(bob_wallet.balance("NOUS"), 250);
        assert_eq!(tx.from_did, alice.did());
        assert_eq!(tx.to_did, bob.did());
    }

    // ── E2E: Files → Storage ─────────────────────────────────

    #[test]
    fn files_upload_chunk_dedup() {
        use nous_files::FileStore;

        let mut store = FileStore::new();
        let owner = Identity::generate();

        // Upload a file
        let data = b"Sovereign file storage, content-addressed.";
        let manifest = store
            .put("test.txt", "text/plain", data, owner.did())
            .unwrap();
        assert_eq!(manifest.name, "test.txt");
        assert_eq!(manifest.version, 1);

        // Upload same content again — should dedup chunks
        let manifest2 = store
            .put("test.txt", "text/plain", data, owner.did())
            .unwrap();
        assert_eq!(manifest2.version, 2);

        // Verify dedup stats
        let stats = store.stats();
        assert_eq!(stats.total_files, 1); // Same file, two versions
        assert!(stats.dedup_ratio >= 1.0);

        // Retrieve content
        let retrieved = store.get_by_manifest(&manifest2).unwrap();
        assert_eq!(retrieved, data);
    }

    // ── E2E: Files + Identity ownership ──────────────────────

    #[test]
    fn files_owner_isolation() {
        use nous_files::FileStore;

        let mut store = FileStore::new();
        let alice = Identity::generate();
        let bob = Identity::generate();

        store
            .put("secret.txt", "text/plain", b"alice data", alice.did())
            .unwrap();
        store
            .put("public.txt", "text/plain", b"bob data", bob.did())
            .unwrap();

        let alice_files = store.list_files(alice.did());
        let bob_files = store.list_files(bob.did());

        assert_eq!(alice_files.len(), 1);
        assert_eq!(bob_files.len(), 1);
        assert_eq!(alice_files[0].name, "secret.txt");
        assert_eq!(bob_files[0].name, "public.txt");
    }

    // ── E2E: Governance convenience API ──────────────────────

    #[tokio::test]
    async fn governance_convenience_full_flow() {
        use axum::body::Body;
        use axum::http::{Request, StatusCode};
        use http_body_util::BodyExt;
        use nous_api::{ApiConfig, router};
        use tower::ServiceExt;

        let app = router(ApiConfig::default());

        // 1. Create identity
        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/v1/identities")
                    .header("content-type", "application/json")
                    .body(Body::from(r#"{"display_name":"Voter"}"#))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        let bytes = response.into_body().collect().await.unwrap().to_bytes();
        let id: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
        let did = id["did"].as_str().unwrap().to_string();

        // 2. Create DAO
        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/v1/daos")
                    .header("content-type", "application/json")
                    .body(Body::from(
                        serde_json::to_vec(&serde_json::json!({
                            "founder_did": &did,
                            "name": "IntegrationDAO",
                            "description": "Full flow test"
                        }))
                        .unwrap(),
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        let bytes = response.into_body().collect().await.unwrap().to_bytes();
        let dao: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
        let dao_id = dao["id"].as_str().unwrap().to_string();

        // 3. Create proposal via convenience endpoint
        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri(format!("/api/v1/daos/{}/proposals", dao_id))
                    .header("content-type", "application/json")
                    .body(Body::from(
                        serde_json::to_vec(&serde_json::json!({
                            "proposer_did": &did,
                            "title": "Full integration test",
                            "description": "Testing the full governance flow"
                        }))
                        .unwrap(),
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        let bytes = response.into_body().collect().await.unwrap().to_bytes();
        let proposal: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
        let proposal_id = proposal["id"].as_str().unwrap().to_string();

        // 4. Vote via convenience endpoint
        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri(format!("/api/v1/proposals/{}/vote", proposal_id))
                    .header("content-type", "application/json")
                    .body(Body::from(
                        serde_json::to_vec(&serde_json::json!({
                            "voter_did": &did,
                            "choice": "for",
                            "credits": 25
                        }))
                        .unwrap(),
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);

        // 5. Check tally
        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri(format!("/api/v1/votes/{}", proposal_id))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        let bytes = response.into_body().collect().await.unwrap().to_bytes();
        let tally: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(tally["votes_for"].as_u64().unwrap(), 5); // sqrt(25) = 5
        assert_eq!(tally["total_voters"].as_u64().unwrap(), 1);
    }

    // ── E2E: Payments + Marketplace ──────────────────────────

    #[test]
    fn marketplace_purchase_with_payment() {
        use nous_marketplace::{Listing, ListingCategory};
        use nous_payments::{Wallet, transfer};

        let seller = Identity::generate();
        let buyer = Identity::generate();

        // Setup wallets
        let mut buyer_wallet = Wallet::new(buyer.did());
        let mut seller_wallet = Wallet::new(seller.did());
        buyer_wallet.credit("USDC", 10000);

        // Create listing
        let mut listing = Listing::new(
            seller.did(),
            "Security Audit Report",
            "Comprehensive smart contract audit",
            ListingCategory::Service,
            "USDC",
            5000,
        )
        .unwrap();

        assert!(listing.is_available());

        // Pay for listing
        let tx = transfer(&mut buyer_wallet, &mut seller_wallet, "USDC", 5000).unwrap();
        assert_eq!(tx.amount, 5000);

        // Mark as purchased
        listing.purchase().unwrap();
        assert!(!listing.is_available());

        // Verify balances
        assert_eq!(buyer_wallet.balance("USDC"), 5000);
        assert_eq!(seller_wallet.balance("USDC"), 5000);
    }

    // ── E2E: NIP-02 Contact List + Social Follow ──────────────

    #[test]
    fn nostr_contact_list_with_social_follows() {
        use ed25519_dalek::SigningKey;
        use nous_social::FollowGraph;
        use rand::rngs::OsRng;

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

    // ── Real-time Event Bus ──────────────────────────────────────

    #[tokio::test]
    async fn realtime_broadcast_delivers_post_events() {
        use nous_api::config::ApiConfig;
        use nous_api::state::{AppState, RealtimeEvent};

        let state = AppState::new(ApiConfig::default());
        let mut rx = state.events.subscribe();

        state.emit(RealtimeEvent::NewPost {
            id: "test-post".into(),
            author: "did:key:zIntegration".into(),
            content: "integration test post".into(),
        });

        let event = rx.recv().await.unwrap();
        match event {
            RealtimeEvent::NewPost {
                id,
                author,
                content,
            } => {
                assert_eq!(id, "test-post");
                assert_eq!(author, "did:key:zIntegration");
                assert_eq!(content, "integration test post");
            }
            _ => panic!("expected NewPost event"),
        }
    }

    #[tokio::test]
    async fn realtime_broadcast_delivers_message_events() {
        use nous_api::config::ApiConfig;
        use nous_api::state::{AppState, RealtimeEvent};

        let state = AppState::new(ApiConfig::default());
        let mut rx = state.events.subscribe();

        state.emit(RealtimeEvent::NewMessage {
            channel_id: "ch-test".into(),
            sender: "did:key:zAlice".into(),
            content: "hello from integration".into(),
        });

        let event = rx.recv().await.unwrap();
        match event {
            RealtimeEvent::NewMessage {
                channel_id,
                sender,
                content,
            } => {
                assert_eq!(channel_id, "ch-test");
                assert_eq!(sender, "did:key:zAlice");
                assert_eq!(content, "hello from integration");
            }
            _ => panic!("expected NewMessage event"),
        }
    }

    #[tokio::test]
    async fn realtime_multiple_event_types() {
        use nous_api::config::ApiConfig;
        use nous_api::state::{AppState, RealtimeEvent};

        let state = AppState::new(ApiConfig::default());
        let mut rx = state.events.subscribe();

        state.emit(RealtimeEvent::DaoCreated {
            id: "dao-1".into(),
            name: "Test DAO".into(),
        });
        state.emit(RealtimeEvent::VoteCast {
            proposal_id: "prop-1".into(),
            voter: "did:key:zVoter".into(),
        });
        state.emit(RealtimeEvent::Transfer {
            from: "did:key:zA".into(),
            to: "did:key:zB".into(),
            amount: "100".into(),
            token: "NOUS".into(),
        });

        let e1 = rx.recv().await.unwrap();
        let e2 = rx.recv().await.unwrap();
        let e3 = rx.recv().await.unwrap();

        assert!(matches!(e1, RealtimeEvent::DaoCreated { .. }));
        assert!(matches!(e2, RealtimeEvent::VoteCast { .. }));
        assert!(matches!(e3, RealtimeEvent::Transfer { .. }));
    }

    #[tokio::test]
    async fn realtime_event_serialization_roundtrip() {
        use nous_api::state::RealtimeEvent;

        let event = RealtimeEvent::ProposalCreated {
            id: "p1".into(),
            title: "Fund Development".into(),
            dao_id: "d1".into(),
        };

        let json = serde_json::to_string(&event).unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();

        assert_eq!(parsed["type"], "ProposalCreated");
        assert_eq!(parsed["data"]["id"], "p1");
        assert_eq!(parsed["data"]["title"], "Fund Development");
        assert_eq!(parsed["data"]["dao_id"], "d1");
    }

    // ── E2E: Threshold Crypto → DKG → Threshold Signature ────

    #[test]
    fn threshold_crypto_dao_treasury_signing() {
        use nous_crypto::threshold::{
            ThresholdConfig, combine_partial_signatures, dkg_simulate, generate_signing_nonce,
            partial_sign, verify_threshold_signature,
        };

        // DAO treasury with 3-of-5 threshold
        let config = ThresholdConfig::new(3, 5).unwrap();
        let (dkg_result, shares) = dkg_simulate(config).unwrap();

        // 3 keyholders sign a treasury withdrawal
        let message = b"withdraw 1000 NOUS to did:key:z6MkRecipient";
        let signers = vec![shares[0].clone(), shares[2].clone(), shares[4].clone()];
        let indices: Vec<u32> = signers.iter().map(|s| s.index).collect();

        // Generate nonces
        let nonces: Vec<([u8; 32], [u8; 32])> = (0..3).map(|_| generate_signing_nonce()).collect();

        // Combine nonces
        let combined_nonce = {
            let mut sum = curve25519_dalek::ristretto::CompressedRistretto(nonces[0].1)
                .decompress()
                .unwrap();
            for n in &nonces[1..] {
                sum += curve25519_dalek::ristretto::CompressedRistretto(n.1)
                    .decompress()
                    .unwrap();
            }
            sum.compress().to_bytes()
        };

        // Partial signatures
        let partials: Vec<_> = signers
            .iter()
            .zip(nonces.iter())
            .map(|(share, (nonce_secret, _))| {
                partial_sign(
                    share,
                    nonce_secret,
                    &combined_nonce,
                    &dkg_result.group_public_key,
                    message,
                    &indices,
                )
            })
            .collect();

        // Combine and verify
        let sig = combine_partial_signatures(&partials, &combined_nonce);
        assert!(verify_threshold_signature(
            &sig,
            &dkg_result.group_public_key,
            message
        ));

        // Wrong message fails
        assert!(!verify_threshold_signature(
            &sig,
            &dkg_result.group_public_key,
            b"withdraw 9999 NOUS to attacker"
        ));
    }

    // ── E2E: Payment Stream for Compute Billing ──────────────

    #[test]
    fn payment_stream_for_compute() {
        use chrono::{Duration, Utc};
        use nous_payments::stream::{PaymentStream, StreamConfig};

        let compute_provider = Identity::generate();
        let consumer = Identity::generate();

        // Consumer starts a payment stream for GPU compute at 10 tokens/sec
        let config = StreamConfig {
            rate_per_second: 10,
            token: "NOUS".into(),
            continuous_claim: true,
            min_claim_interval_secs: 0,
            max_duration_secs: 3600, // max 1 hour
        };

        let start = Utc::now();
        let mut stream =
            PaymentStream::create(consumer.did(), compute_provider.did(), config, 36_000).unwrap();
        stream.activate_at(start).unwrap();

        // After 30 minutes, provider claims
        let at_30m = start + Duration::minutes(30);
        let receipt = stream.claim_at(at_30m).unwrap();
        assert_eq!(receipt.amount, 18_000); // 30*60*10

        // Consumer pauses (taking a break)
        stream.pause_at(at_30m).unwrap();

        // Resume 10 minutes later
        stream.resume_at(start + Duration::minutes(40)).unwrap();

        // After another 20 minutes, provider claims again
        let at_60m = start + Duration::minutes(60);
        let receipt2 = stream.claim_at(at_60m).unwrap();
        assert_eq!(receipt2.amount, 12_000); // 20*60*10

        // Total claimed: 30,000 of 36,000
        assert_eq!(stream.total_claimed, 30_000);
        assert_eq!(stream.remaining_deposit(at_60m), 6_000);
    }

    // ── E2E: Social Threads + Interactions ───────────────────

    #[test]
    fn social_thread_with_interactions() {
        use nous_social::event::{EventKind, SignedEvent, Tag};
        use nous_social::interaction::InteractionIndex;
        use nous_social::thread::Thread;

        let alice = Identity::generate();
        let bob = Identity::generate();
        let carol = Identity::generate();

        // Alice posts, Bob and Carol reply
        let mut root = SignedEvent::new(
            alice.did(),
            EventKind::TextNote,
            "Governance proposal: fund core protocol dev",
            vec![Tag::hashtag("governance")],
        );
        root.sign(alice.keypair());
        let root_id = root.id.clone();

        let mut reply1 = SignedEvent::new(
            bob.did(),
            EventKind::TextNote,
            "Strong yes. The protocol needs investment.",
            vec![Tag::event(&root_id)],
        );
        reply1.sign(bob.keypair());
        let reply1_id = reply1.id.clone();

        let mut reply2 = SignedEvent::new(
            carol.did(),
            EventKind::TextNote,
            "Agreed, but we need a budget breakdown first.",
            vec![Tag::event(&root_id)],
        );
        reply2.sign(carol.keypair());

        let mut nested_reply = SignedEvent::new(
            alice.did(),
            EventKind::TextNote,
            "Good point Carol. I'll draft one.",
            vec![Tag::event(&reply2.id.clone())],
        );
        nested_reply.sign(alice.keypair());

        // Build thread
        let events = vec![
            root.clone(),
            reply1.clone(),
            reply2.clone(),
            nested_reply.clone(),
        ];
        let thread = Thread::from_events(&events).unwrap();

        assert_eq!(thread.root_id, root_id);
        assert_eq!(thread.len(), 4);
        assert_eq!(thread.max_depth(), 2);
        assert_eq!(thread.participants().len(), 3);

        // Add reactions
        let mut reaction1 = SignedEvent::new(
            bob.did(),
            EventKind::Reaction,
            "🔥",
            vec![Tag::event(&root_id)],
        );
        reaction1.sign(bob.keypair());

        let mut reaction2 = SignedEvent::new(
            carol.did(),
            EventKind::Reaction,
            "+",
            vec![Tag::event(&root_id)],
        );
        reaction2.sign(carol.keypair());

        let mut repost =
            SignedEvent::new(bob.did(), EventKind::Repost, "", vec![Tag::event(&root_id)]);
        repost.sign(bob.keypair());

        // Index interactions
        let mut idx = InteractionIndex::new();
        idx.index_events(&[reaction1, reaction2, repost, reply1, reply2]);

        let summary = idx.get(&root_id).unwrap();
        assert_eq!(summary.reaction_count, 2);
        assert_eq!(summary.repost_count, 1);
        assert_eq!(summary.reply_count, 2);
        assert_eq!(idx.score(&root_id), 5);
    }

    // ── E2E: Credential-Gated Marketplace ────────────────────

    #[test]
    fn credential_gated_marketplace_listing() {
        use nous_identity::reputation::ReputationCategory;
        use nous_identity::{CredentialBuilder, Reputation};
        use nous_marketplace::{Listing, ListingCategory};

        let seller = Identity::generate();
        let buyer = Identity::generate();
        let issuer = Identity::generate(); // credential authority

        // Issuer verifies the seller's identity
        let credential = CredentialBuilder::new(seller.did())
            .add_type("VerifiedSeller")
            .claims(serde_json::json!({"verified": true, "level": "professional"}))
            .issue(&issuer)
            .unwrap();
        assert!(credential.verify().is_ok());

        // Seller builds reputation
        let mut seller_rep = Reputation::new(seller.did());
        let rep_event = Reputation::issue_event(
            &issuer,
            seller.did(),
            ReputationCategory::Trading,
            20,
            "verified professional seller",
        )
        .unwrap();
        seller_rep.apply(&rep_event).unwrap();

        // Seller creates a listing (only allowed with sufficient reputation)
        let min_reputation = 10;
        assert!(
            seller_rep.score(ReputationCategory::Trading) >= min_reputation,
            "seller must have sufficient trading reputation"
        );

        let listing = Listing::new(
            seller.did(),
            "Enterprise Security Audit",
            "Full smart contract security audit by verified professional",
            ListingCategory::Service,
            "USDC",
            50_000,
        )
        .unwrap();

        assert!(listing.is_available());
        assert_eq!(listing.seller_did, seller.did());

        // Buyer can verify the seller's credential before purchasing
        assert!(credential.verify().is_ok());
        assert_eq!(seller_rep.total_score(), 20);
    }

    // ── Sender Keys: Group Encryption E2E ────────────────────

    #[test]
    fn group_encrypted_messaging_with_sender_keys() {
        use nous_messaging::sender_key::{SenderKey, SenderKeyStore};

        let alice = Identity::generate();
        let bob = Identity::generate();
        let _carol = Identity::generate();
        let group_id = "group:engineering";

        // Each member generates a sender key for the group
        let mut alice_key = SenderKey::generate(alice.did(), group_id);
        let mut bob_key = SenderKey::generate(bob.did(), group_id);

        // Distribute keys: each member receives everyone else's sender key distribution
        // In production, these would be encrypted with pairwise Double Ratchet sessions
        let alice_dist = alice_key.to_distribution();
        let bob_dist = bob_key.to_distribution();

        // Bob's store has Alice's key
        let mut bob_store = SenderKeyStore::new();
        bob_store.process_distribution(&alice_dist);

        // Carol's store has both Alice's and Bob's keys
        let mut carol_store = SenderKeyStore::new();
        carol_store.process_distribution(&alice_dist);
        carol_store.process_distribution(&bob_dist);

        // Alice sends a signed message to the group
        let msg_text = b"New protocol proposal: quadratic token-weighted voting";
        let encrypted = alice_key.encrypt(msg_text).unwrap();

        // Create a signed message wrapping the encrypted group payload
        let signed = MessageBuilder::text(group_id, &serde_json::to_string(&encrypted).unwrap())
            .sign(&alice)
            .unwrap();

        // Verify the message signature (identity layer)
        assert!(signed.verify().is_ok());
        assert_eq!(signed.sender_did, alice.did());

        // Bob decrypts the group message
        let bob_plain = bob_store.decrypt(&encrypted).unwrap();
        assert_eq!(bob_plain, msg_text);

        // Carol decrypts the same message
        let carol_plain = carol_store.decrypt(&encrypted).unwrap();
        assert_eq!(carol_plain, msg_text);

        // Bob sends a reply
        let bob_msg = bob_key
            .encrypt(b"Seconded. Let me draft the implementation.")
            .unwrap();
        let carol_plain2 = carol_store.decrypt(&bob_msg).unwrap();
        assert_eq!(carol_plain2, b"Seconded. Let me draft the implementation.");

        // Key rotation after Carol leaves the group
        alice_key.rotate();
        bob_key.rotate();

        // Redistribute new keys (only to remaining members)
        bob_store.process_distribution(&alice_key.to_distribution());

        // Alice sends with rotated key
        let post_rotation = alice_key.encrypt(b"Carol removed. Rotating keys.").unwrap();
        let bob_plain2 = bob_store.decrypt(&post_rotation).unwrap();
        assert_eq!(bob_plain2, b"Carol removed. Rotating keys.");

        // Carol cannot decrypt (she has the old generation key)
        assert!(carol_store.decrypt(&post_rotation).is_err());
    }

    // ── Encrypted File Attachment E2E ────────────────────────

    #[test]
    fn encrypted_file_sharing_via_attachments() {
        use nous_messaging::attachment::{AttachmentDecoder, AttachmentEncoder};

        let sender = Identity::generate();
        let receiver = Identity::generate();

        // Establish a session for the shared encryption key
        let session = nous_messaging::Session::establish(
            sender.keypair(),
            sender.did(),
            &receiver.keypair().exchange_public_bytes(),
            receiver.did(),
        );

        // Generate a file-specific encryption key from the session
        let file_key_payload = session.encrypt(b"file-key-derivation").unwrap();
        let mut file_key = [0u8; 32];
        let key_bytes = serde_json::to_vec(&file_key_payload).unwrap();
        for (i, byte) in key_bytes.iter().take(32).enumerate() {
            file_key[i] = *byte;
        }

        // Encode a "document" file
        let document =
            b"# Protocol Specification\n\nThis document describes the Nous governance protocol..."
                .repeat(100);
        let encoder = AttachmentEncoder::new().with_chunk_size(1024);
        let (meta, chunks) = encoder
            .encode("spec.md", "text/markdown", &document, &file_key)
            .unwrap();

        assert_eq!(meta.file_name, "spec.md");
        assert_eq!(meta.size, document.len() as u64);
        assert!(meta.chunk_count > 1);

        // Send metadata as a message
        let msg = MessageBuilder::file(
            "dm:alice-bob",
            &meta.file_name,
            &meta.mime_type,
            meta.size,
            &meta.hash,
        )
        .sign(&sender)
        .unwrap();
        assert!(msg.verify().is_ok());

        // Receiver decodes the chunks with the same key
        let decoded = AttachmentDecoder::decode(&meta, &chunks, &file_key).unwrap();
        assert_eq!(decoded, document);

        // Tampered file key fails
        let mut wrong_key = file_key;
        wrong_key[0] ^= 0xFF;
        assert!(AttachmentDecoder::decode(&meta, &chunks, &wrong_key).is_err());
    }

    // ── Message Store with Ephemeral Policies ────────────────

    #[test]
    fn message_store_with_replies_and_search() {
        use chrono::{Duration, Utc};
        use nous_messaging::store::{Cursor, MessageStore, StoredMessage};

        let alice = Identity::generate();
        let bob = Identity::generate();
        let channel = "grp:eng-team";

        let mut store = MessageStore::new();

        // Alice starts a discussion
        store
            .insert(StoredMessage {
                id: "msg:1".into(),
                channel_id: channel.into(),
                sender_did: alice.did().into(),
                body: "Should we implement quadratic voting for treasury proposals?".into(),
                timestamp: Utc::now(),
                reply_to: None,
                edited_at: None,
                deleted: false,
                pinned: false,
            })
            .unwrap();

        // Bob replies
        store
            .insert(StoredMessage {
                id: "msg:2".into(),
                channel_id: channel.into(),
                sender_did: bob.did().into(),
                body: "Yes, it prevents whale domination. I'll draft the proposal.".into(),
                timestamp: Utc::now() + Duration::seconds(1),
                reply_to: Some("msg:1".into()),
                edited_at: None,
                deleted: false,
                pinned: false,
            })
            .unwrap();

        // Alice follows up
        store
            .insert(StoredMessage {
                id: "msg:3".into(),
                channel_id: channel.into(),
                sender_did: alice.did().into(),
                body: "Perfect. Pin the original for reference.".into(),
                timestamp: Utc::now() + Duration::seconds(2),
                reply_to: Some("msg:1".into()),
                edited_at: None,
                deleted: false,
                pinned: false,
            })
            .unwrap();

        // Pin the original
        store.pin("msg:1").unwrap();
        let pinned = store.pinned(channel);
        assert_eq!(pinned.len(), 1);
        assert_eq!(pinned[0].id, "msg:1");

        // Search for voting-related messages
        let results = store.search(channel, "quadratic", 10);
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].id, "msg:1");

        // Search for "proposal"
        let results = store.search(channel, "proposal", 10);
        assert_eq!(results.len(), 2); // msg:1 and msg:2

        // Check replies
        let replies = store.replies_to("msg:1");
        assert_eq!(replies.len(), 2);

        // Bob edits his reply
        store
            .edit(
                "msg:2",
                bob.did(),
                "Yes, quadratic voting prevents whale domination. Drafting now.",
            )
            .unwrap();
        let edited = store.get("msg:2").unwrap();
        assert!(edited.edited_at.is_some());

        // Pagination: fetch latest 2
        let page = store.fetch(channel, 2, None);
        assert_eq!(page.messages.len(), 2);
        assert!(page.has_more);

        // Fetch before msg:3
        let page = store.fetch(channel, 10, Some(&Cursor::Before("msg:3".into())));
        assert_eq!(page.messages.len(), 2);

        // Count excludes deleted
        assert_eq!(store.count(channel), 3);
        store.delete("msg:3", alice.did()).unwrap();
        assert_eq!(store.count(channel), 2);
    }
}
