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

use std::collections::BTreeMap;
use std::sync::Arc;
use std::time::{Duration, Instant};

use ed25519_dalek::SigningKey;
use parking_lot::Mutex;
use serde::{Deserialize, Serialize};
use tokio::task::JoinHandle;
use tracing::{debug, info, warn};

use crate::bft::{Vote, VoteCertificate, form_quorum_cert};
use crate::block::{Block, BlockHeight, verify_block};
use crate::engine::{Engine, WorkExecutor};
use crate::mempool::Mempool;
use crate::network::{Network, NetworkEvent, Topic};
use crate::state::{ChainState, WorkerId};
use crate::store::Store;
use crate::tx::Transaction;

/// Number of recently-finalized blocks every node keeps in memory to serve
/// to peers that fall behind the chain head.
const RECENT_BLOCKS_CACHE: usize = 64;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct SyncReq {
    pub from_height: BlockHeight,
    pub to_height: BlockHeight,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct SyncResp {
    pub blocks: Vec<Block>,
}

#[derive(Debug, Clone)]
pub struct NodeConfig {
    pub tick_ms: u64,
    /// Drop the in-flight block if it has held this long without forming a QC.
    /// Lets the next leader try afresh and unblocks chains that hit a flaky
    /// proposal during mesh warmup.
    pub vote_timeout_ms: u64,
    /// Stake-weighted threshold (micro) for finality (default ⅔).
    pub finality_threshold_micro: u32,
    /// Don't propose or vote until this much time has passed since spawn.
    /// Lets the libp2p gossipsub mesh form before the first round.
    pub warmup_ms: u64,
    /// Don't propose until at least this many peers are connected (real net).
    /// Sim networks return `usize::MAX` from `peer_count()`, so this gate is
    /// trivially satisfied for in-process tests.
    pub min_peers_to_propose: usize,
}

impl Default for NodeConfig {
    fn default() -> Self {
        Self {
            tick_ms: 250,
            vote_timeout_ms: 5_000,
            finality_threshold_micro: 666_667,
            warmup_ms: 0,
            min_peers_to_propose: 0,
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
    /// When the in-flight block was set; used to time out stuck proposals.
    in_flight_at: Option<Instant>,
    last_qc: Option<VoteCertificate>,
    cfg: NodeConfig,
    started_at: Instant,
    /// Wall-clock anchor used to compute the current view round (= number of
    /// `vote_timeout_ms` windows that have elapsed since the last block
    /// finalized). Every node sees the same view round modulo clock skew, so
    /// they all agree on the leader for height H+1 at any given moment.
    last_finalize_at: Instant,
    /// Last `RECENT_BLOCKS_CACHE` finalized blocks, by height. Served to
    /// peers that fall behind during mesh formation or churn.
    recent_blocks: BTreeMap<BlockHeight, Block>,
    /// Future blocks (height > self.height + 1) we've received but can't
    /// apply yet. Applied in order once the gap closes via sync.
    pending_blocks: BTreeMap<BlockHeight, Block>,
    /// When we last sent a sync request — debounce so we don't flood.
    last_sync_req_at: Option<Instant>,
}

/// A runnable single-validator PoUW node.
pub struct PouwNode<N: Network, E: WorkExecutor + Send + 'static> {
    inner: Arc<Mutex<Inner<N, E>>>,
    network: Arc<N>,
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
        let network_for_handle = network.clone();
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
            in_flight_at: None,
            last_qc: None,
            cfg: cfg.clone(),
            started_at: Instant::now(),
            last_finalize_at: Instant::now(),
            recent_blocks: BTreeMap::new(),
            pending_blocks: BTreeMap::new(),
            last_sync_req_at: None,
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
        Self {
            inner,
            network: network_for_handle,
            handle,
        }
    }

    /// Peer count from the underlying network (for diagnostics / readiness).
    pub fn peer_count(&self) -> usize {
        <N as Network>::peer_count(self.network.as_ref())
    }

    /// Mempool size (pending tx count).
    pub fn mempool_len(&self) -> usize {
        self.inner.lock().mempool.len()
    }

    /// Look up a block by height: first the in-memory recent_blocks LRU,
    /// then fall back to the persisted store.
    pub fn lookup_block_at(&self, height: BlockHeight) -> Option<Block> {
        let g = self.inner.lock();
        if let Some(b) = g.recent_blocks.get(&height) {
            return Some(b.clone());
        }
        if let Some(store) = &g.store
            && let Ok(Some(b)) = store.block_at(height)
        {
            return Some(b);
        }
        None
    }

    /// Latest finalized block, if any.
    pub fn lookup_head_block(&self) -> Option<Block> {
        let g = self.inner.lock();
        let h = g.engine.state.height;
        if h == 0 {
            return None;
        }
        if let Some(b) = g.recent_blocks.get(&h) {
            return Some(b.clone());
        }
        if let Some(store) = &g.store
            && let Ok(Some(b)) = store.block_at(h)
        {
            return Some(b);
        }
        None
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
    // 1. Drain inbound events (always — we want to absorb peer traffic
    //    even during warmup so our pending_votes is hot).
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
                cache_recent(&mut g.recent_blocks, in_flight.clone());
                g.last_finalize_at = Instant::now();
                // After advancing the head, see if any buffered future
                // blocks are now applicable.
                apply_pending_blocks(g);
            }
            g.in_flight = None;
            g.in_flight_at = None;
            g.pending_votes
                .retain(|v| v.height > in_flight.header.height);
        } else if let Some(t0) = g.in_flight_at {
            // Vote-timeout: drop a stale in-flight so the next leader (per the
            // current view round) can try.
            if t0.elapsed() > Duration::from_millis(g.cfg.vote_timeout_ms) {
                debug!(
                    "in_flight block height={} timed out without QC, retrying",
                    in_flight.header.height
                );
                g.in_flight = None;
                g.in_flight_at = None;
                g.pending_votes
                    .retain(|v| v.height != in_flight.header.height);
            }
        }
    }

    // 3. Warmup gate: don't propose until the mesh has had a chance to form.
    let warmed_up = g.started_at.elapsed() >= Duration::from_millis(g.cfg.warmup_ms);
    let enough_peers = <N as Network>::peer_count(g.network.as_ref()) >= g.cfg.min_peers_to_propose;

    if warmed_up && enough_peers && g.in_flight.is_none() && is_leader_for_next_block(g) {
        propose_block(g);
    }
}

/// Wall-clock-driven view round for the current next-block slot. Every node
/// computes the same value from `last_finalize_at` modulo clock skew, so the
/// leader rotation is deterministic across the cluster: if the primary
/// leader is slow / offline, the backup at view+1 takes over uniformly.
fn current_view_round<N: Network, E: WorkExecutor + Send>(g: &Inner<N, E>) -> u64 {
    let elapsed_ms = g.last_finalize_at.elapsed().as_millis() as u64;
    elapsed_ms / g.cfg.vote_timeout_ms.max(1)
}

fn is_leader_for_next_block<N: Network, E: WorkExecutor + Send>(g: &Inner<N, E>) -> bool {
    let validators: Vec<WorkerId> = g.engine.state.validators.iter().copied().collect();
    if validators.is_empty() {
        return false;
    }
    let next_height = g.engine.state.height + 1;
    let view = current_view_round(g);
    let idx = ((next_height + view) as usize) % validators.len();
    validators[idx] == g.id
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
    let self_vote = Vote::new_signed(
        block.header.height,
        bh,
        &SigningKey::from_bytes(&g.sk.to_bytes()),
    );
    let vote_payload = serde_json::to_vec(&self_vote).unwrap_or_default();
    if !vote_payload.is_empty() {
        g.network.publish(Topic::Votes, g.id, vote_payload);
    }
    g.pending_votes.push(self_vote);
    g.in_flight = Some(block);
    g.in_flight_at = Some(Instant::now());
}

fn handle_event<N: Network, E: WorkExecutor + Send>(g: &mut Inner<N, E>, evt: NetworkEvent) {
    match evt.topic {
        Topic::Blocks => handle_inbound_block(g, &evt.payload),
        Topic::Votes => handle_inbound_vote(g, &evt.payload),
        Topic::Jobs => handle_inbound_tx(g, &evt.payload),
        Topic::SyncRequest => handle_sync_request(g, &evt.payload),
        Topic::SyncResponse => handle_sync_response(g, &evt.payload),
        _ => {}
    }
}

fn handle_sync_request<N: Network, E: WorkExecutor + Send>(g: &mut Inner<N, E>, payload: &[u8]) {
    let req: SyncReq = match serde_json::from_slice(payload) {
        Ok(r) => r,
        Err(_) => return,
    };
    // Cap the response to avoid huge messages.
    let cap = 32u64;
    let from = req.from_height.max(1);
    let to = req.to_height.min(from + cap);
    let mut blocks = Vec::new();
    for h in from..=to {
        if let Some(b) = g.recent_blocks.get(&h) {
            blocks.push(b.clone());
            continue;
        }
        if let Some(store) = &g.store
            && let Ok(Some(b)) = store.block_at(h)
        {
            blocks.push(b);
        }
    }
    if blocks.is_empty() {
        return;
    }
    let resp = SyncResp { blocks };
    if let Ok(bytes) = serde_json::to_vec(&resp) {
        g.network.publish(Topic::SyncResponse, g.id, bytes);
    }
}

fn handle_sync_response<N: Network, E: WorkExecutor + Send>(g: &mut Inner<N, E>, payload: &[u8]) {
    let resp: SyncResp = match serde_json::from_slice(payload) {
        Ok(r) => r,
        Err(_) => return,
    };
    for block in resp.blocks {
        if verify_block(&block).is_err() {
            continue;
        }
        if block.header.height <= g.engine.state.height {
            continue; // already have it
        }
        g.pending_blocks.insert(block.header.height, block);
    }
    apply_pending_blocks(g);
}

fn apply_pending_blocks<N: Network, E: WorkExecutor + Send>(g: &mut Inner<N, E>) {
    loop {
        let next = g.engine.state.height + 1;
        let block = match g.pending_blocks.remove(&next) {
            Some(b) => b,
            None => return,
        };
        if let Err(e) = g.engine.apply_external_block(&block) {
            warn!("apply_pending_blocks failed at height {}: {e}", next);
            return;
        }
        if let Some(store) = &mut g.store {
            let _ = store.save_block(&block, &g.engine.state);
        }
        g.mempool.remove_included(&block.body.transactions);
        g.mempool.prune(&g.engine.state);
        cache_recent(&mut g.recent_blocks, block);
    }
}

fn cache_recent(cache: &mut BTreeMap<BlockHeight, Block>, block: Block) {
    cache.insert(block.header.height, block);
    while cache.len() > RECENT_BLOCKS_CACHE {
        let lowest = *cache.keys().next().expect("non-empty just checked");
        cache.remove(&lowest);
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
    let want = g.engine.state.height + 1;
    if block.header.height < want {
        return; // stale block; we already moved past it
    }
    if block.header.height > want {
        // Future block — buffer it and ask a peer for the gap.
        g.pending_blocks.insert(block.header.height, block);
        request_sync_if_needed(g);
        return;
    }
    // For height == want, the proposer is the height-based leader for this
    // round. Multiple proposers can race during view-change drift: we vote
    // for the FIRST verifiable block we see for this height and hold it as
    // our in_flight; later proposals at the same height are ignored (a
    // standard primary-backup BFT discipline that prevents accidental
    // double-finalization on this branch).
    if let Some(existing) = &g.in_flight
        && existing.header.height == block.header.height
    {
        return;
    }
    let bh = block.hash();
    let vote = Vote::new_signed(
        block.header.height,
        bh,
        &SigningKey::from_bytes(&g.sk.to_bytes()),
    );
    g.pending_votes.push(vote.clone());
    let payload = match serde_json::to_vec(&vote) {
        Ok(b) => b,
        Err(_) => return,
    };
    g.network.publish(Topic::Votes, g.id, payload);
    g.in_flight = Some(block);
    g.in_flight_at = Some(Instant::now());
}

fn request_sync_if_needed<N: Network, E: WorkExecutor + Send>(g: &mut Inner<N, E>) {
    // Don't spam — send a fresh request at most every 1 second.
    if let Some(t) = g.last_sync_req_at
        && t.elapsed() < Duration::from_secs(1)
    {
        return;
    }
    let from_height = g.engine.state.height + 1;
    let to_height = g
        .pending_blocks
        .keys()
        .next_back()
        .copied()
        .unwrap_or(from_height);
    if to_height < from_height {
        return;
    }
    let req = SyncReq {
        from_height,
        to_height,
    };
    if let Ok(bytes) = serde_json::to_vec(&req) {
        g.network.publish(Topic::SyncRequest, g.id, bytes);
        g.last_sync_req_at = Some(Instant::now());
        debug!("sent sync request from {} to {}", from_height, to_height);
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
