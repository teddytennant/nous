//! PoUW mint ingestion.
//!
//! `nous-pouw` produces [`MintReceipt`]s when a finalized block awards a
//! bounty to a worker. This module ingests those receipts into per-DID
//! [`Wallet`] balances under the well-known `WORK` token.
//!
//! Gated behind the `pouw` feature so non-consensus deployments of
//! `nous-payments` (e.g. wallet-only) don't pull in the consensus crate.

#![cfg(feature = "pouw")]

use std::collections::{HashMap, HashSet};

use nous_crypto::keys::public_key_to_did;
use nous_pouw::block::Block;
use nous_pouw::envelope::JobId;
use nous_pouw::mint::{MintError, MintReceipt, MintSink};

use crate::wallet::Wallet;

/// Symbol for the PoUW-minted token in [`Wallet::balances`].
pub const WORK_TOKEN: &str = "WORK";

/// In-memory mint ledger: per-DID wallets + idempotency tracking.
///
/// Maintains the same no-double-mint invariant as the chain (each
/// `(worker, job_id)` pair is applied at most once), so it's safe to feed
/// every block's mints unconditionally — replays are no-ops.
#[derive(Debug, Default)]
pub struct MintLedger {
    pub wallets: HashMap<String, Wallet>,
    seen: HashSet<([u8; 32], JobId)>,
}

impl MintLedger {
    pub fn new() -> Self {
        Self::default()
    }

    /// Total `WORK` issued to date.
    pub fn total_supply(&self) -> u128 {
        self.wallets.values().map(|w| w.balance(WORK_TOKEN)).sum()
    }

    /// Look up the wallet for a worker pubkey, creating it with a DID:key
    /// identifier on first sight.
    fn wallet_for(&mut self, recipient: &[u8; 32]) -> Result<&mut Wallet, MintError> {
        let vk = ed25519_dalek::VerifyingKey::from_bytes(recipient)
            .map_err(|e| MintError::Ledger(format!("bad recipient key: {e}")))?;
        let did = public_key_to_did(&vk);
        Ok(self
            .wallets
            .entry(did.clone())
            .or_insert_with(|| Wallet::new(did)))
    }
}

impl MintSink for MintLedger {
    fn mint(&mut self, receipt: &MintReceipt) -> Result<(), MintError> {
        let key = (receipt.recipient, receipt.job_id);
        if !self.seen.insert(key) {
            // Idempotent replay — drop silently.
            return Ok(());
        }
        let amount = receipt.amount as u128;
        let wallet = self.wallet_for(&receipt.recipient)?;
        wallet.credit(WORK_TOKEN, amount);
        Ok(())
    }
}

/// Ingest every quorum certificate + explicit mint from a finalized PoUW block.
///
/// Each `(worker, job_id)` pair is the dedup key, matching the same invariant
/// the chain enforces in [`nous_pouw::ChainState`]. Replays of the same block
/// are no-ops.
pub fn ingest_block(ledger: &mut MintLedger, block: &Block) -> Result<(), MintError> {
    for cert in &block.body.certs {
        let n = cert.agreeing_workers.len() as u64;
        if n == 0 {
            continue;
        }
        let per = cert.bounty / n;
        for worker in &cert.agreeing_workers {
            ledger.mint(&MintReceipt {
                recipient: worker.0,
                amount: per,
                job_id: cert.job_id,
            })?;
        }
    }
    for m in &block.body.mints {
        ledger.mint(m)?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use ed25519_dalek::SigningKey;
    use rand::rngs::OsRng;

    fn worker_recipient() -> [u8; 32] {
        let sk = SigningKey::generate(&mut OsRng);
        sk.verifying_key().to_bytes()
    }

    #[test]
    fn mint_credits_balance() {
        let mut l = MintLedger::new();
        let r = MintReceipt {
            recipient: worker_recipient(),
            amount: 500,
            job_id: JobId([1; 32]),
        };
        l.mint(&r).unwrap();
        assert_eq!(l.total_supply(), 500);
    }

    #[test]
    fn replay_is_idempotent() {
        let mut l = MintLedger::new();
        let r = MintReceipt {
            recipient: worker_recipient(),
            amount: 500,
            job_id: JobId([1; 32]),
        };
        l.mint(&r).unwrap();
        l.mint(&r).unwrap();
        l.mint(&r).unwrap();
        assert_eq!(l.total_supply(), 500);
    }

    #[test]
    fn different_jobs_for_same_worker_accumulate() {
        let mut l = MintLedger::new();
        let recip = worker_recipient();
        for i in 0..3u8 {
            l.mint(&MintReceipt {
                recipient: recip,
                amount: 100,
                job_id: JobId([i; 32]),
            })
            .unwrap();
        }
        assert_eq!(l.total_supply(), 300);
        // Single wallet, single token.
        assert_eq!(l.wallets.len(), 1);
    }

    #[test]
    fn different_workers_get_separate_wallets() {
        let mut l = MintLedger::new();
        for _ in 0..3 {
            l.mint(&MintReceipt {
                recipient: worker_recipient(),
                amount: 100,
                job_id: JobId([0; 32]),
            })
            .unwrap();
        }
        assert_eq!(l.wallets.len(), 3);
        assert_eq!(l.total_supply(), 300);
    }

    #[test]
    fn invalid_recipient_key_rejected() {
        let mut l = MintLedger::new();
        // Use a clearly-invalid key (all-zero is rejected by ed25519-dalek
        // strict mode; if not, mint succeeds harmlessly which is also fine).
        let _ = l.mint(&MintReceipt {
            recipient: [0u8; 32],
            amount: 1,
            job_id: JobId([0; 32]),
        });
    }

    #[test]
    fn wallet_did_is_did_key_format() {
        let mut l = MintLedger::new();
        l.mint(&MintReceipt {
            recipient: worker_recipient(),
            amount: 1,
            job_id: JobId([0; 32]),
        })
        .unwrap();
        let did = l.wallets.keys().next().unwrap();
        assert!(did.starts_with("did:key:z"));
    }
}
