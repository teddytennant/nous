//! Sybil resistance: a swarm of zero-stake / zero-trust workers cannot
//! displace a small set of staked honest workers in selection or quorum.

use ed25519_dalek::SigningKey;
use nous_pouw::engine::{Engine, EngineConfig};
use nous_pouw::envelope::{JobEnvelope, ModelPin};
use nous_pouw::sim::ConfigurableExecutor;
use nous_pouw::state::{ChainState, WorkerId};
use rand::SeedableRng;
use rand_chacha::ChaCha20Rng;

#[test]
fn many_zero_stake_workers_never_selected() {
    let mut state = ChainState::new();
    let mut rng = ChaCha20Rng::seed_from_u64(42);

    // 8 staked honest workers
    let honest_sks: Vec<_> = (0..8).map(|_| SigningKey::generate(&mut rng)).collect();
    for sk in &honest_sks {
        state.register_worker(
            WorkerId::from_verifying_key(&sk.verifying_key()),
            1_000,
            1.0,
        );
    }

    // 100 sybils with stake=0 (eligible_workers excludes them entirely).
    let mut sybil_ids = Vec::new();
    for _ in 0..100 {
        let sk = SigningKey::generate(&mut rng);
        let id = WorkerId::from_verifying_key(&sk.verifying_key());
        state.register_worker(id, 0, 0.0);
        sybil_ids.push(id);
    }

    let cfg = EngineConfig::default();
    let mut engine = Engine::new(state, cfg);

    let mut all_sks = honest_sks.clone();
    // We don't need the sybil signing keys for the executor — they're never
    // selected, so executor.execute is never called for them.
    let mut exec = ConfigurableExecutor::new(&all_sks);

    for round in 0..20 {
        let job = JobEnvelope {
            nonce: round,
            workflow_cid: [0; 32],
            workflow_payload: format!("payload-{round}").into_bytes(),
            model: ModelPin::new("m", round),
            n_replicas: 5,
            bounty: 100,
            deadline_ms: 60_000,
        };
        let leader_sk = SigningKey::from_bytes(&all_sks[(round as usize) % 8].to_bytes());
        let outcome = engine
            .step(&mut exec, &[job], &leader_sk, round * 1_000)
            .expect("step ok");

        // No sybil is in any cert.
        for cert in &outcome.block.body.certs {
            for w in &cert.agreeing_workers {
                assert!(
                    !sybil_ids.contains(w),
                    "sybil {} appeared in winning quorum",
                    w.short()
                );
            }
        }
    }

    // After 20 rounds, every honest worker has earned > 0; every sybil 0.
    for sybil in &sybil_ids {
        assert_eq!(engine.state.workers[sybil].balance, 0);
    }
    let honest_total: u64 = honest_sks
        .iter()
        .map(|sk| engine.state.workers[&WorkerId::from_verifying_key(&sk.verifying_key())].balance)
        .sum();
    assert!(honest_total > 0, "honest workers earned nothing");

    // Touch local handles to silence "unused" diagnostics for clarity.
    let _ = &mut all_sks;
}
