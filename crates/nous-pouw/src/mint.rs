//! Mint receipts: the link between consensus and the payments ledger.
//!
//! `nous-pouw` produces [`MintReceipt`]s; the `nous-payments::mint` module
//! ingests them via the [`MintSink`] trait. This decoupling lets the
//! consensus engine stay unaware of `nous-payments`' internals.

use serde::{Deserialize, Serialize};

use crate::envelope::JobId;

/// One credit to one worker for one job.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct MintReceipt {
    /// 32-byte ed25519 worker pubkey.
    pub recipient: [u8; 32],
    pub amount: u64,
    pub job_id: JobId,
}

/// Anything that can ingest mint receipts. Implemented in `nous-payments`.
pub trait MintSink: Send + Sync {
    fn mint(&mut self, receipt: &MintReceipt) -> Result<(), MintError>;
}

#[derive(Debug, thiserror::Error)]
pub enum MintError {
    #[error("ledger error: {0}")]
    Ledger(String),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mint_receipt_serde_round_trip() {
        let m = MintReceipt {
            recipient: [9u8; 32],
            amount: 1234,
            job_id: JobId([5u8; 32]),
        };
        let json = serde_json::to_string(&m).unwrap();
        let back: MintReceipt = serde_json::from_str(&json).unwrap();
        assert_eq!(m, back);
    }

    #[test]
    fn mint_receipt_equality() {
        let a = MintReceipt {
            recipient: [1u8; 32],
            amount: 1,
            job_id: JobId([0u8; 32]),
        };
        let b = a;
        assert_eq!(a, b);
    }

    struct CountingSink(u64);
    impl MintSink for CountingSink {
        fn mint(&mut self, r: &MintReceipt) -> Result<(), MintError> {
            self.0 += r.amount;
            Ok(())
        }
    }

    #[test]
    fn mint_sink_aggregates() {
        let mut s = CountingSink(0);
        let m = MintReceipt {
            recipient: [0; 32],
            amount: 7,
            job_id: JobId([0; 32]),
        };
        s.mint(&m).unwrap();
        s.mint(&m).unwrap();
        assert_eq!(s.0, 14);
    }
}
