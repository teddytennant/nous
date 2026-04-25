//! End-to-end: 8 honest workers reach finality on a workflow and produce a mint.

use nous_pouw::sim::{DevnetReport, Harness};
use rand::SeedableRng;
use rand::rngs::StdRng;

#[test]
fn eight_honest_workers_reach_finality_and_mint() {
    let mut h = Harness::builder()
        .workers(8)
        .initial_stake(1_000)
        .seed(11)
        .build();

    let report: DevnetReport = h.run(20, StdRng::seed_from_u64(13));

    assert_eq!(report.workers, 8);
    assert_eq!(report.byzantine_count, 0);
    assert_eq!(report.rounds, 20);
    assert_eq!(report.failed_jobs, 0);
    assert_eq!(report.certs, 20);
    assert_eq!(report.slashes, 0);
    assert_eq!(report.final_height, 20);
    // Bounty 1000 per job * 20 jobs = 20_000 minted in total.
    assert_eq!(report.total_minted, 20_000);
    assert_eq!(report.total_supply, 20_000);
    // Stake unchanged (no slashes).
    assert_eq!(report.active_stake, 8_000);
}

#[test]
fn mints_distributed_across_multiple_workers() {
    let mut h = Harness::builder().workers(8).seed(17).build();
    let report = h.run(20, StdRng::seed_from_u64(19));
    assert!(
        report.mints_per_worker.len() >= 5,
        "fewer than 5 distinct workers earned: {}",
        report.mints_per_worker.len()
    );
}
