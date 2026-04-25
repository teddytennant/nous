//! End-to-end: drive a PoUW round and ingest its mints into [`MintLedger`].

#![cfg(feature = "pouw")]

use nous_payments::mint::{MintLedger, WORK_TOKEN, ingest_block};
use nous_pouw::engine::mints_from_block;
use nous_pouw::sim::Harness;
use rand::SeedableRng;
use rand::rngs::StdRng;

#[test]
fn pouw_round_credits_mint_ledger() {
    let mut h = Harness::builder().workers(8).seed(99).build();
    let job = h.job(1, b"hello", 1_000);
    let outcome = h.step(&[job]);

    let mut ledger = MintLedger::new();
    ingest_block(&mut ledger, &outcome.block).unwrap();

    let total_in_block: u64 = mints_from_block(&outcome.block).values().sum();
    assert_eq!(ledger.total_supply(), total_in_block as u128);
    assert_eq!(ledger.total_supply(), 1_000); // bounty was 1000
    for wallet in ledger.wallets.values() {
        assert!(wallet.balance(WORK_TOKEN) > 0);
    }
}

#[test]
fn ingest_is_idempotent_across_blocks() {
    let mut h = Harness::builder().workers(8).seed(101).build();
    let job = h.job(1, b"x", 500);
    let outcome = h.step(&[job]);

    let mut ledger = MintLedger::new();
    ingest_block(&mut ledger, &outcome.block).unwrap();
    let after_first = ledger.total_supply();
    ingest_block(&mut ledger, &outcome.block).unwrap();
    assert_eq!(ledger.total_supply(), after_first);
}

#[test]
fn many_rounds_total_supply_matches_chain() {
    let mut h = Harness::builder().workers(8).seed(202).build();
    let mut ledger = MintLedger::new();

    let mut rng = StdRng::seed_from_u64(303);
    use rand::RngCore;
    for round in 0..15 {
        let mut buf = [0u8; 8];
        rng.fill_bytes(&mut buf);
        let job = h.job(round as u64 + 1, &buf, 1_000);
        let outcome = h.step(&[job]);
        ingest_block(&mut ledger, &outcome.block).unwrap();
    }

    assert_eq!(ledger.total_supply(), h.engine.state.total_supply as u128);
}
