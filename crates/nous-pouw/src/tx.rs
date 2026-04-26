//! On-chain transactions: transfers, staking, validator registration.
//!
//! Every transaction is signed by `from` (or `worker`) over its canonical
//! body. Each (sender, nonce) is consumed at most once — the chain rejects
//! replays via a per-worker monotonic nonce in [`WorkerInfo`].
//!
//! Transactions are gossiped between nodes via the [`Mempool`](crate::mempool::Mempool)
//! and folded into block bodies by leaders during [`Engine::step`](crate::engine::Engine::step).

use ed25519_dalek::{Signer as DalekSigner, SigningKey};
use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::state::WorkerId;

/// One on-chain action.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum TxBody {
    /// Move balance between workers.
    Transfer {
        from: WorkerId,
        to: WorkerId,
        amount: u64,
    },
    /// Lock balance into stake (cannot be transferred until unstaked).
    Stake { worker: WorkerId, amount: u64 },
    /// Release stake back to balance. v0 has no unbonding period.
    Unstake { worker: WorkerId, amount: u64 },
    /// Add the worker to the active validator set (must already have stake).
    RegisterValidator { worker: WorkerId },
}

impl TxBody {
    /// Sender of the tx (the key that must sign it).
    pub fn sender(&self) -> WorkerId {
        match self {
            TxBody::Transfer { from, .. } => *from,
            TxBody::Stake { worker, .. } => *worker,
            TxBody::Unstake { worker, .. } => *worker,
            TxBody::RegisterValidator { worker } => *worker,
        }
    }
}

/// A signed, replay-protected transaction.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Transaction {
    pub body: TxBody,
    /// Per-sender monotonic nonce. The chain accepts a tx iff
    /// `tx.nonce == sender.nonce + 1` (i.e. strictly the next).
    pub nonce: u64,
    /// Optional fee paid to the leader (v0: not enforced, included for forward compat).
    pub fee: u64,
    /// ed25519 signature over the canonical (body, nonce, fee) tuple.
    pub signature: Vec<u8>,
}

#[derive(Debug, Error, PartialEq)]
pub enum TxError {
    #[error("invalid signature")]
    InvalidSignature,
    #[error("nonce mismatch: expected {expected}, got {actual}")]
    NonceMismatch { expected: u64, actual: u64 },
    #[error("insufficient balance: have {have}, need {need}")]
    InsufficientBalance { have: u64, need: u64 },
    #[error("insufficient stake: have {have}, need {need}")]
    InsufficientStake { have: u64, need: u64 },
    #[error("sender not registered as worker")]
    UnknownSender,
    #[error("recipient not registered as worker")]
    UnknownRecipient,
    #[error("self-transfer")]
    SelfTransfer,
    #[error("zero amount")]
    ZeroAmount,
    #[error("invalid sender public key")]
    InvalidKey,
    #[error("validator already registered")]
    AlreadyValidator,
}

impl Transaction {
    /// Build + sign in one step.
    pub fn new_signed(body: TxBody, nonce: u64, fee: u64, sk: &SigningKey) -> Self {
        let mut tx = Transaction {
            body,
            nonce,
            fee,
            signature: vec![],
        };
        tx.sign(sk);
        tx
    }

    /// Canonical bytes that the sender signs.
    pub fn signing_bytes(&self) -> Vec<u8> {
        let unsigned = Transaction {
            signature: vec![],
            ..self.clone()
        };
        serde_json::to_vec(&unsigned).expect("Transaction is JSON-serializable")
    }

    pub fn sign(&mut self, sk: &SigningKey) {
        let bytes = self.signing_bytes();
        let sig = sk.sign(&bytes);
        self.signature = sig.to_bytes().to_vec();
    }

    pub fn verify_signature(&self) -> Result<(), TxError> {
        let sender = self.body.sender();
        let vk =
            ed25519_dalek::VerifyingKey::from_bytes(&sender.0).map_err(|_| TxError::InvalidKey)?;
        let sig_bytes: [u8; 64] = self
            .signature
            .as_slice()
            .try_into()
            .map_err(|_| TxError::InvalidSignature)?;
        let sig = ed25519_dalek::Signature::from_bytes(&sig_bytes);
        ed25519_dalek::Verifier::verify(&vk, &self.signing_bytes(), &sig)
            .map_err(|_| TxError::InvalidSignature)
    }

    /// Stable id for mempool dedup + gossip.
    pub fn id(&self) -> [u8; 32] {
        *blake3::hash(&self.signing_bytes()).as_bytes()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ed25519_dalek::SigningKey;
    use rand::rngs::OsRng;

    fn sk_pair() -> (SigningKey, WorkerId) {
        let sk = SigningKey::generate(&mut OsRng);
        let id = WorkerId::from_verifying_key(&sk.verifying_key());
        (sk, id)
    }

    #[test]
    fn transfer_signs_and_verifies() {
        let (sk, from) = sk_pair();
        let (_, to) = sk_pair();
        let tx = Transaction::new_signed(
            TxBody::Transfer {
                from,
                to,
                amount: 100,
            },
            1,
            0,
            &sk,
        );
        tx.verify_signature().unwrap();
    }

    #[test]
    fn verify_rejects_wrong_signer() {
        let (sk1, _) = sk_pair();
        let (_sk2, from2) = sk_pair();
        let (_, to) = sk_pair();
        // Body claims from2 but signed with sk1.
        let tx = Transaction::new_signed(
            TxBody::Transfer {
                from: from2,
                to,
                amount: 1,
            },
            1,
            0,
            &sk1,
        );
        assert_eq!(tx.verify_signature(), Err(TxError::InvalidSignature));
    }

    #[test]
    fn id_changes_with_nonce() {
        let (sk, w) = sk_pair();
        let a = Transaction::new_signed(
            TxBody::Stake {
                worker: w,
                amount: 1,
            },
            1,
            0,
            &sk,
        );
        let b = Transaction::new_signed(
            TxBody::Stake {
                worker: w,
                amount: 1,
            },
            2,
            0,
            &sk,
        );
        assert_ne!(a.id(), b.id());
    }

    #[test]
    fn id_is_deterministic() {
        let (sk, w) = sk_pair();
        let a = Transaction::new_signed(
            TxBody::Stake {
                worker: w,
                amount: 5,
            },
            1,
            0,
            &sk,
        );
        let b = Transaction::new_signed(
            TxBody::Stake {
                worker: w,
                amount: 5,
            },
            1,
            0,
            &sk,
        );
        assert_eq!(a.id(), b.id());
    }

    #[test]
    fn sender_for_each_variant() {
        let (sk, w) = sk_pair();
        let (_, other) = sk_pair();
        assert_eq!(
            TxBody::Transfer {
                from: w,
                to: other,
                amount: 1,
            }
            .sender(),
            w
        );
        assert_eq!(
            TxBody::Stake {
                worker: w,
                amount: 1
            }
            .sender(),
            w
        );
        assert_eq!(
            TxBody::Unstake {
                worker: w,
                amount: 1
            }
            .sender(),
            w
        );
        assert_eq!(TxBody::RegisterValidator { worker: w }.sender(), w);
        let _ = sk;
    }

    #[test]
    fn signature_changes_with_body() {
        let (sk, from) = sk_pair();
        let (_, to1) = sk_pair();
        let (_, to2) = sk_pair();
        let a = Transaction::new_signed(
            TxBody::Transfer {
                from,
                to: to1,
                amount: 1,
            },
            1,
            0,
            &sk,
        );
        let b = Transaction::new_signed(
            TxBody::Transfer {
                from,
                to: to2,
                amount: 1,
            },
            1,
            0,
            &sk,
        );
        assert_ne!(a.signature, b.signature);
    }

    #[test]
    fn serde_round_trip() {
        let (sk, w) = sk_pair();
        let tx = Transaction::new_signed(
            TxBody::Stake {
                worker: w,
                amount: 5,
            },
            1,
            0,
            &sk,
        );
        let json = serde_json::to_string(&tx).unwrap();
        let back: Transaction = serde_json::from_str(&json).unwrap();
        assert_eq!(tx, back);
        back.verify_signature().unwrap();
    }
}
