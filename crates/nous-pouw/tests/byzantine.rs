//! Up to ⌊(n-1)/3⌋ malicious workers — the chain still finalizes the
//! correct (honest-majority) output_hash, no honest worker is slashed, and
//! no incorrect mint is issued.

use nous_pouw::engine::EngineConfig;
use nous_pouw::sim::Harness;
use rand::SeedableRng;
use rand::rngs::StdRng;

#[test]
fn one_third_liars_chain_still_finalizes() {
    // 9 workers, 3 liars (33%); n_replicas=7, threshold=4/7 ≈ 571_429
    let cfg = EngineConfig {
        n_replicas_per_job: 7,
        quorum_threshold_micro: 571_429,
        ..Default::default()
    };
    let mut h = Harness::builder()
        .workers(9)
        .byzantine_fraction(0.33)
        .config(cfg)
        .seed(101)
        .build();

    let report = h.run(30, StdRng::seed_from_u64(102));
    assert_eq!(report.failed_jobs, 0);
    assert_eq!(report.certs, 30);

    // Slashes are dissent slashes for liars; expected to be > 0 because liars
    // get picked sometimes and dissent.
    assert!(report.slashes > 0, "no liars slashed for dissent");
}

#[test]
fn liars_earn_nothing() {
    let cfg = EngineConfig {
        n_replicas_per_job: 7,
        quorum_threshold_micro: 571_429,
        ..Default::default()
    };
    let mut h = Harness::builder()
        .workers(9)
        .byzantine_fraction(0.33)
        .config(cfg)
        .seed(201)
        .build();

    // The first 3 workers (in sks order) are the liars.
    let liar_ids: Vec<_> = h.sks[..3]
        .iter()
        .map(|sk| nous_pouw::WorkerId::from_verifying_key(&sk.verifying_key()))
        .collect();

    let report = h.run(50, StdRng::seed_from_u64(202));
    for liar in &liar_ids {
        let earned = report.mints_per_worker.get(liar).copied().unwrap_or(0);
        assert_eq!(earned, 0, "liar {} earned {}", liar.short(), earned);
    }
}

#[test]
fn honest_workers_never_slashed_below_one_third_byzantine() {
    let cfg = EngineConfig {
        n_replicas_per_job: 7,
        quorum_threshold_micro: 571_429,
        dissent_slash: 50,
        ..Default::default()
    };
    let mut h = Harness::builder()
        .workers(12)
        .byzantine_fraction(0.25) // 3/12 = 25%
        .initial_stake(1_000)
        .config(cfg)
        .seed(303)
        .build();

    // Honest workers: indices 3..12.
    let honest_ids: Vec<_> = h.sks[3..]
        .iter()
        .map(|sk| nous_pouw::WorkerId::from_verifying_key(&sk.verifying_key()))
        .collect();

    let _ = h.run(40, StdRng::seed_from_u64(304));

    for hid in &honest_ids {
        let info = &h.engine.state.workers[hid];
        assert!(!info.slashed, "honest worker {} got slashed", hid.short());
        assert_eq!(
            info.stake,
            1_000,
            "honest worker {} lost stake",
            hid.short()
        );
    }
}
