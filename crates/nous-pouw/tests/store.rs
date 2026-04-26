//! Integration tests for the SQLite-backed [`Store`].
//!
//! These tests drive realistic blocks through the consensus [`Engine`] using
//! the existing in-process [`sim::Harness`], then assert that round-tripping
//! through the [`Store`] preserves the chain state and block history.

use nous_pouw::sim::Harness;
use nous_pouw::store::Store;
use rand::SeedableRng;
use rand::rngs::StdRng;

/// Build a harness that finalizes a block on every step, then drive `rounds`
/// rounds, persisting both the block and the resulting state into `store`
/// after each step.
fn drive(harness: &mut Harness, store: &mut Store, rounds: usize, rng_seed: u64) {
    use rand::RngCore;
    let mut rng = StdRng::seed_from_u64(rng_seed);
    for round in 0..rounds {
        let mut buf = [0u8; 16];
        rng.fill_bytes(&mut buf);
        let job = harness.job(round as u64 + 1, &buf, 1_000);
        let outcome = harness.step(&[job]);
        store
            .save_block(&outcome.block, &harness.engine.state)
            .expect("save_block ok");
    }
}

#[test]
fn save_and_load_round_trip() {
    let mut store = Store::open_in_memory().expect("open store");
    let mut h = Harness::builder().workers(8).seed(11).build();

    drive(&mut h, &mut store, 5, 22);

    let loaded = store.load_state().expect("load_state ok");
    let live = &h.engine.state;

    assert_eq!(loaded.height, live.height, "height matches");
    assert_eq!(loaded.head_hash, live.head_hash, "head_hash matches");
    assert_eq!(
        loaded.total_supply, live.total_supply,
        "total_supply matches"
    );
    assert_eq!(loaded.workers.len(), live.workers.len(), "worker count");

    for (id, info) in &live.workers {
        let got = loaded.workers.get(id).expect("worker present");
        assert_eq!(got.stake, info.stake, "stake matches for {}", id.short());
        assert_eq!(got.balance, info.balance, "balance matches");
        assert_eq!(got.trust.to_bits(), info.trust.to_bits(), "trust matches");
        assert_eq!(got.slashed, info.slashed, "slashed matches");
    }

    assert_eq!(loaded.used_jobs, live.used_jobs, "used_jobs matches");
    assert_eq!(
        loaded.used_worker_jobs, live.used_worker_jobs,
        "used_worker_jobs matches"
    );
}

#[test]
fn iter_blocks_returns_all_in_order() {
    let mut store = Store::open_in_memory().expect("open store");
    let mut h = Harness::builder().workers(6).seed(13).build();

    drive(&mut h, &mut store, 5, 23);

    let blocks = store.iter_blocks().expect("iter_blocks ok");
    assert_eq!(blocks.len(), 5, "5 blocks");
    for (i, b) in blocks.iter().enumerate() {
        let expected_height = (i as u64) + 1;
        assert_eq!(
            b.header.height, expected_height,
            "block {i} has height {expected_height}"
        );
        // Each block's hash must match its header digest.
        let lookup = store.block_by_hash(&b.hash()).expect("by_hash ok");
        assert!(lookup.is_some(), "block {i} found by hash");
    }
}

#[test]
fn block_at_and_block_by_hash() {
    let mut store = Store::open_in_memory().expect("open store");
    let mut h = Harness::builder().workers(5).seed(14).build();

    drive(&mut h, &mut store, 3, 24);

    for height in 1..=3u64 {
        let by_height = store
            .block_at(height)
            .expect("block_at ok")
            .expect("block at height present");
        assert_eq!(by_height.header.height, height);

        let hash = by_height.hash();
        let by_hash = store
            .block_by_hash(&hash)
            .expect("block_by_hash ok")
            .expect("block by hash present");
        assert_eq!(by_hash.header.height, height);
        assert_eq!(by_hash.hash(), hash);
    }

    assert!(
        store.block_at(99).expect("ok").is_none(),
        "missing height -> None"
    );
    assert!(
        store.block_by_hash(&[0u8; 32]).expect("ok").is_none(),
        "missing hash -> None"
    );
}

#[test]
fn crash_recovery() {
    let dir = tempfile::tempdir().expect("tempdir");
    let path = dir.path().join("chain.db");

    let live_state_snapshot = {
        let mut store = Store::open(&path).expect("open store on disk");
        let mut h = Harness::builder().workers(7).seed(15).build();
        drive(&mut h, &mut store, 3, 25);
        h.engine.state.clone()
    };

    // Drop above scope -> Connection closes; reopen and assert state matches.
    let store = Store::open(&path).expect("reopen store");
    let loaded = store.load_state().expect("load_state ok");

    assert_eq!(loaded.height, live_state_snapshot.height);
    assert_eq!(loaded.head_hash, live_state_snapshot.head_hash);
    assert_eq!(loaded.total_supply, live_state_snapshot.total_supply);
    assert_eq!(loaded.workers.len(), live_state_snapshot.workers.len());
    assert_eq!(loaded.used_jobs, live_state_snapshot.used_jobs);
    assert_eq!(
        loaded.used_worker_jobs,
        live_state_snapshot.used_worker_jobs
    );
    assert_eq!(store.head_height().expect("head_height"), 3);
    assert_eq!(store.iter_blocks().expect("iter_blocks").len(), 3);
}

#[test]
fn head_height_zero_when_empty() {
    let store = Store::open_in_memory().expect("open store");
    assert_eq!(store.head_height().expect("head_height"), 0);
    let state = store.load_state().expect("load_state");
    assert_eq!(state.height, 0);
    assert_eq!(state.head_hash, [0u8; 32]);
    assert_eq!(state.total_supply, 0);
    assert!(state.workers.is_empty());
    assert!(store.iter_blocks().expect("iter").is_empty());
    assert!(store.block_at(1).expect("ok").is_none());
}

#[test]
fn idempotent_resave() {
    let mut store = Store::open_in_memory().expect("open store");
    let mut h = Harness::builder().workers(5).seed(16).build();

    drive(&mut h, &mut store, 1, 26);

    let first = store
        .block_at(1)
        .expect("block_at ok")
        .expect("present after first save");

    // Second save with the same block (and same state) should be a no-op
    // and not error.
    let snapshot = h.engine.state.clone();
    store.save_block(&first, &snapshot).expect("resave ok");
    store
        .save_block(&first, &snapshot)
        .expect("resave again ok");

    let blocks = store.iter_blocks().expect("iter_blocks ok");
    assert_eq!(blocks.len(), 1, "still only one block after resaves");
    assert_eq!(blocks[0].hash(), first.hash(), "same hash preserved");

    let loaded = store.load_state().expect("load_state ok");
    assert_eq!(loaded.height, snapshot.height);
    assert_eq!(loaded.total_supply, snapshot.total_supply);
}
