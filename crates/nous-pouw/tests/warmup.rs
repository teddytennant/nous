//! Tests that the warmup-time and min-peers gates actually prevent the
//! driver from proposing before they're satisfied.

use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::Duration;

use ed25519_dalek::SigningKey;
use nous_pouw::engine::{Engine, EngineConfig};
use nous_pouw::network::{Network, NetworkEvent, Topic};
use nous_pouw::node::{NodeConfig, PouwNode};
use nous_pouw::sim::ConfigurableExecutor;
use nous_pouw::state::{ChainState, WorkerId};
use rand::rngs::OsRng;

/// Counts how many block-publish events were sent on Topic::Blocks.
struct CountingNet {
    proposals: Arc<AtomicUsize>,
    /// Force a peer count for the gate test.
    peers: usize,
}
impl Network for CountingNet {
    fn publish(&self, topic: Topic, _from: WorkerId, _payload: Vec<u8>) {
        if topic == Topic::Blocks {
            self.proposals.fetch_add(1, Ordering::Relaxed);
        }
    }
    fn drain(&self) -> Vec<NetworkEvent> {
        Vec::new()
    }
    fn peer_count(&self) -> usize {
        self.peers
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn warmup_gate_blocks_proposals_until_elapsed() {
    let sk = SigningKey::generate(&mut OsRng);
    let mut state = ChainState::new();
    let id = WorkerId::from_verifying_key(&sk.verifying_key());
    state.register_worker(id, 1_000, 1.0);
    state.validators.insert(id);

    let proposals = Arc::new(AtomicUsize::new(0));
    let net = Arc::new(CountingNet {
        proposals: proposals.clone(),
        peers: usize::MAX,
    });

    let engine = Engine::new(state, EngineConfig::default());
    let executor = ConfigurableExecutor::new(&[SigningKey::from_bytes(&sk.to_bytes())]);
    let _node = PouwNode::spawn(
        SigningKey::from_bytes(&sk.to_bytes()),
        engine,
        executor,
        net.clone(),
        None,
        NodeConfig {
            tick_ms: 100,
            vote_timeout_ms: 5_000,
            finality_threshold_micro: 666_667,
            warmup_ms: 1_500,
            min_peers_to_propose: 0,
        },
    );

    // 500ms in: warmup not yet elapsed, no proposals.
    tokio::time::sleep(Duration::from_millis(500)).await;
    let early = proposals.load(Ordering::Relaxed);
    assert_eq!(early, 0, "got {early} proposals before warmup elapsed");

    // 2.5s in: warmup elapsed, proposals should now happen.
    tokio::time::sleep(Duration::from_millis(2_000)).await;
    let late = proposals.load(Ordering::Relaxed);
    assert!(late >= 1, "no proposals after warmup elapsed");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn min_peers_gate_blocks_proposals_until_satisfied() {
    let sk = SigningKey::generate(&mut OsRng);
    let mut state = ChainState::new();
    let id = WorkerId::from_verifying_key(&sk.verifying_key());
    state.register_worker(id, 1_000, 1.0);
    state.validators.insert(id);

    let proposals = Arc::new(AtomicUsize::new(0));
    let net = Arc::new(CountingNet {
        proposals: proposals.clone(),
        peers: 0, // no peers, gate triggered
    });

    let engine = Engine::new(state, EngineConfig::default());
    let executor = ConfigurableExecutor::new(&[SigningKey::from_bytes(&sk.to_bytes())]);
    let _node = PouwNode::spawn(
        SigningKey::from_bytes(&sk.to_bytes()),
        engine,
        executor,
        net.clone(),
        None,
        NodeConfig {
            tick_ms: 100,
            vote_timeout_ms: 5_000,
            finality_threshold_micro: 666_667,
            warmup_ms: 0,
            min_peers_to_propose: 2,
        },
    );

    tokio::time::sleep(Duration::from_millis(800)).await;
    let count = proposals.load(Ordering::Relaxed);
    assert_eq!(
        count, 0,
        "{count} proposals while peer_count=0 < min_peers_to_propose=2"
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn min_peers_gate_unblocks_when_threshold_met() {
    let sk = SigningKey::generate(&mut OsRng);
    let mut state = ChainState::new();
    let id = WorkerId::from_verifying_key(&sk.verifying_key());
    state.register_worker(id, 1_000, 1.0);
    state.validators.insert(id);

    let proposals = Arc::new(AtomicUsize::new(0));
    let net = Arc::new(CountingNet {
        proposals: proposals.clone(),
        peers: 5, // way above threshold
    });

    let engine = Engine::new(state, EngineConfig::default());
    let executor = ConfigurableExecutor::new(&[SigningKey::from_bytes(&sk.to_bytes())]);
    let _node = PouwNode::spawn(
        SigningKey::from_bytes(&sk.to_bytes()),
        engine,
        executor,
        net.clone(),
        None,
        NodeConfig {
            tick_ms: 100,
            vote_timeout_ms: 5_000,
            finality_threshold_micro: 666_667,
            warmup_ms: 0,
            min_peers_to_propose: 2,
        },
    );

    tokio::time::sleep(Duration::from_millis(800)).await;
    let count = proposals.load(Ordering::Relaxed);
    assert!(count >= 1, "no proposals despite peer_count >= threshold");
}
