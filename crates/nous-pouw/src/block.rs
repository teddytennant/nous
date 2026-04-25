//! Block format + sign/verify.

use ed25519_dalek::{Signer as DalekSigner, SigningKey, Verifier};
use serde::{Deserialize, Serialize};

use crate::mint::MintReceipt;
use crate::quorum::QuorumCertificate;
use crate::slashing::SlashEvent;
use crate::state::{StateRoot, WorkerId};

pub type BlockHeight = u64;
pub type BlockHash = [u8; 32];

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BlockHeader {
    pub height: BlockHeight,
    pub prev_hash: BlockHash,
    pub state_root: StateRoot,
    pub body_hash: [u8; 32],
    pub timestamp_ms: u64,
    pub leader: WorkerId,
    pub signature: Vec<u8>,
}

impl BlockHeader {
    /// Canonical bytes that the leader signs.
    pub fn signing_bytes(&self) -> Vec<u8> {
        let unsigned = BlockHeader {
            signature: vec![],
            ..self.clone()
        };
        serde_json::to_vec(&unsigned).expect("BlockHeader is JSON-serializable")
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BlockBody {
    pub certs: Vec<QuorumCertificate>,
    pub slashes: Vec<SlashEvent>,
    pub mints: Vec<MintReceipt>,
}

impl BlockBody {
    pub fn hash(&self) -> [u8; 32] {
        let bytes = serde_json::to_vec(self).expect("BlockBody is JSON-serializable");
        *blake3::hash(&bytes).as_bytes()
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Block {
    pub header: BlockHeader,
    pub body: BlockBody,
}

impl Block {
    pub fn hash(&self) -> BlockHash {
        let bytes = self.header.signing_bytes();
        *blake3::hash(&bytes).as_bytes()
    }
}

/// Sign a header with the leader's signing key (overwrites `signature`).
pub fn sign_block(header: &mut BlockHeader, signing: &SigningKey) {
    let bytes = header.signing_bytes();
    let sig = signing.sign(&bytes);
    header.signature = sig.to_bytes().to_vec();
}

/// Verify a block: leader signature is valid and `body_hash` matches.
pub fn verify_block(block: &Block) -> Result<(), nous_core::Error> {
    if block.header.body_hash != block.body.hash() {
        return Err(nous_core::Error::Crypto("body_hash mismatch".into()));
    }
    let vk = ed25519_dalek::VerifyingKey::from_bytes(&block.header.leader.0)
        .map_err(|e| nous_core::Error::Crypto(format!("bad leader pubkey: {e}")))?;
    let sig_bytes: [u8; 64] = block
        .header
        .signature
        .as_slice()
        .try_into()
        .map_err(|_| nous_core::Error::Crypto("sig must be 64 bytes".into()))?;
    let sig = ed25519_dalek::Signature::from_bytes(&sig_bytes);
    vk.verify(&block.header.signing_bytes(), &sig)
        .map_err(|e| nous_core::Error::Crypto(format!("block sig invalid: {e}")))
}

#[cfg(test)]
mod tests {
    use super::*;
    use ed25519_dalek::SigningKey;
    use rand::rngs::OsRng;

    fn make_block(sk: &SigningKey, height: u64) -> Block {
        let body = BlockBody {
            certs: vec![],
            slashes: vec![],
            mints: vec![],
        };
        let mut header = BlockHeader {
            height,
            prev_hash: [0; 32],
            state_root: [0; 32],
            body_hash: body.hash(),
            timestamp_ms: 0,
            leader: WorkerId::from_verifying_key(&sk.verifying_key()),
            signature: vec![],
        };
        sign_block(&mut header, sk);
        Block { header, body }
    }

    #[test]
    fn sign_verify_round_trip() {
        let sk = SigningKey::generate(&mut OsRng);
        let block = make_block(&sk, 1);
        verify_block(&block).expect("ok");
    }

    #[test]
    fn verify_rejects_tampered_body() {
        let sk = SigningKey::generate(&mut OsRng);
        let mut block = make_block(&sk, 1);
        block.body.mints.push(MintReceipt {
            recipient: [9; 32],
            amount: 1,
            job_id: crate::envelope::JobId([0; 32]),
        });
        // body changed but header.body_hash didn't → verify fails
        assert!(verify_block(&block).is_err());
    }

    #[test]
    fn verify_rejects_tampered_header() {
        let sk = SigningKey::generate(&mut OsRng);
        let mut block = make_block(&sk, 1);
        block.header.height = 999; // signature won't cover this
        assert!(verify_block(&block).is_err());
    }

    #[test]
    fn block_hash_is_deterministic() {
        let sk = SigningKey::generate(&mut OsRng);
        let block = make_block(&sk, 1);
        let h1 = block.hash();
        let h2 = block.hash();
        assert_eq!(h1, h2);
    }

    #[test]
    fn body_hash_changes_with_contents() {
        let body1 = BlockBody {
            certs: vec![],
            slashes: vec![],
            mints: vec![],
        };
        let body2 = BlockBody {
            certs: vec![],
            slashes: vec![],
            mints: vec![MintReceipt {
                recipient: [1; 32],
                amount: 5,
                job_id: crate::envelope::JobId([0; 32]),
            }],
        };
        assert_ne!(body1.hash(), body2.hash());
    }
}
