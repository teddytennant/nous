//! Harness: spin up N workers, run R rounds, collect a [`DevnetReport`].

use std::collections::HashMap;

use ed25519_dalek::SigningKey;
use rand::SeedableRng;
use rand::rngs::StdRng;
use rand_chacha::ChaCha20Rng;

use crate::engine::{Engine, EngineConfig, RoundOutcome, mints_from_block};
use crate::envelope::{JobEnvelope, ModelPin};
use crate::sim::byzantine::{ByzantineKind, ConfigurableExecutor};
use crate::state::{ChainState, WorkerId};

/// Builder for [`Harness`].
pub struct HarnessBuilder {
    n_workers: usize,
    initial_stake: u64,
    initial_trust: f64,
    byzantine_fraction: f64,
    seed: u64,
    config: EngineConfig,
}

impl Default for HarnessBuilder {
    fn default() -> Self {
        Self {
            n_workers: 8,
            initial_stake: 1_000,
            initial_trust: 1.0,
            byzantine_fraction: 0.0,
            seed: 0,
            config: EngineConfig::default(),
        }
    }
}

impl HarnessBuilder {
    pub fn workers(mut self, n: usize) -> Self {
        self.n_workers = n;
        self
    }
    pub fn initial_stake(mut self, s: u64) -> Self {
        self.initial_stake = s;
        self
    }
    pub fn initial_trust(mut self, t: f64) -> Self {
        self.initial_trust = t;
        self
    }
    pub fn byzantine_fraction(mut self, f: f64) -> Self {
        self.byzantine_fraction = f.clamp(0.0, 1.0);
        self
    }
    pub fn seed(mut self, s: u64) -> Self {
        self.seed = s;
        self
    }
    pub fn config(mut self, c: EngineConfig) -> Self {
        self.config = c;
        self
    }

    pub fn build(self) -> Harness {
        // Deterministic key generation via ChaCha20Rng seeded by `self.seed`.
        let mut rng = ChaCha20Rng::seed_from_u64(self.seed);
        let sks: Vec<SigningKey> = (0..self.n_workers)
            .map(|_| SigningKey::generate(&mut rng))
            .collect();

        let mut state = ChainState::new();
        for sk in &sks {
            state.register_worker(
                WorkerId::from_verifying_key(&sk.verifying_key()),
                self.initial_stake,
                self.initial_trust,
            );
        }

        let mut exec = ConfigurableExecutor::new(&sks);
        let n_byzantine = (self.n_workers as f64 * self.byzantine_fraction).round() as usize;
        // Pick byzantine workers deterministically from the *front* of `sks`.
        for sk in sks.iter().take(n_byzantine) {
            exec.set_behavior(
                WorkerId::from_verifying_key(&sk.verifying_key()),
                ByzantineKind::Liar,
            );
        }

        let engine = Engine::new(state, self.config);
        Harness {
            engine,
            sks,
            executor: exec,
            timestamp_ms: 0,
            byzantine_count: n_byzantine,
        }
    }
}

pub struct Harness {
    pub engine: Engine,
    pub sks: Vec<SigningKey>,
    pub executor: ConfigurableExecutor,
    pub timestamp_ms: u64,
    pub byzantine_count: usize,
}

impl Harness {
    pub fn builder() -> HarnessBuilder {
        HarnessBuilder::default()
    }

    /// Convenience: build a single job envelope with the given payload.
    pub fn job(&self, nonce: u64, payload: &[u8], bounty: u64) -> JobEnvelope {
        JobEnvelope {
            nonce,
            workflow_cid: *blake3::hash(payload).as_bytes(),
            workflow_payload: payload.to_vec(),
            model: ModelPin::new("sim-model", nonce),
            n_replicas: self.engine.config.n_replicas_per_job,
            bounty,
            deadline_ms: self.timestamp_ms + 60_000,
        }
    }

    /// Run one round. Leader is picked round-robin over `self.sks`.
    pub fn step(&mut self, jobs: &[JobEnvelope]) -> RoundOutcome {
        let leader_idx = (self.engine.epoch as usize) % self.sks.len();
        // Reconstruct the leader signing key (SigningKey doesn't impl Clone).
        let leader_sk = SigningKey::from_bytes(&self.sks[leader_idx].to_bytes());
        let outcome = self
            .engine
            .step(&mut self.executor, jobs, &leader_sk, self.timestamp_ms)
            .expect("step should succeed in v0 sim");
        self.timestamp_ms += 1_000;
        outcome
    }

    /// Run R rounds with one fresh job per round; return aggregate report.
    pub fn run(&mut self, rounds: usize, mut rng: StdRng) -> DevnetReport {
        use rand::RngCore;
        let mut report = DevnetReport {
            workers: self.sks.len(),
            byzantine_count: self.byzantine_count,
            ..Default::default()
        };
        for round in 0..rounds {
            // Random payload to make each job unique.
            let mut buf = [0u8; 16];
            rng.fill_bytes(&mut buf);
            let job = self.job(round as u64 + 1, &buf, 1_000);
            let outcome = self.step(&[job]);

            report.rounds += 1;
            report.certs += outcome.block.body.certs.len();
            report.failed_jobs += outcome.failed_jobs.len();
            report.slashes += outcome.block.body.slashes.len();

            for (worker, amount) in mints_from_block(&outcome.block) {
                *report.mints_per_worker.entry(worker).or_default() += amount;
                report.total_minted += amount;
            }
        }
        report.final_height = self.engine.state.height;
        report.total_supply = self.engine.state.total_supply;
        report.active_stake = self.engine.state.active_stake();
        report
    }
}

#[derive(Debug, Default)]
pub struct DevnetReport {
    pub workers: usize,
    pub byzantine_count: usize,
    pub rounds: usize,
    pub certs: usize,
    pub failed_jobs: usize,
    pub slashes: usize,
    pub final_height: u64,
    pub total_minted: u64,
    pub total_supply: u64,
    pub active_stake: u64,
    pub mints_per_worker: HashMap<WorkerId, u64>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn all_honest_finalizes_every_round() {
        let mut h = Harness::builder().workers(8).seed(1).build();
        let report = h.run(10, StdRng::seed_from_u64(2));
        assert_eq!(report.rounds, 10);
        assert_eq!(report.failed_jobs, 0);
        assert_eq!(report.certs, 10);
        assert_eq!(report.final_height, 10);
        assert!(report.total_minted > 0);
    }

    #[test]
    fn under_one_third_byzantine_still_finalizes() {
        let mut h = Harness::builder()
            .workers(9)
            .byzantine_fraction(0.22) // 2/9 < 1/3
            .seed(3)
            .build();
        let report = h.run(20, StdRng::seed_from_u64(4));
        assert_eq!(report.failed_jobs, 0);
        assert_eq!(report.certs, 20);
    }

    #[test]
    fn over_two_thirds_byzantine_fails_quorum() {
        let mut h = Harness::builder()
            .workers(9)
            .byzantine_fraction(0.78) // 7/9 > 2/3
            .seed(5)
            .build();
        let report = h.run(10, StdRng::seed_from_u64(6));
        // Many jobs will fail to reach quorum (chaotic group splits).
        assert!(report.failed_jobs > 0);
    }

    #[test]
    fn deterministic_under_same_seed() {
        let mut h1 = Harness::builder().workers(5).seed(42).build();
        let mut h2 = Harness::builder().workers(5).seed(42).build();
        let r1 = h1.run(5, StdRng::seed_from_u64(7));
        let r2 = h2.run(5, StdRng::seed_from_u64(7));
        assert_eq!(r1.total_minted, r2.total_minted);
        assert_eq!(r1.final_height, r2.final_height);
        assert_eq!(r1.certs, r2.certs);
    }

    #[test]
    fn total_supply_equals_total_minted_when_no_slash() {
        let mut h = Harness::builder().workers(5).seed(8).build();
        let report = h.run(5, StdRng::seed_from_u64(9));
        assert_eq!(report.total_supply, report.total_minted);
    }
}
