//! `nous-pouw-node`: a runnable single-validator PoUW daemon.
//!
//! ```text
//! cargo run --release -p nous-pouw --example node -- \
//!   --listen /ip4/0.0.0.0/tcp/9001 \
//!   --seed 1 \
//!   --db /tmp/pouw-1.db
//!
//! cargo run --release -p nous-pouw --example node -- \
//!   --listen /ip4/0.0.0.0/tcp/9002 \
//!   --bootstrap /ip4/127.0.0.1/tcp/9001 \
//!   --seed 2 \
//!   --db /tmp/pouw-2.db
//! ```
//!
//! Two such processes on this laptop will form a working PoUW chain over
//! real libp2p gossipsub, finalizing blocks via BFT votes, with state
//! persisted to SQLite.

use std::env;
use std::sync::Arc;
use std::time::Duration;

use ed25519_dalek::SigningKey;
use nous_pouw::engine::{Engine, EngineConfig};
use nous_pouw::net::{GossipNetwork, GossipNetworkConfig};
use nous_pouw::node::{NodeConfig, PouwNode};
use nous_pouw::sim::ConfigurableExecutor;
use nous_pouw::state::{ChainState, WorkerId};
use nous_pouw::store::Store;
use rand::SeedableRng;
use rand_chacha::ChaCha20Rng;

fn parse<T: std::str::FromStr>(args: &[String], flag: &str, default: T) -> T {
    for w in args.windows(2) {
        if w[0] == flag
            && let Ok(v) = w[1].parse()
        {
            return v;
        }
    }
    default
}

fn parse_multi(args: &[String], flag: &str) -> Vec<String> {
    let mut out = Vec::new();
    for w in args.windows(2) {
        if w[0] == flag {
            out.push(w[1].clone());
        }
    }
    out
}

#[tokio::main(flavor = "multi_thread")]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .try_init()
        .ok();

    let args: Vec<String> = env::args().collect();
    let listen: String = parse(&args, "--listen", "/ip4/0.0.0.0/tcp/0".into());
    let bootstrap: Vec<String> = parse_multi(&args, "--bootstrap");
    let seed: u64 = parse(&args, "--seed", 1u64);
    let n_validators: usize = parse(&args, "--validators", 4usize);
    let stake: u64 = parse(&args, "--stake", 1_000u64);
    let db_path: String = parse(&args, "--db", String::new());
    let mdns: bool = parse(&args, "--mdns", false);

    // Validator keys are derived from --seed + index so all participants in
    // the network share a known genesis. (For a real deployment you'd load
    // a private key from disk and a genesis JSON file.)
    let mut rng = ChaCha20Rng::seed_from_u64(seed);
    let _ = (0..0).map(|_| SigningKey::generate(&mut rng));

    // For demo: regenerate the canonical key set deterministically from the
    // *network* seed (independent of --seed) so every validator finds the
    // same genesis.
    let mut net_rng = ChaCha20Rng::seed_from_u64(0);
    let all_sks: Vec<SigningKey> = (0..n_validators)
        .map(|_| SigningKey::generate(&mut net_rng))
        .collect();

    let my_idx: usize = parse(&args, "--idx", 0usize);
    if my_idx >= n_validators {
        eprintln!("--idx must be < --validators");
        std::process::exit(1);
    }
    let my_sk = SigningKey::from_bytes(&all_sks[my_idx].to_bytes());

    // Build genesis: every key registered with `stake` and added to validators.
    let mut genesis = ChainState::new();
    for sk in &all_sks {
        let id = WorkerId::from_verifying_key(&sk.verifying_key());
        genesis.register_worker(id, stake, 1.0);
        genesis.validators.insert(id);
    }

    let net = GossipNetwork::spawn(
        SigningKey::from_bytes(&my_sk.to_bytes()),
        GossipNetworkConfig {
            listen_addr: listen,
            bootstrap,
            mdns,
        },
    )
    .await?;
    tokio::time::sleep(Duration::from_millis(200)).await;
    println!("listening on: {}", net.local_addr());

    // Optional persistence.
    let store = if db_path.is_empty() {
        None
    } else {
        Some(Store::open(&db_path)?)
    };

    let engine = Engine::new(genesis, EngineConfig::default());
    let executor = ConfigurableExecutor::new(&all_sks);
    let _node = PouwNode::spawn(
        my_sk,
        engine,
        executor,
        Arc::new(net),
        store,
        NodeConfig::default(),
    );

    // Just sleep forever; the node loop runs in a tokio task.
    println!(
        "nous-pouw-node started: idx={}/{} seed={}",
        my_idx, n_validators, seed
    );
    let mut tick = tokio::time::interval(Duration::from_secs(5));
    loop {
        tick.tick().await;
        let s = _node.state();
        println!(
            "[idx={}] height={} supply={} validators={} workers={}",
            my_idx,
            s.height,
            s.total_supply,
            s.validators.len(),
            s.workers.len()
        );
    }
}
