//! HTTP RPC for [`PouwNode`].
//!
//! Minimal axum-based JSON API exposing read-only chain queries plus a
//! `POST /tx` endpoint for submitting signed transactions. Each route holds
//! a snapshot reference to the node so requests don't block the consensus
//! tick more than the time it takes to clone state.
//!
//! ```ignore
//! let node = PouwNode::spawn(...);
//! let rpc = RpcServer::spawn(&node, "127.0.0.1:8080".parse().unwrap()).await?;
//! ```
//!
//! Endpoints:
//! - `GET  /status`            — chain height, head hash, supply, peer count
//! - `GET  /balance/:did`      — balance + stake + nonce for a DID:key worker
//! - `GET  /head`              — latest finalized block (JSON)
//! - `GET  /block/:height`     — block at height (404 if missing)
//! - `POST /tx`                — submit a signed `Transaction`
//! - `GET  /peers`             — current peer count

use std::net::SocketAddr;
use std::sync::Arc;

use axum::Router;
use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::Json;
use axum::routing::{get, post};
use serde::{Deserialize, Serialize};
use tokio::net::TcpListener;
use tokio::task::JoinHandle;

use crate::block::{Block, BlockHeight};
use crate::engine::WorkExecutor;
use crate::network::Network;
use crate::node::PouwNode;
use crate::state::WorkerId;
use crate::store::Store;
use crate::tx::Transaction;

/// Trait erasing the generic parameters of `PouwNode` so an `axum::Router` can
/// hold a single shared handle, regardless of what `Network`/`WorkExecutor`
/// it was built with.
pub trait NodeHandle: Send + Sync + 'static {
    fn snapshot(&self) -> NodeSnapshot;
    fn submit_tx(&self, tx: Transaction) -> Result<[u8; 32], String>;
    fn block_at(&self, height: BlockHeight) -> Option<Block>;
    fn head_block(&self) -> Option<Block>;
    fn workers(&self) -> std::collections::BTreeMap<WorkerId, crate::state::WorkerInfo>;
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeSnapshot {
    pub height: BlockHeight,
    pub head_hash_hex: String,
    pub total_supply: u64,
    pub validators: usize,
    pub workers: usize,
    pub peer_count: usize,
    pub mempool_len: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BalanceResponse {
    pub did: String,
    pub balance: u64,
    pub stake: u64,
    pub nonce: u64,
    pub trust: f64,
    pub slashed: bool,
}

#[derive(Debug, Deserialize)]
pub struct SubmitTxRequest {
    pub tx: Transaction,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SubmitTxResponse {
    pub tx_id_hex: String,
}

#[derive(Debug, Serialize)]
pub struct ErrorBody {
    pub error: String,
}

impl<N, E> NodeHandle for PouwNode<N, E>
where
    N: Network + 'static,
    E: WorkExecutor + Send + 'static,
{
    fn snapshot(&self) -> NodeSnapshot {
        let s = self.state();
        NodeSnapshot {
            height: s.height,
            head_hash_hex: hex::encode(s.head_hash),
            total_supply: s.total_supply,
            validators: s.validators.len(),
            workers: s.workers.len(),
            peer_count: self.peer_count(),
            mempool_len: self.mempool_len(),
        }
    }

    fn submit_tx(&self, tx: Transaction) -> Result<[u8; 32], String> {
        let id = tx.id();
        self.submit_tx(tx).map_err(|e| e.to_string())?;
        Ok(id)
    }

    fn block_at(&self, height: BlockHeight) -> Option<Block> {
        self.lookup_block_at(height)
    }

    fn head_block(&self) -> Option<Block> {
        self.lookup_head_block()
    }

    fn workers(&self) -> std::collections::BTreeMap<WorkerId, crate::state::WorkerInfo> {
        self.state().workers
    }
}

#[derive(Clone)]
pub struct RpcState {
    handle: Arc<dyn NodeHandle>,
}

pub struct RpcServer {
    pub local_addr: SocketAddr,
    pub task: JoinHandle<()>,
}

impl RpcServer {
    /// Spawn an HTTP server on `bind_addr` (e.g. `"127.0.0.1:8080".parse().unwrap()`).
    pub async fn spawn(
        handle: Arc<dyn NodeHandle>,
        bind_addr: SocketAddr,
    ) -> std::io::Result<Self> {
        let listener = TcpListener::bind(bind_addr).await?;
        let local_addr = listener.local_addr()?;
        let state = RpcState { handle };
        let app = build_router(state);
        let task = tokio::spawn(async move {
            if let Err(e) = axum::serve(listener, app).await {
                tracing::warn!("rpc serve ended: {e}");
            }
        });
        Ok(Self { local_addr, task })
    }
}

impl Drop for RpcServer {
    fn drop(&mut self) {
        self.task.abort();
    }
}

fn build_router(state: RpcState) -> Router {
    Router::new()
        .route("/status", get(get_status))
        .route("/balance/{did}", get(get_balance))
        .route("/peers", get(get_peers))
        .route("/tx", post(post_tx))
        .route("/head", get(get_head))
        .route("/block/{height}", get(get_block_at))
        .with_state(state)
}

async fn get_status(State(s): State<RpcState>) -> Json<NodeSnapshot> {
    Json(s.handle.snapshot())
}

async fn get_peers(State(s): State<RpcState>) -> Json<serde_json::Value> {
    Json(serde_json::json!({ "peer_count": s.handle.snapshot().peer_count }))
}

async fn get_head(State(s): State<RpcState>) -> Result<Json<Block>, (StatusCode, Json<ErrorBody>)> {
    s.handle.head_block().map(Json).ok_or((
        StatusCode::NOT_FOUND,
        Json(ErrorBody {
            error: "no finalized blocks yet".into(),
        }),
    ))
}

async fn get_block_at(
    State(s): State<RpcState>,
    Path(height): Path<BlockHeight>,
) -> Result<Json<Block>, (StatusCode, Json<ErrorBody>)> {
    s.handle.block_at(height).map(Json).ok_or((
        StatusCode::NOT_FOUND,
        Json(ErrorBody {
            error: format!("no block at height {height}"),
        }),
    ))
}

async fn get_balance(
    State(s): State<RpcState>,
    Path(did): Path<String>,
) -> Result<Json<BalanceResponse>, (StatusCode, Json<ErrorBody>)> {
    // Convert DID:key back to ed25519 verifying key bytes.
    let vk = nous_crypto::keys::did_to_public_key(&did).map_err(|e| {
        (
            StatusCode::BAD_REQUEST,
            Json(ErrorBody {
                error: format!("invalid did:key: {e}"),
            }),
        )
    })?;
    let id = WorkerId::from_verifying_key(&vk);
    let snap = s.handle.snapshot();
    // Pull the worker from the held node via a small extension point on the
    // trait. We add it inline with downcast-by-state for simplicity.
    let info = lookup_worker(s.handle.as_ref(), &id);
    let info = match info {
        Some(i) => i,
        None => {
            return Err((
                StatusCode::NOT_FOUND,
                Json(ErrorBody {
                    error: format!("unknown worker {did}"),
                }),
            ));
        }
    };
    let _ = snap;
    Ok(Json(BalanceResponse {
        did,
        balance: info.balance,
        stake: info.stake,
        nonce: info.nonce,
        trust: info.trust,
        slashed: info.slashed,
    }))
}

async fn post_tx(
    State(s): State<RpcState>,
    Json(req): Json<SubmitTxRequest>,
) -> Result<Json<SubmitTxResponse>, (StatusCode, Json<ErrorBody>)> {
    let id = s
        .handle
        .submit_tx(req.tx)
        .map_err(|e| (StatusCode::BAD_REQUEST, Json(ErrorBody { error: e })))?;
    Ok(Json(SubmitTxResponse {
        tx_id_hex: hex::encode(id),
    }))
}

fn lookup_worker(handle: &dyn NodeHandle, id: &WorkerId) -> Option<crate::state::WorkerInfo> {
    handle.workers().get(id).cloned()
}

#[allow(dead_code)]
fn _store_anchor(_s: Option<&Store>) {}
