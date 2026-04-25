//! Equivocation detection: a leader that signs two blocks at the same height
//! is provably faulty; the proof can be verified offline and converted to a
//! SlashEvent.

use ed25519_dalek::SigningKey;
use nous_pouw::block::{BlockBody, BlockHeader, sign_block};
use nous_pouw::slashing::{EquivocationProof, SlashKind, detect_equivocation};
use nous_pouw::state::WorkerId;
use rand::rngs::OsRng;

fn signed_header(sk: &SigningKey, height: u64, state_root: [u8; 32]) -> BlockHeader {
    let body = BlockBody {
        certs: vec![],
        slashes: vec![],
        mints: vec![],
    };
    let mut hdr = BlockHeader {
        height,
        prev_hash: [0; 32],
        state_root,
        body_hash: body.hash(),
        timestamp_ms: height,
        leader: WorkerId::from_verifying_key(&sk.verifying_key()),
        signature: vec![],
    };
    sign_block(&mut hdr, sk);
    hdr
}

#[test]
fn double_sign_at_same_height_yields_slash() {
    let sk = SigningKey::generate(&mut OsRng);
    let a = signed_header(&sk, 5, [1; 32]);
    let b = signed_header(&sk, 5, [2; 32]);

    let proof = EquivocationProof {
        leader: a.leader,
        a,
        b,
    };

    let slash = detect_equivocation(&proof, 1_000).expect("valid equivocation");
    assert_eq!(slash.kind, SlashKind::Equivocation);
    assert_eq!(slash.amount, 1_000);
}

#[test]
fn detection_is_offline_and_replayable() {
    // Anyone with the two headers can independently produce the same slash.
    let sk = SigningKey::generate(&mut OsRng);
    let a = signed_header(&sk, 7, [10; 32]);
    let b = signed_header(&sk, 7, [20; 32]);
    let proof = EquivocationProof {
        leader: a.leader,
        a: a.clone(),
        b: b.clone(),
    };
    let slash1 = detect_equivocation(&proof, 500).unwrap();
    let slash2 = detect_equivocation(&proof, 500).unwrap();
    assert_eq!(slash1, slash2);
}

#[test]
fn forged_proof_is_rejected() {
    // Attacker submits two headers, but flips a bit in one signature.
    let sk = SigningKey::generate(&mut OsRng);
    let a = signed_header(&sk, 9, [3; 32]);
    let mut b = signed_header(&sk, 9, [4; 32]);
    b.signature[0] ^= 0xff;
    let proof = EquivocationProof {
        leader: a.leader,
        a,
        b,
    };
    assert!(detect_equivocation(&proof, 100).is_err());
}
