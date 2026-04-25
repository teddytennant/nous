//! Property tests: invariants that must hold across random schedules.
//!
//! 1. Total supply only grows by sum of mint receipts in finalized blocks.
//! 2. No double-mint per (job_id, worker).
//! 3. Finality (state.height) is monotonically increasing.
//! 4. With ≤⌊(n-1)/3⌋ liars, every cert's winning output_hash matches the
//!    honest computation (blake3 of payload).

use nous_pouw::engine::EngineConfig;
use nous_pouw::sim::Harness;
use proptest::prelude::*;
use rand::SeedableRng;
use rand::rngs::StdRng;

proptest! {
    #![proptest_config(ProptestConfig {
        cases: 32,
        .. ProptestConfig::default()
    })]

    #[test]
    fn invariants_under_arbitrary_schedule(
        n_workers in 6usize..=12,
        rounds in 5usize..=20,
        seed in any::<u64>(),
        rng_seed in any::<u64>(),
    ) {
        let mut h = Harness::builder()
            .workers(n_workers)
            .seed(seed)
            .build();

        let mut prev_height = 0u64;
        let mut prev_supply = 0u64;
        let mut prev_minted = 0u64;
        let mut seen_pairs: std::collections::HashSet<(nous_pouw::WorkerId, nous_pouw::JobId)> = Default::default();

        let mut rng = StdRng::seed_from_u64(rng_seed);
        for round in 0..rounds {
            use rand::RngCore;
            let mut buf = [0u8; 16];
            rng.fill_bytes(&mut buf);
            let job = h.job(round as u64 + 1, &buf, 1_000);

            let outcome = h.step(std::slice::from_ref(&job));

            // Inv 3: height strictly increases by 1.
            prop_assert_eq!(h.engine.state.height, prev_height + 1);
            prev_height = h.engine.state.height;

            // Inv 4 (no liars): canonical output_hash matches blake3(payload).
            let canonical = *blake3::hash(&job.workflow_payload).as_bytes();
            for cert in &outcome.block.body.certs {
                prop_assert_eq!(cert.output_hash, canonical);
            }

            // Inv 1: supply growth equals mints in this block.
            let mints_this: u64 = nous_pouw::engine::mints_from_block(&outcome.block).values().sum();
            prop_assert_eq!(h.engine.state.total_supply - prev_supply, mints_this);
            prev_supply = h.engine.state.total_supply;
            prev_minted += mints_this;
            prop_assert_eq!(prev_minted, h.engine.state.total_supply);

            // Inv 2: each (worker, job_id) appears at most once across all blocks.
            for cert in &outcome.block.body.certs {
                for w in &cert.agreeing_workers {
                    let pair = (*w, cert.job_id);
                    prop_assert!(!seen_pairs.contains(&pair), "double mint for {:?}", pair);
                    seen_pairs.insert(pair);
                }
            }
        }
    }

    #[test]
    fn invariants_with_some_byzantine(
        n_workers in 9usize..=15,
        liar_pct in 0u32..=30,
        rounds in 5usize..=15,
        seed in any::<u64>(),
        rng_seed in any::<u64>(),
    ) {
        let cfg = EngineConfig {
            n_replicas_per_job: 7,
            // 4/7 ≈ 571_428 — strict majority of replicas.
            quorum_threshold_micro: 571_429,
            ..Default::default()
        };
        let mut h = Harness::builder()
            .workers(n_workers)
            .byzantine_fraction(liar_pct as f64 / 100.0)
            .config(cfg)
            .seed(seed)
            .build();

        let mut rng = StdRng::seed_from_u64(rng_seed);
        for round in 0..rounds {
            use rand::RngCore;
            let mut buf = [0u8; 16];
            rng.fill_bytes(&mut buf);
            let job = h.job(round as u64 + 1, &buf, 1_000);
            let outcome = h.step(std::slice::from_ref(&job));

            // The chain may fail to certify some jobs (when too many liars
            // happen to be selected by VRF); that's allowed. But any cert
            // that DOES form must agree on the canonical hash.
            let canonical = *blake3::hash(&job.workflow_payload).as_bytes();
            for cert in &outcome.block.body.certs {
                prop_assert_eq!(cert.output_hash, canonical);
            }

            // Height advances every round even when no cert formed.
            prop_assert_eq!(h.engine.state.height, round as u64 + 1);
        }
    }
}
