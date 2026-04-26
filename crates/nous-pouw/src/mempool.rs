//! Pending transaction pool.
//!
//! v0 design: in-memory `BTreeMap<(sender, nonce), Transaction>` ordered for
//! deterministic block selection. Block leaders call [`Mempool::take`] to
//! drain up to N transactions sorted by (priority desc, nonce asc).
//!
//! Replays / late arrivals are dropped: a tx whose nonce ≤ the chain's
//! current sender nonce is discarded on insert.

use std::collections::BTreeMap;

use crate::state::{ChainState, WorkerId};
use crate::tx::{Transaction, TxError};

/// Maximum txs returned from one [`take`](Mempool::take) call (block size cap).
pub const DEFAULT_MAX_TX_PER_BLOCK: usize = 256;

#[derive(Debug, Default)]
pub struct Mempool {
    /// Keyed by (sender, nonce) for stable ordering.
    txs: BTreeMap<(WorkerId, u64), Transaction>,
    /// Total bytes currently held, for cheap memory pressure metrics.
    bytes: usize,
}

impl Mempool {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn len(&self) -> usize {
        self.txs.len()
    }

    pub fn is_empty(&self) -> bool {
        self.txs.is_empty()
    }

    pub fn bytes(&self) -> usize {
        self.bytes
    }

    /// Insert a transaction. Returns `Ok(())` on success, `Err` if the tx
    /// would be rejected by the chain (bad signature, stale nonce, etc).
    /// Idempotent: re-inserting the same tx is a no-op.
    pub fn insert(&mut self, state: &ChainState, tx: Transaction) -> Result<(), TxError> {
        tx.verify_signature()?;
        let sender = tx.body.sender();
        // Nonce must strictly exceed the chain's current sender nonce.
        let chain_nonce = state.workers.get(&sender).map(|w| w.nonce).unwrap_or(0);
        if tx.nonce <= chain_nonce {
            return Err(TxError::NonceMismatch {
                expected: chain_nonce + 1,
                actual: tx.nonce,
            });
        }
        let key = (sender, tx.nonce);
        if !self.txs.contains_key(&key) {
            self.bytes += tx.signing_bytes().len();
            self.txs.insert(key, tx);
        }
        Ok(())
    }

    /// Drain up to `max` txs in canonical order: per-sender by nonce
    /// ascending, across senders by lex(WorkerId).
    ///
    /// We only return contiguous nonce streams — i.e. sender X's tx with
    /// nonce N is included only if all of N-1, N-2, … back to (chain_nonce+1)
    /// are also present. Skips senders with gaps.
    pub fn take(&mut self, state: &ChainState, max: usize) -> Vec<Transaction> {
        let mut out = Vec::new();
        let mut by_sender: BTreeMap<WorkerId, Vec<u64>> = BTreeMap::new();
        for (sender, nonce) in self.txs.keys() {
            by_sender.entry(*sender).or_default().push(*nonce);
        }
        for (sender, mut nonces) in by_sender {
            nonces.sort();
            let chain_nonce = state.workers.get(&sender).map(|w| w.nonce).unwrap_or(0);
            let mut next_expected = chain_nonce + 1;
            for n in nonces {
                if n != next_expected {
                    break;
                }
                if out.len() == max {
                    break;
                }
                if let Some(tx) = self.txs.remove(&(sender, n)) {
                    self.bytes = self.bytes.saturating_sub(tx.signing_bytes().len());
                    out.push(tx);
                    next_expected += 1;
                }
            }
            if out.len() == max {
                break;
            }
        }
        out
    }

    /// Drop txs that are now stale (chain advanced past them).
    pub fn prune(&mut self, state: &ChainState) {
        self.txs.retain(|(sender, nonce), tx| {
            let chain_nonce = state.workers.get(sender).map(|w| w.nonce).unwrap_or(0);
            let keep = *nonce > chain_nonce;
            if !keep {
                self.bytes = self.bytes.saturating_sub(tx.signing_bytes().len());
            }
            keep
        });
    }

    /// Remove specific txs (e.g. once they've been included in a finalized block).
    pub fn remove_included(&mut self, txs: &[Transaction]) {
        for tx in txs {
            let key = (tx.body.sender(), tx.nonce);
            if let Some(removed) = self.txs.remove(&key) {
                self.bytes = self.bytes.saturating_sub(removed.signing_bytes().len());
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::state::ChainState;
    use crate::tx::TxBody;
    use ed25519_dalek::SigningKey;
    use rand::rngs::OsRng;

    fn worker_with_balance(state: &mut ChainState, balance: u64) -> (SigningKey, WorkerId) {
        let sk = SigningKey::generate(&mut OsRng);
        let w = WorkerId::from_verifying_key(&sk.verifying_key());
        state.register_worker(w, 0, 1.0);
        // Stake/balance/nonce are mutated directly for tests since there's no
        // free-money tx in v0.
        state.workers.get_mut(&w).unwrap().balance = balance;
        (sk, w)
    }

    fn transfer_tx(sk: &SigningKey, from: WorkerId, to: WorkerId, amt: u64, nonce: u64) -> Transaction {
        Transaction::new_signed(
            TxBody::Transfer {
                from,
                to,
                amount: amt,
            },
            nonce,
            0,
            sk,
        )
    }

    #[test]
    fn insert_then_take_returns_tx() {
        let mut state = ChainState::new();
        let (sk, from) = worker_with_balance(&mut state, 100);
        let (_, to) = worker_with_balance(&mut state, 0);
        let mut mp = Mempool::new();
        mp.insert(&state, transfer_tx(&sk, from, to, 10, 1)).unwrap();
        assert_eq!(mp.len(), 1);
        let taken = mp.take(&state, 100);
        assert_eq!(taken.len(), 1);
        assert!(mp.is_empty());
    }

    #[test]
    fn stale_nonce_rejected() {
        let mut state = ChainState::new();
        let (sk, from) = worker_with_balance(&mut state, 100);
        let (_, to) = worker_with_balance(&mut state, 0);
        // Chain already advanced sender nonce to 5.
        state.workers.get_mut(&from).unwrap().nonce = 5;
        let mut mp = Mempool::new();
        let err = mp
            .insert(&state, transfer_tx(&sk, from, to, 3, 0))
            .unwrap_err();
        assert!(matches!(err, TxError::NonceMismatch { .. }));
    }

    #[test]
    fn take_returns_contiguous_nonces_only() {
        let mut state = ChainState::new();
        let (sk, from) = worker_with_balance(&mut state, 100);
        let (_, to) = worker_with_balance(&mut state, 0);
        let mut mp = Mempool::new();
        // Insert nonces 1, 2, 4 (skip 3 → 4 is unreachable).
        mp.insert(&state, transfer_tx(&sk, from, to, 1, 1)).unwrap();
        mp.insert(&state, transfer_tx(&sk, from, to, 1, 2)).unwrap();
        mp.insert(&state, transfer_tx(&sk, from, to, 1, 4)).unwrap();
        let taken = mp.take(&state, 100);
        assert_eq!(taken.len(), 2);
        assert_eq!(taken[0].nonce, 1);
        assert_eq!(taken[1].nonce, 2);
        assert_eq!(mp.len(), 1, "nonce-4 should remain");
    }

    #[test]
    fn duplicate_insert_is_idempotent() {
        let mut state = ChainState::new();
        let (sk, from) = worker_with_balance(&mut state, 100);
        let (_, to) = worker_with_balance(&mut state, 0);
        let mut mp = Mempool::new();
        let tx = transfer_tx(&sk, from, to, 1, 1);
        mp.insert(&state, tx.clone()).unwrap();
        mp.insert(&state, tx).unwrap();
        assert_eq!(mp.len(), 1);
    }

    #[test]
    fn take_respects_max() {
        let mut state = ChainState::new();
        let (sk, from) = worker_with_balance(&mut state, 1000);
        let (_, to) = worker_with_balance(&mut state, 0);
        let mut mp = Mempool::new();
        for n in 1..=5 {
            mp.insert(&state, transfer_tx(&sk, from, to, 1, n)).unwrap();
        }
        let taken = mp.take(&state, 3);
        assert_eq!(taken.len(), 3);
        assert_eq!(mp.len(), 2);
    }

    #[test]
    fn prune_drops_stale_txs() {
        let mut state = ChainState::new();
        let (sk, from) = worker_with_balance(&mut state, 100);
        let (_, to) = worker_with_balance(&mut state, 0);
        let mut mp = Mempool::new();
        mp.insert(&state, transfer_tx(&sk, from, to, 1, 1)).unwrap();
        mp.insert(&state, transfer_tx(&sk, from, to, 1, 2)).unwrap();
        // Chain advances past nonce 1.
        state.workers.get_mut(&from).unwrap().nonce = 1;
        mp.prune(&state);
        assert_eq!(mp.len(), 1);
    }

    #[test]
    fn invalid_signature_rejected() {
        let mut state = ChainState::new();
        let (sk, from) = worker_with_balance(&mut state, 100);
        let (_, to) = worker_with_balance(&mut state, 0);
        let mut tx = transfer_tx(&sk, from, to, 1, 1);
        tx.signature[0] ^= 0xff;
        let mut mp = Mempool::new();
        assert!(mp.insert(&state, tx).is_err());
    }

    #[test]
    fn cross_sender_canonical_order() {
        let mut state = ChainState::new();
        let (sk_a, a) = worker_with_balance(&mut state, 100);
        let (sk_b, b) = worker_with_balance(&mut state, 100);
        let (_, dest) = worker_with_balance(&mut state, 0);
        let mut mp = Mempool::new();
        mp.insert(&state, transfer_tx(&sk_b, b, dest, 1, 1)).unwrap();
        mp.insert(&state, transfer_tx(&sk_a, a, dest, 1, 1)).unwrap();
        let taken = mp.take(&state, 10);
        // Both included; ordering is BTreeMap-stable on WorkerId.
        assert_eq!(taken.len(), 2);
        let sender_a_first = taken[0].body.sender() == a.min(b);
        assert!(sender_a_first || taken[0].body.sender() == a.min(b));
    }
}
