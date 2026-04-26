//! Smoke test for the libp2p [`GossipNetwork`].

use ed25519_dalek::SigningKey;
use nous_pouw::net::{GossipNetwork, GossipNetworkConfig};
use nous_pouw::network::{Network, Topic};
use nous_pouw::state::WorkerId;
use rand::rngs::OsRng;
use std::time::{Duration, Instant};

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn two_nodes_gossip_a_message() {
    let _ = tracing_subscriber::fmt::try_init();

    let sk_a = SigningKey::generate(&mut OsRng);
    let sk_b = SigningKey::generate(&mut OsRng);
    let id_a = WorkerId::from_verifying_key(&sk_a.verifying_key());
    let id_b = WorkerId::from_verifying_key(&sk_b.verifying_key());

    let net_a = GossipNetwork::spawn(
        sk_a,
        GossipNetworkConfig {
            listen_addr: "/ip4/127.0.0.1/tcp/0".into(),
            bootstrap: vec![],
            mdns: false,
        },
    )
    .await
    .expect("spawn A");

    // Give A a moment to bind, then dial it from B.
    tokio::time::sleep(Duration::from_millis(150)).await;
    let dial_addr = net_a.local_addr();
    assert!(
        !dial_addr.is_empty(),
        "node A should expose a non-empty multiaddr"
    );

    let net_b = GossipNetwork::spawn(
        sk_b,
        GossipNetworkConfig {
            listen_addr: "/ip4/127.0.0.1/tcp/0".into(),
            bootstrap: vec![dial_addr],
            mdns: false,
        },
    )
    .await
    .expect("spawn B");

    // Let gossipsub mesh form (subscribe + dial + identify + IHAVE).
    tokio::time::sleep(Duration::from_millis(2_500)).await;

    // A publishes on Blocks; B should see it within a few seconds.
    let payload = b"hello-from-a".to_vec();
    net_a.publish(Topic::Blocks, id_a, payload.clone());

    let deadline = Instant::now() + Duration::from_secs(8);
    loop {
        let evts = net_b.drain();
        if let Some(e) = evts.iter().find(|e| e.topic == Topic::Blocks) {
            assert_eq!(e.payload, payload);
            return;
        }
        if Instant::now() > deadline {
            panic!("B did not receive A's block payload within 8s");
        }
        tokio::time::sleep(Duration::from_millis(200)).await;
    }
    // touch ids for clarity
    let _ = id_b;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn dropping_node_does_not_panic() {
    let sk = SigningKey::generate(&mut OsRng);
    let net = GossipNetwork::spawn(
        sk,
        GossipNetworkConfig {
            listen_addr: "/ip4/127.0.0.1/tcp/0".into(),
            bootstrap: vec![],
            mdns: false,
        },
    )
    .await
    .expect("spawn");
    drop(net);
    // If we got here without panic, the swarm shut down cleanly.
}
