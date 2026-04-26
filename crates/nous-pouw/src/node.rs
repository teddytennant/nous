//! Async runnable node: ties [`Engine`] + [`Network`] + [`Store`] + [`Mempool`] together.
//!
//! Each [`PouwNode`] is one validator. Spawning N PouwNodes (in N processes
//! on one machine, or across the internet) yields a full PoUW chain that
//! finalizes blocks via real ed25519 BFT votes gossiped over libp2p.
//!
//! The driver loop (private) does, every `tick_ms` milliseconds:
//!   1. Drain inbound network events into typed handlers (block / vote / tx).
//!   2. If the local node is the elected leader for the current epoch, build
//!      a block from local jobs + mempool + last_qc, broadcast it.
//!   3. Verify any received block, sign a [`Vote`], broadcast it.
//!   4. Once ⅔ stake-weighted votes accumulate for some block, form a
//!      [`VoteCertificate`], apply the block locally + persist + prune mempool.

use std::sync::Arc;
use std::time::Duration;

use ed25519_dalek::SigningKey;
use parking_lot::Mutex;
use tokio::task::JoinHandle;
use tracing::{debug, info, warn};

use crate::bft::{Vote, VoteCertificate, form_quorum_cert};
use crate::block::{Block, verify_block};
use crate::engine::{Engine, WorkExecutor};
use crate::mempool::Mempool;
use crate::network::{Network, NetworkEvent, Topic};
use crate::state::{ChainState, WorkerId};
use crate::store::Store;
use crate::tx::Transaction;

#[derive(Debug, Clone)]
pub struct NodeConfig {
    pub tick_ms: u64,
    /// How long to wait for votes before giving up on a proposed block.
    pub vote_timeout_ms: u64,
    /// Stake-weighted threshold (micro) for finality (default ⅔).
    pub finality_threshold_micro: u32,
    /// How many rounds without finality before forcing a leader rotation.
    pub stuck_threshold_rounds: u32,
}

impl Default for NodeConfig {
    fn default() -> Self {
        Self {
            tick_ms: 250,
            vote_timeout_ms: 2_000,
            finality_threshold_micro: 666_667,
            stuck_threshold_rounds: 4,
        }
    }
}

/// Shared mutable inner state, used by the driver task.
struct Inner<N: Network, E: WorkExecutor + Send + 'static> {
    sk: SigningKey,
    id: WorkerId,
    engine: Engine,
    mempool: Mempool,
    store: Option<Store>,
    executor: E,
    network: Arc<N>,
    pending_votes: Vec<Vote>,
    /// Block we proposed and are waiting on votes for.
    in_flight: Option<Block>,
    last_qc: Option<VoteCertificate>,
    epoch: u64,
    cfg: NodeConfig,
}

/// A runnable single-validator PoUW node.
pub struct PouwNode<N: Network, E: WorkExecutor + Send + 'static> {
    inner: Arc<Mutex<Inner<N, E>>>,
    handle: JoinHandle<()>,
}

impl<N: Network + 'static, E: WorkExecutor + Send + 'static> PouwNode<N, E> {
    /// Construct + spawn the driver task.
    pub fn spawn(
        sk: SigningKey,
        engine: Engine,
        executor: E,
        network: Arc<N>,
        store: Option<Store>,
        cfg: NodeConfig,
    ) -> Self {
        let id = WorkerId::from_verifying_key(&sk.verifying_key());
        let inner = Arc::new(Mutex::new(Inner {
            sk,
            id,
            engine,
            mempool: Mempool::new(),
            store,
            executor,
            network,
            pending_votes: Vec::new(),
            in_flight: None,
            last_qc: None,
            epoch: 0,
            cfg: cfg.clone(),
        }));

        let inner_for_task = inner.clone();
        let handle = tokio::spawn(async move {
            let mut tick = tokio::time::interval(Duration::from_millis(cfg.tick_ms));
            loop {
                tick.tick().await;
                let mut g = inner_for_task.lock();
                drive_one_tick(&mut g);
            }
        });
        Self { inner, handle }
    }

    /// Submit a transaction to the local mempool + broadcast.
    pub fn submit_tx(&self, tx: Transaction) -> Result<(), crate::tx::TxError> {
        let mut g = self.inner.lock();
        let snap = g.engine.state.clone();
        g.mempool.insert(&snap, tx.clone())?;
        let payload = serde_json::to_vec(&tx).map_err(|_| crate::tx::TxError::InvalidSignature)?;
        let id = g.id;
        g.network.publish(Topic::Jobs, id, payload); // reuse Jobs topic for txs in v0
        Ok(())
    }

    /// Snapshot current chain state.
    pub fn state(&self) -> ChainState {
        self.inner.lock().engine.state.clone()
    }

    /// Stop the driver task.
    pub fn shutdown(&self) {
        self.handle.abort();
    }
}

fn drive_one_tick<N: Network, E: WorkExecutor + Send>(g: &mut Inner<N, E>) {
    // 1. Drain inbound events.
    for evt in g.network.drain() {
        handle_event(g, evt);
    }

    // 2. Try to finalize the in-flight block from collected votes.
    if let Some(in_flight) = g.in_flight.clone() {
        let bh = in_flight.hash();
        let votes_for: Vec<Vote> = g
            .pending_votes
            .iter()
            .filter(|v| v.block_hash == bh)
            .cloned()
            .collect();
        if let Ok(cert) = form_quorum_cert(
            in_flight.header.height,
            bh,
            votes_for,
            &g.engine.state,
            g.cfg.finality_threshold_micro,
        ) {
            // Finalize: apply block + persist + reset state.
            if let Err(e) = g.engine.apply_external_block(&in_flight) {
                warn!("apply finalized in-flight block failed: {e}");
            } else {
                if let Some(store) = &mut g.store {
                    let _ = store.save_block(&in_flight, &g.engine.state);
                }
                g.mempool.remove_included(&in_flight.body.transactions);
                g.mempool.prune(&g.engine.state);
                g.last_qc = Some(cert);
                info!(
                    "finalized block height={} hash={}",
                    in_flight.header.height,
                    hex::encode(&bh[..6])
                );
            }
            g.in_flight = None;
            g.pending_votes
                .retain(|v| v.height > in_flight.header.height);
        }
    }

    // 3. Leader for this epoch?
    if g.in_flight.is_none() && is_leader_for_epoch(g.id, &g.engine.state, g.epoch) {
        propose_block(g);
    }
    g.epoch = g.epoch.wrapping_add(1);
}

fn is_leader_for_epoch(me: WorkerId, state: &ChainState, epoch: u64) -> bool {
    let validators: Vec<WorkerId> = state.validators.iter().copied().collect();
    if validators.is_empty() {
        return false;
    }
    let idx = (epoch as usize) % validators.len();
    validators[idx] == me
}

fn propose_block<N: Network, E: WorkExecutor + Send>(g: &mut Inner<N, E>) {
    let snap = g.engine.state.clone();
    let txs: Vec<Transaction> = g.mempool.take(&snap, crate::DEFAULT_MAX_TX_PER_BLOCK);
    let leader_sk = SigningKey::from_bytes(&g.sk.to_bytes());
    let now = current_ts_ms();
    let parent_qc = g.last_qc.clone();

    let outcome = match g.engine.step_full(
        &mut g.executor,
        &[],
        &txs,
        &leader_sk,
        now,
        parent_qc,
        false, // wait for QC before applying
    ) {
        Ok(o) => o,
        Err(e) => {
            warn!("propose_block step failed: {e}");
            return;
        }
    };

    let block = outcome.block;
    debug!(
        "proposing block height={} hash={}",
        block.header.height,
        hex::encode(&block.hash()[..6])
    );
    let payload = match serde_json::to_vec(&block) {
        Ok(b) => b,
        Err(e) => {
            warn!("encode block: {e}");
            return;
        }
    };
    g.network.publish(Topic::Blocks, g.id, payload);
    // The leader votes for its own block (gossipsub does not echo to the
    // sender, so without this we'd never reach quorum).
    let bh = block.hash();
    let self_vote = Vote::new_signed(block.header.height, bh, &SigningKey::from_bytes(&g.sk.to_bytes()));
    let vote_payload = serde_json::to_vec(&self_vote).unwrap_or_default();
    if !vote_payload.is_empty() {
        g.network.publish(Topic::Votes, g.id, vote_payload);
    }
    g.pending_votes.push(self_vote);
    g.in_flight = Some(block);
}

fn handle_event<N: Network, E: WorkExecutor + Send>(g: &mut Inner<N, E>, evt: NetworkEvent) {
    match evt.topic {
        Topic::Blocks => handle_inbound_block(g, &evt.payload),
        Topic::Votes => handle_inbound_vote(g, &evt.payload),
        Topic::Jobs => handle_inbound_tx(g, &evt.payload),
        _ => {}
    }
}

fn handle_inbound_block<N: Network, E: WorkExecutor + Send>(g: &mut Inner<N, E>, payload: &[u8]) {
    let block: Block = match serde_json::from_slice(payload) {
        Ok(b) => b,
        Err(_) => return,
    };
    if verify_block(&block).is_err() {
        return;
    }
    if block.header.height != g.engine.state.height + 1 {
        return; // stale or future block
    }
    // Sign + broadcast our vote.
    let bh = block.hash();
    let vote = Vote::new_signed(block.header.height, bh, &SigningKey::from_bytes(&g.sk.to_bytes()));
    g.pending_votes.push(vote.clone());
    let payload = match serde_json::to_vec(&vote) {
        Ok(b) => b,
        Err(_) => return,
    };
    g.network.publish(Topic::Votes, g.id, payload);
    // Hold this block for finalization.
    if g.in_flight.is_none() {
        g.in_flight = Some(block);
    }
}

fn handle_inbound_vote<N: Network, E: WorkExecutor + Send>(g: &mut Inner<N, E>, payload: &[u8]) {
    let vote: Vote = match serde_json::from_slice(payload) {
        Ok(v) => v,
        Err(_) => return,
    };
    if vote.verify().is_ok() && g.engine.state.validators.contains(&vote.validator) {
        g.pending_votes.push(vote);
    }
}

fn handle_inbound_tx<N: Network, E: WorkExecutor + Send>(g: &mut Inner<N, E>, payload: &[u8]) {
    let tx: Transaction = match serde_json::from_slice(payload) {
        Ok(t) => t,
        Err(_) => return,
    };
    let snap = g.engine.state.clone();
    let _ = g.mempool.insert(&snap, tx);
}

fn current_ts_ms() -> u64 {
    use std::time::{SystemTime, UNIX_EPOCH};
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}
