//! Persistence recovery: a node that crashes mid-chain can be restarted
//! from disk and resume with identical state.

use std::sync::Arc;
use std::time::Duration;

use ed25519_dalek::SigningKey;
use nous_pouw::Network;
use nous_pouw::engine::{Engine, EngineConfig};
use nous_pouw::node::{NodeConfig, PouwNode};
use nous_pouw::sim::ConfigurableExecutor;
use nous_pouw::state::{ChainState, WorkerId};
use nous_pouw::store::Store;
use nous_pouw::tx::{Transaction, TxBody};
use rand::rngs::OsRng;

struct NoopNet;
impl Network for NoopNet {
    fn publish(&self, _topic: nous_pouw::Topic, _from: WorkerId, _payload: Vec<u8>) {}
    fn drain(&self) -> Vec<nous_pouw::NetworkEvent> {
        Vec::new()
    }
}

fn build_genesis(sk: &SigningKey, balance: u64) -> ChainState {
    let mut state = ChainState::new();
    let id = WorkerId::from_verifying_key(&sk.verifying_key());
    state.register_worker(id, 1_000, 1.0);
    state.validators.insert(id);
    state.workers.get_mut(&id).unwrap().balance = balance;
    state
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn store_persists_across_node_restart() {
    let dir = tempfile::tempdir().unwrap();
    let db_path = dir.path().join("pouw.db");

    let sk = SigningKey::generate(&mut OsRng);
    let id = WorkerId::from_verifying_key(&sk.verifying_key());
    let recipient_sk = SigningKey::generate(&mut OsRng);
    let recipient_id = WorkerId::from_verifying_key(&recipient_sk.verifying_key());

    // First lifetime: bring up a node, push a tx, let it persist a block.
    {
        let mut genesis = build_genesis(&sk, 1_000);
        genesis.register_worker(recipient_id, 0, 1.0);
        let store = Store::open(&db_path).expect("open store");
        let engine = Engine::new(genesis, EngineConfig::default());
        let executor = ConfigurableExecutor::new(&[SigningKey::from_bytes(&sk.to_bytes())]);
        let node = PouwNode::spawn(
            SigningKey::from_bytes(&sk.to_bytes()),
            engine,
            executor,
            Arc::new(NoopNet),
            Some(store),
            NodeConfig {
                tick_ms: 100,
                vote_timeout_ms: 5_000,
                finality_threshold_micro: 666_667,
                warmup_ms: 0,
                min_peers_to_propose: 0,
            },
        );

        let tx = Transaction::new_signed(
            TxBody::Transfer {
                from: id,
                to: recipient_id,
                amount: 250,
            },
            1,
            0,
            &SigningKey::from_bytes(&sk.to_bytes()),
        );
        node.submit_tx(tx).expect("submit tx");

        // Let the chain advance a few rounds so a block carrying the tx is
        // produced + persisted to the store.
        let deadline = std::time::Instant::now() + Duration::from_secs(20);
        loop {
            let s = node.state();
            if s.workers[&recipient_id].balance == 250 {
                break;
            }
            if std::time::Instant::now() > deadline {
                panic!("tx never finalized; recipient still 0");
            }
            tokio::time::sleep(Duration::from_millis(200)).await;
        }
        node.shutdown();
        // Drop node (the store is owned + flushed on drop).
        drop(node);
        tokio::time::sleep(Duration::from_millis(200)).await;
    }

    // Second lifetime: open the store, replay state, assert balances match.
    let store = Store::open(&db_path).expect("reopen store");
    let recovered: ChainState = store.load_state().expect("load state");
    assert!(recovered.height >= 1);
    assert_eq!(recovered.workers[&recipient_id].balance, 250);
    assert_eq!(recovered.workers[&id].balance, 750);
    assert_eq!(recovered.workers[&id].nonce, 1);

    // The block at the height where the tx finalized must replay cleanly.
    let blocks = store.iter_blocks().expect("iter blocks");
    assert!(!blocks.is_empty());
    let mut chain = ChainState::new();
    chain.register_worker(id, 1_000, 1.0);
    chain.validators.insert(id);
    chain.register_worker(recipient_id, 0, 1.0);
    chain.workers.get_mut(&id).unwrap().balance = 1_000;
    for b in &blocks {
        chain.apply_block(b).expect("re-apply persisted block");
    }
    assert_eq!(chain.workers[&recipient_id].balance, 250);
    assert_eq!(chain.workers[&id].balance, 750);
}
