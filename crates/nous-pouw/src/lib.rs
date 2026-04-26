//! Proof-of-useful-work consensus for the nous mesh.
//!
//! Each block bundles [`QuorumCertificate`]s — independent worker receipts that
//! agreed (k-of-n, trust-weighted) on the canonical output of a job. Workers
//! that disagreed are slashed. Workers in the winning quorum are minted
//! [`MintReceipt`]s that the `nous-payments` ledger ingests.
//!
//! The "useful work" is whatever an external [`WorkExecutor`] runs — in the
//! reference adapter (`axon-pouw`) that's an axon orchestration workflow
//! (subagent task graph). The consensus engine is agnostic: it sees only
//! [`JobEnvelope`] in, [`WorkReceipt`] out, with the receipt's `output_hash`
//! determining quorum.
//!
//! # Crate boundaries
//!
//! Per the workspace CLAUDE.md, this crate is purely the consensus state
//! machine + sim. No I/O, no DB, no libp2p. The [`Network`] trait is sketched
//! for v1; v0 testing uses the in-process simulator under [`sim`].

pub mod bft;
pub mod block;
pub mod engine;
pub mod envelope;
pub mod mempool;
pub mod mint;
pub mod net;
pub mod network;
pub mod node;
pub mod quorum;
pub mod receipt;
pub mod rpc;
pub mod selection;
pub mod sim;
pub mod slashing;
pub mod state;
pub mod store;
pub mod tx;

pub use bft::{
    BftError, Vote, VoteCertificate, detect_double_vote, form_quorum_cert, tally_by_block,
    verify_quorum_cert,
};
pub use block::{Block, BlockBody, BlockHash, BlockHeader, BlockHeight, sign_block, verify_block};
pub use engine::{Engine, EngineConfig, RoundOutcome};
pub use envelope::{JobEnvelope, JobId, ModelPin};
pub use mempool::{DEFAULT_MAX_TX_PER_BLOCK, Mempool};
pub use mint::{MintReceipt, MintSink};
pub use network::{Network, NetworkEvent, Topic};
pub use quorum::{QuorumCertificate, QuorumError, form_quorum};
pub use receipt::{
    OutputHash, ReceiptCommitment, RubricScore, TraceRoot, WorkReceipt, sign_receipt,
};
pub use selection::{select_workers, vrf_score};
pub use slashing::{EquivocationProof, SlashEvent, SlashKind, detect_equivocation};
pub use state::{ChainState, StateError, StateRoot, WorkerId, WorkerInfo};
pub use tx::{Transaction, TxBody, TxError};
