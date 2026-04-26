//! Multi-validator devnet: N independent [`Engine`]s exchange proposals + BFT
//! votes via an in-process bus, all converge on the same finalized chain.
//!
//! This is the v0 stand-in for a real libp2p deployment — every node owns its
//! own [`ChainState`], its own [`Mempool`], its own keypair, and its own
//! ed25519 [`Vote`] signing flow. The bus is a `Vec<Block>` / `Vec<Vote>`
//! the harness manages between rounds. Replacing the bus with libp2p
//! gossipsub (already wired up in [`crate::net`]) is a drop-in v1 swap.
//!
//! The harness exercises:
//! - Round-robin leader rotation by stake-ranked index.
//! - ⅔ stake-weighted vote certification per block.
//! - Deferred state application: validators only apply a block once its QC forms.
//! - Mempool integration: the leader pulls the next contiguous tx batch.
//! - parent_qc propagation: each block embeds the QC of the previous one.

use ed25519_dalek::SigningKey;
use rand::SeedableRng;
use rand_chacha::ChaCha20Rng;

use crate::bft::{Vote, VoteCertificate, form_quorum_cert};
use crate::block::Block;
use crate::engine::{Engine, EngineConfig};
use crate::envelope::JobEnvelope;
use crate::mempool::Mempool;
use crate::sim::byzantine::{ByzantineKind, ConfigurableExecutor};
use crate::state::{ChainState, WorkerId};
use crate::tx::Transaction;

/// One BFT validator + worker node.
pub struct ValidatorNode {
    pub sk: SigningKey,
    pub id: WorkerId,
    pub engine: Engine,
    pub mempool: Mempool,
    /// If true, this validator votes for whatever it's asked (honest).
    /// If false, it abstains from voting (simulates downtime / byzantine non-voter).
    pub votes: bool,
}

impl ValidatorNode {
    pub fn signing_key(&self) -> SigningKey {
        SigningKey::from_bytes(&self.sk.to_bytes())
    }
}

/// In-process multi-validator devnet. Drives one full BFT round per `step`.
pub struct MultiNodeDevnet {
    pub nodes: Vec<ValidatorNode>,
    pub executor: ConfigurableExecutor,
    pub config: EngineConfig,
    pub finality_threshold_micro: u32,
    /// QC of the most-recently-finalized block, embedded in the next block's header.
    pub last_qc: Option<VoteCertificate>,
    pub timestamp_ms: u64,
    /// Round-robin leader index — bumped each round.
    pub leader_idx: usize,
}

/// Result of one BFT round.
#[derive(Debug, Clone)]
pub struct BftRoundReport {
    pub block: Block,
    pub leader: WorkerId,
    pub votes_cast: usize,
    pub stake_micro: u32,
    pub finalized: bool,
}

impl MultiNodeDevnet {
    /// Spin up N validators with equal stake, registered + added to validator set.
    pub fn build(n_validators: usize, stake: u64, seed: u64, config: EngineConfig) -> Self {
        let mut rng = ChaCha20Rng::seed_from_u64(seed);
        let sks: Vec<SigningKey> = (0..n_validators).map(|_| SigningKey::generate(&mut rng)).collect();

        let mut genesis = ChainState::new();
        for sk in &sks {
            let id = WorkerId::from_verifying_key(&sk.verifying_key());
            genesis.register_worker(id, stake, 1.0);
            genesis.validators.insert(id);
        }

        let nodes: Vec<ValidatorNode> = sks
            .iter()
            .map(|sk| ValidatorNode {
                sk: SigningKey::from_bytes(&sk.to_bytes()),
                id: WorkerId::from_verifying_key(&sk.verifying_key()),
                engine: Engine::new(genesis.clone(), config.clone()),
                mempool: Mempool::new(),
                votes: true,
            })
            .collect();

        let executor = ConfigurableExecutor::new(&sks);

        Self {
            nodes,
            executor,
            config,
            finality_threshold_micro: 666_667,
            last_qc: None,
            timestamp_ms: 0,
            leader_idx: 0,
        }
    }

    /// Mark `n` validators (in iteration order) as byzantine non-voters and
    /// liar workers. Returns the affected ids.
    pub fn make_first_n_byzantine(&mut self, n: usize) -> Vec<WorkerId> {
        let mut byz = Vec::new();
        for node in self.nodes.iter_mut().take(n) {
            node.votes = false;
            self.executor.set_behavior(node.id, ByzantineKind::Liar);
            byz.push(node.id);
        }
        byz
    }

    /// Number of nodes currently participating as honest validators.
    pub fn honest_count(&self) -> usize {
        self.nodes.iter().filter(|n| n.votes).count()
    }

    /// Submit a tx to every validator's mempool (broadcast simulation).
    pub fn broadcast_tx(&mut self, tx: Transaction) -> Result<(), crate::tx::TxError> {
        for node in &mut self.nodes {
            // Snapshot the local view of state at insert time.
            let state_snapshot = node.engine.state.clone();
            node.mempool.insert(&state_snapshot, tx.clone())?;
        }
        Ok(())
    }

    /// Advance one BFT round: leader proposes, validators vote, QC forms,
    /// every node applies the block.
    pub fn step(&mut self, jobs: &[JobEnvelope]) -> BftRoundReport {
        let leader_idx = self.leader_idx % self.nodes.len();
        self.leader_idx = self.leader_idx.wrapping_add(1);

        // 1. Leader builds (but does NOT yet apply) the proposed block.
        let leader_sk = self.nodes[leader_idx].signing_key();
        let leader_id = self.nodes[leader_idx].id;
        // Snapshot the leader's state to avoid the double-borrow when calling
        // mempool.take(&state, ...).
        let leader_state_snapshot = self.nodes[leader_idx].engine.state.clone();
        let txs: Vec<Transaction> = self.nodes[leader_idx]
            .mempool
            .take(&leader_state_snapshot, crate::DEFAULT_MAX_TX_PER_BLOCK);
        let timestamp_ms = self.timestamp_ms;
        self.timestamp_ms += 1_000;

        let outcome = self.nodes[leader_idx]
            .engine
            .step_full(
                &mut self.executor,
                jobs,
                &txs,
                &leader_sk,
                timestamp_ms,
                self.last_qc.clone(),
                false, // do NOT auto-apply; wait for QC
            )
            .expect("leader step must succeed in v0 sim");
        let block = outcome.block;
        let block_hash = block.hash();

        // 2. Each honest validator verifies + votes.
        let mut votes: Vec<Vote> = Vec::new();
        for node in &self.nodes {
            if !node.votes {
                continue;
            }
            if crate::block::verify_block(&block).is_err() {
                continue;
            }
            let vote = Vote::new_signed(block.header.height, block_hash, &node.signing_key());
            votes.push(vote);
        }
        let votes_cast = votes.len();

        // 3. Form QC from the collected votes.
        let qc = form_quorum_cert(
            block.header.height,
            block_hash,
            votes,
            // Voters check against the *current* canonical state (any node has
            // an identical state since they all saw the same finalized prefix).
            &self.nodes[0].engine.state,
            self.finality_threshold_micro,
        );

        let (finalized, stake_micro) = match qc {
            Ok(cert) => {
                let stake_micro = cert.stake_micro;
                self.last_qc = Some(cert);
                // 4. Apply the block to every validator's state.
                for node in &mut self.nodes {
                    node.engine
                        .apply_external_block(&block)
                        .expect("external block must apply if QC valid");
                    node.mempool.remove_included(&block.body.transactions);
                    node.mempool.prune(&node.engine.state);
                }
                (true, stake_micro)
            }
            Err(_) => (false, 0),
        };

        BftRoundReport {
            block,
            leader: leader_id,
            votes_cast,
            stake_micro,
            finalized,
        }
    }

    /// Convenience: run R rounds with one randomly-payloaded job each.
    pub fn run(&mut self, rounds: usize, seed: u64) -> MultiNodeReport {
        let mut report = MultiNodeReport::default();
        report.validators = self.nodes.len();
        let mut rng = ChaCha20Rng::seed_from_u64(seed);
        use rand::RngCore;
        for round in 0..rounds {
            let mut buf = [0u8; 16];
            rng.fill_bytes(&mut buf);
            let job = JobEnvelope {
                nonce: round as u64 + 1,
                workflow_cid: *blake3::hash(&buf).as_bytes(),
                workflow_payload: buf.to_vec(),
                model: crate::envelope::ModelPin::new("multinode-sim", round as u64),
                n_replicas: self.config.n_replicas_per_job,
                bounty: 1_000,
                deadline_ms: self.timestamp_ms + 60_000,
            };
            let r = self.step(&[job]);
            report.rounds += 1;
            if r.finalized {
                report.finalized += 1;
            }
            report.blocks_produced += 1;
            report.total_votes += r.votes_cast;
        }
        report.final_height = self.nodes[0].engine.state.height;
        report.divergent_states = self.divergent_count();
        report
    }

    /// Count nodes whose state.head_hash differs from node[0]'s — should be 0.
    pub fn divergent_count(&self) -> usize {
        let head = self.nodes[0].engine.state.head_hash;
        self.nodes
            .iter()
            .filter(|n| n.engine.state.head_hash != head)
            .count()
    }
}

#[derive(Debug, Default, Clone)]
pub struct MultiNodeReport {
    pub validators: usize,
    pub rounds: usize,
    pub finalized: usize,
    pub blocks_produced: usize,
    pub total_votes: usize,
    pub final_height: u64,
    /// Should always be 0 if BFT works.
    pub divergent_states: usize,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn four_validators_finalize_every_round() {
        let cfg = EngineConfig {
            n_replicas_per_job: 3,
            quorum_threshold_micro: 666_667,
            ..Default::default()
        };
        let mut net = MultiNodeDevnet::build(4, 100, 1, cfg);
        let report = net.run(10, 99);
        assert_eq!(report.rounds, 10);
        assert_eq!(report.finalized, 10);
        assert_eq!(report.divergent_states, 0);
        assert_eq!(report.final_height, 10);
    }

    #[test]
    fn one_byzantine_of_four_still_finalizes() {
        let cfg = EngineConfig::default();
        let mut net = MultiNodeDevnet::build(4, 100, 2, cfg);
        net.make_first_n_byzantine(1); // 1/4 = 25% < 1/3
        let report = net.run(10, 100);
        assert_eq!(report.finalized, 10, "10 blocks must finalize with 25% byzantine");
        assert_eq!(report.divergent_states, 0);
    }

    #[test]
    fn two_byzantine_of_four_fails_quorum() {
        let cfg = EngineConfig::default();
        let mut net = MultiNodeDevnet::build(4, 100, 3, cfg);
        net.make_first_n_byzantine(2); // 2/4 = 50% — only 50% honest, can't reach ⅔
        let report = net.run(5, 101);
        assert_eq!(report.finalized, 0, "no rounds should finalize");
    }

    #[test]
    fn states_converge_across_validators() {
        let mut net = MultiNodeDevnet::build(5, 100, 4, EngineConfig::default());
        let report = net.run(20, 102);
        assert_eq!(report.divergent_states, 0);
        // All nodes have the same head hash.
        let head = net.nodes[0].engine.state.head_hash;
        for n in &net.nodes {
            assert_eq!(n.engine.state.head_hash, head);
            assert_eq!(n.engine.state.height, 20);
        }
    }

    #[test]
    fn parent_qc_chains_correctly() {
        let mut net = MultiNodeDevnet::build(4, 100, 5, EngineConfig::default());
        let r1 = net.step(&[]);
        assert!(r1.finalized);
        let block_hash_1 = r1.block.hash();
        let r2 = net.step(&[]);
        assert!(r2.finalized);
        // Block 2's parent_qc must justify block 1.
        let qc = r2
            .block
            .header
            .parent_qc
            .as_ref()
            .expect("block 2 must carry parent_qc for block 1");
        assert_eq!(qc.height, 1);
        assert_eq!(qc.block_hash, block_hash_1);
    }

    #[test]
    fn transactions_flow_through_mempool_to_block() {
        use crate::tx::{Transaction, TxBody};
        let mut net = MultiNodeDevnet::build(4, 100, 6, EngineConfig::default());
        // Bootstrap: give validator 0 some balance to spend (manual mutation).
        let donor_id = net.nodes[0].id;
        let recipient_id = net.nodes[1].id;
        for node in &mut net.nodes {
            node.engine.state.workers.get_mut(&donor_id).unwrap().balance = 1_000;
        }

        let donor_sk = net.nodes[0].signing_key();
        let tx = Transaction::new_signed(
            TxBody::Transfer {
                from: donor_id,
                to: recipient_id,
                amount: 100,
            },
            1,
            0,
            &donor_sk,
        );
        net.broadcast_tx(tx).unwrap();

        // The next round should embed the tx in the block.
        let r = net.step(&[]);
        assert!(r.finalized);
        assert_eq!(r.block.body.transactions.len(), 1);

        // Recipient balance updated on every validator.
        for n in &net.nodes {
            assert_eq!(n.engine.state.workers[&recipient_id].balance, 100);
            assert_eq!(n.engine.state.workers[&donor_id].balance, 900);
        }
    }

    #[test]
    fn leader_rotates_round_robin() {
        let mut net = MultiNodeDevnet::build(4, 100, 7, EngineConfig::default());
        let leaders: Vec<_> = (0..8).map(|_| net.step(&[]).leader).collect();
        // Each leader id appears exactly twice across 8 rounds.
        for node in &net.nodes {
            let count = leaders.iter().filter(|l| **l == node.id).count();
            assert_eq!(count, 2);
        }
    }

    #[test]
    fn divergent_states_remains_zero_under_byzantine() {
        let mut net = MultiNodeDevnet::build(7, 100, 8, EngineConfig::default());
        net.make_first_n_byzantine(2); // 2/7 ≈ 28% < 1/3
        let report = net.run(15, 9);
        assert_eq!(report.divergent_states, 0);
        assert_eq!(report.finalized, 15);
    }
}
