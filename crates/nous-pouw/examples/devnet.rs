//! Devnet demo: spin up N workers, run R rounds, print a report.
//!
//! ```text
//! cargo run --release -p nous-pouw --example devnet -- \
//!   --workers 32 --byzantine-fraction 0.25 --rounds 50 --seed 42
//! ```

use nous_pouw::engine::EngineConfig;
use nous_pouw::sim::Harness;
use rand::SeedableRng;
use rand::rngs::StdRng;
use std::env;
use std::time::Instant;

fn parse_arg<T: std::str::FromStr>(args: &[String], flag: &str, default: T) -> T {
    for w in args.windows(2) {
        if w[0] == flag
            && let Ok(v) = w[1].parse()
        {
            return v;
        }
    }
    default
}

fn main() {
    let args: Vec<String> = env::args().collect();
    let workers: usize = parse_arg(&args, "--workers", 32);
    let byz: f64 = parse_arg(&args, "--byzantine-fraction", 0.0);
    let rounds: usize = parse_arg(&args, "--rounds", 50);
    let seed: u64 = parse_arg(&args, "--seed", 0);
    let n_replicas: u8 = parse_arg(&args, "--replicas", 7);

    let cfg = EngineConfig {
        n_replicas_per_job: n_replicas,
        // 4/7 ≈ majority of replicas
        quorum_threshold_micro: 571_429,
        ..Default::default()
    };

    println!(
        "devnet: workers={workers} byzantine_fraction={byz:.2} rounds={rounds} \
         seed={seed} n_replicas={n_replicas}"
    );

    let mut h = Harness::builder()
        .workers(workers)
        .byzantine_fraction(byz)
        .config(cfg)
        .seed(seed)
        .build();

    let start = Instant::now();
    let report = h.run(rounds, StdRng::seed_from_u64(seed.wrapping_add(1)));
    let elapsed = start.elapsed();

    println!("\n=== devnet report ===");
    println!("workers:           {}", report.workers);
    println!("byzantine workers: {}", report.byzantine_count);
    println!("rounds run:        {}", report.rounds);
    println!("blocks finalized:  {}", report.final_height);
    println!("certs in blocks:   {}", report.certs);
    println!("failed jobs:       {}", report.failed_jobs);
    println!("slash events:      {}", report.slashes);
    println!("total minted:      {}", report.total_minted);
    println!("total supply:      {}", report.total_supply);
    println!("active stake:      {}", report.active_stake);
    println!("distinct earners:  {}", report.mints_per_worker.len());
    println!(
        "rounds/sec:        {:.2}",
        rounds as f64 / elapsed.as_secs_f64()
    );
    println!("elapsed:           {:.3}s", elapsed.as_secs_f64());
}
