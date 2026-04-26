//! End-to-end: N real `PouwNode`s, each with its own `GossipNetwork` on
//! loopback, all converge on the same finalized chain via real libp2p
//! gossipsub. This is the "decentralized PoUW chain actually works" test.

use std::sync::Arc;
use std::time::{Duration, Instant};

use ed25519_dalek::SigningKey;
use nous_pouw::engine::{Engine, EngineConfig};
use nous_pouw::net::{GossipNetwork, GossipNetworkConfig};
use nous_pouw::node::{NodeConfig, PouwNode};
use nous_pouw::sim::ConfigurableExecutor;
use nous_pouw::state::{ChainState, WorkerId};
use nous_pouw::tx::{Transaction, TxBody};
use rand::SeedableRng;
use rand_chacha::ChaCha20Rng;

#[tokio::test(flavor = "multi_thread", worker_threads = 8)]
async fn four_real_nodes_finalize_blocks_over_libp2p() {
    let _ = tracing_subscriber::fmt::try_init();

    let n = 4usize;
    let mut rng = ChaCha20Rng::seed_from_u64(7);
    let sks: Vec<SigningKey> = (0..n).map(|_| SigningKey::generate(&mut rng)).collect();

    // Build a shared genesis state: every node is a registered validator with equal stake.
    let mut genesis = ChainState::new();
    for sk in &sks {
        let id = WorkerId::from_verifying_key(&sk.verifying_key());
        genesis.register_worker(id, 1_000, 1.0);
        genesis.validators.insert(id);
    }

    // Spawn the bootstrap node first.
    let net_first = GossipNetwork::spawn(
        SigningKey::from_bytes(&sks[0].to_bytes()),
        GossipNetworkConfig {
            listen_addr: "/ip4/127.0.0.1/tcp/0".into(),
            bootstrap: vec![],
            mdns: false,
        },
    )
    .await
    .expect("spawn first GossipNetwork");
    tokio::time::sleep(Duration::from_millis(150)).await;
    let bootstrap_addr = net_first.local_addr();

    // Spawn the other networks dialing into the first.
    let mut networks: Vec<Arc<GossipNetwork>> = Vec::with_capacity(n);
    networks.push(Arc::new(net_first));
    for sk in &sks[1..] {
        let net = GossipNetwork::spawn(
            SigningKey::from_bytes(&sk.to_bytes()),
            GossipNetworkConfig {
                listen_addr: "/ip4/127.0.0.1/tcp/0".into(),
                bootstrap: vec![bootstrap_addr.clone()],
                mdns: false,
            },
        )
        .await
        .expect("spawn GossipNetwork");
        networks.push(Arc::new(net));
    }

    // Let the gossipsub mesh form before any node tries to propose.
    tokio::time::sleep(Duration::from_millis(2_500)).await;

    // Spawn the PouwNode driver tasks.
    let cfg = NodeConfig {
        tick_ms: 250,
        ..Default::default()
    };
    let nodes: Vec<PouwNode<GossipNetwork, ConfigurableExecutor>> = sks
        .iter()
        .zip(networks.iter())
        .map(|(sk, net)| {
            let engine = Engine::new(genesis.clone(), EngineConfig::default());
            let executor = ConfigurableExecutor::new(&sks);
            PouwNode::spawn(
                SigningKey::from_bytes(&sk.to_bytes()),
                engine,
                executor,
                net.clone(),
                None, // no persistence in this test
                cfg.clone(),
            )
        })
        .collect();

    // Wait for the chain to advance to height ≥ 3 on every node.
    let target_height: u64 = 3;
    let deadline = Instant::now() + Duration::from_secs(45);
    loop {
        let heights: Vec<u64> = nodes.iter().map(|n| n.state().height).collect();
        let min = heights.iter().min().copied().unwrap_or(0);
        if min >= target_height {
            break;
        }
        if Instant::now() > deadline {
            panic!(
                "timed out waiting for height ≥ {target_height} on all nodes; current heights = {heights:?}"
            );
        }
        tokio::time::sleep(Duration::from_millis(500)).await;
    }

    // All nodes must agree on the head hash.
    let states: Vec<ChainState> = nodes.iter().map(|n| n.state()).collect();
    let head = states[0].head_hash;
    for s in &states[1..] {
        assert_eq!(s.head_hash, head, "head_hash diverged across nodes");
    }

    for n in &nodes {
        n.shutdown();
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 8)]
async fn transfer_tx_propagates_and_finalizes() {
    let _ = tracing_subscriber::fmt::try_init();

    let mut rng = ChaCha20Rng::seed_from_u64(101);
    let sks: Vec<SigningKey> = (0..3).map(|_| SigningKey::generate(&mut rng)).collect();

    let mut genesis = ChainState::new();
    for sk in &sks {
        let id = WorkerId::from_verifying_key(&sk.verifying_key());
        genesis.register_worker(id, 1_000, 1.0);
        genesis.validators.insert(id);
    }
    // Give validator 0 some balance to spend.
    let donor_id = WorkerId::from_verifying_key(&sks[0].verifying_key());
    let recipient_id = WorkerId::from_verifying_key(&sks[1].verifying_key());
    genesis.workers.get_mut(&donor_id).unwrap().balance = 5_000;

    // Bootstrap network on validator 0.
    let net0 = GossipNetwork::spawn(
        SigningKey::from_bytes(&sks[0].to_bytes()),
        GossipNetworkConfig {
            listen_addr: "/ip4/127.0.0.1/tcp/0".into(),
            bootstrap: vec![],
            mdns: false,
        },
    )
    .await
    .unwrap();
    tokio::time::sleep(Duration::from_millis(150)).await;
    let boot = net0.local_addr();

    let mut networks = vec![Arc::new(net0)];
    for sk in &sks[1..] {
        let net = GossipNetwork::spawn(
            SigningKey::from_bytes(&sk.to_bytes()),
            GossipNetworkConfig {
                listen_addr: "/ip4/127.0.0.1/tcp/0".into(),
                bootstrap: vec![boot.clone()],
                mdns: false,
            },
        )
        .await
        .unwrap();
        networks.push(Arc::new(net));
    }
    tokio::time::sleep(Duration::from_millis(2_500)).await;

    let cfg = NodeConfig::default();
    let nodes: Vec<PouwNode<GossipNetwork, ConfigurableExecutor>> = sks
        .iter()
        .zip(networks.iter())
        .map(|(sk, net)| {
            let engine = Engine::new(genesis.clone(), EngineConfig::default());
            let executor = ConfigurableExecutor::new(&sks);
            PouwNode::spawn(
                SigningKey::from_bytes(&sk.to_bytes()),
                engine,
                executor,
                net.clone(),
                None,
                cfg.clone(),
            )
        })
        .collect();

    // Submit a transfer to *every* node's mempool so the tx is available
    // regardless of which validator wins the leader race for the next round.
    let tx = Transaction::new_signed(
        TxBody::Transfer {
            from: donor_id,
            to: recipient_id,
            amount: 250,
        },
        1,
        0,
        &SigningKey::from_bytes(&sks[0].to_bytes()),
    );
    for n in &nodes {
        let _ = n.submit_tx(tx.clone());
    }

    // Wait for tx to land on every node.
    let deadline = Instant::now() + Duration::from_secs(60);
    loop {
        let recv_balances: Vec<u64> = nodes
            .iter()
            .map(|n| n.state().workers[&recipient_id].balance)
            .collect();
        if recv_balances.iter().all(|b| *b == 250) {
            break;
        }
        if Instant::now() > deadline {
            panic!(
                "tx did not finalize across all nodes in time; balances = {recv_balances:?}"
            );
        }
        tokio::time::sleep(Duration::from_millis(500)).await;
    }

    // Donor balance must have decreased by 250 across all nodes.
    for n in &nodes {
        let s = n.state();
        assert_eq!(s.workers[&donor_id].balance, 4_750);
        assert_eq!(s.workers[&recipient_id].balance, 250);
    }

    for n in &nodes {
        n.shutdown();
    }
}
