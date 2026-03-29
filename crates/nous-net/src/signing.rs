use nous_crypto::keys::{did_to_public_key, KeyPair};
use nous_crypto::signing::{Signature, Signer, Verifier};

use crate::events::WireMessage;

/// Sign a `WireMessage` in place using the given keypair.
///
/// The signature covers `topic || payload || sender_did || timestamp` to prevent
/// replay and topic-swap attacks.
pub fn sign_message(message: &mut WireMessage, keypair: &KeyPair) {
    let signable = message.signable_bytes();
    let signer = Signer::new(keypair);
    let sig = signer.sign(&signable);
    message.signature = sig.as_bytes().to_vec();
}

/// Verify a `WireMessage` signature against the sender's DID.
///
/// Extracts the ed25519 public key from `sender_did` (DID:key method) and
/// verifies the signature over the canonical signable bytes.
pub fn verify_message(message: &WireMessage) -> nous_core::Result<()> {
    if message.signature.is_empty() {
        return Err(nous_core::Error::Crypto("message is unsigned".into()));
    }

    let public_key = did_to_public_key(&message.sender_did)?;
    let sig = Signature(message.signature.clone());
    let signable = message.signable_bytes();
    Verifier::verify(&public_key, &signable, &sig)
}

/// Returns true if the message has a non-empty signature.
pub fn is_signed(message: &WireMessage) -> bool {
    !message.signature.is_empty()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::topics::NousTopic;
    use nous_crypto::keys::public_key_to_did;

    fn make_keypair_and_did() -> (KeyPair, String) {
        let kp = KeyPair::generate();
        let did = public_key_to_did(&kp.verifying_key());
        (kp, did)
    }

    #[test]
    fn sign_and_verify_message() {
        let (kp, did) = make_keypair_and_did();
        let mut msg = WireMessage::new(
            NousTopic::Messages,
            b"hello decentralized world".to_vec(),
            did,
        );

        assert!(!is_signed(&msg));
        sign_message(&mut msg, &kp);
        assert!(is_signed(&msg));
        assert!(verify_message(&msg).is_ok());
    }

    #[test]
    fn verify_rejects_unsigned() {
        let msg = WireMessage::new(
            NousTopic::Messages,
            b"unsigned".to_vec(),
            "did:key:z6MkhaXgBZDvotDkL5257faiztiGiC2QtKLGpbnnEGta2doK".to_string(),
        );

        assert!(verify_message(&msg).is_err());
    }

    #[test]
    fn verify_rejects_tampered_payload() {
        let (kp, did) = make_keypair_and_did();
        let mut msg = WireMessage::new(NousTopic::Social, b"original".to_vec(), did);
        sign_message(&mut msg, &kp);

        msg.payload = b"tampered".to_vec();
        assert!(verify_message(&msg).is_err());
    }

    #[test]
    fn verify_rejects_tampered_topic() {
        let (kp, did) = make_keypair_and_did();
        let mut msg = WireMessage::new(NousTopic::Social, b"data".to_vec(), did);
        sign_message(&mut msg, &kp);

        msg.topic = NousTopic::Governance;
        assert!(verify_message(&msg).is_err());
    }

    #[test]
    fn verify_rejects_tampered_timestamp() {
        let (kp, did) = make_keypair_and_did();
        let mut msg = WireMessage::new(NousTopic::Payments, b"pay".to_vec(), did);
        sign_message(&mut msg, &kp);

        msg.timestamp_ms += 1;
        assert!(verify_message(&msg).is_err());
    }

    #[test]
    fn verify_rejects_wrong_sender_did() {
        let (kp, did) = make_keypair_and_did();
        let (_, other_did) = make_keypair_and_did();
        let mut msg = WireMessage::new(NousTopic::Identity, b"identity".to_vec(), did);
        sign_message(&mut msg, &kp);

        msg.sender_did = other_did;
        assert!(verify_message(&msg).is_err());
    }

    #[test]
    fn verify_rejects_wrong_key_signature() {
        let (_kp1, did1) = make_keypair_and_did();
        let (kp2, _did2) = make_keypair_and_did();
        let mut msg = WireMessage::new(NousTopic::Marketplace, b"listing".to_vec(), did1);

        // Sign with kp2 but sender_did points to kp1.
        sign_message(&mut msg, &kp2);
        assert!(verify_message(&msg).is_err());
    }

    #[test]
    fn signature_is_64_bytes() {
        let (kp, did) = make_keypair_and_did();
        let mut msg = WireMessage::new(NousTopic::Sync, b"sync".to_vec(), did);
        sign_message(&mut msg, &kp);
        assert_eq!(msg.signature.len(), 64);
    }

    #[test]
    fn sign_is_deterministic() {
        let (kp, did) = make_keypair_and_did();
        let mut msg1 = WireMessage {
            topic: NousTopic::Messages,
            payload: b"determinism".to_vec(),
            sender_did: did.clone(),
            timestamp_ms: 1000,
            signature: Vec::new(),
        };
        let mut msg2 = WireMessage {
            topic: NousTopic::Messages,
            payload: b"determinism".to_vec(),
            sender_did: did,
            timestamp_ms: 1000,
            signature: Vec::new(),
        };

        sign_message(&mut msg1, &kp);
        sign_message(&mut msg2, &kp);
        assert_eq!(msg1.signature, msg2.signature);
    }

    #[test]
    fn verify_rejects_invalid_did() {
        let msg = WireMessage {
            topic: NousTopic::Messages,
            payload: b"test".to_vec(),
            sender_did: "not-a-did".to_string(),
            timestamp_ms: 1000,
            signature: vec![0u8; 64],
        };

        assert!(verify_message(&msg).is_err());
    }

    #[test]
    fn verify_rejects_truncated_signature() {
        let (_, did) = make_keypair_and_did();
        let msg = WireMessage {
            topic: NousTopic::Messages,
            payload: b"test".to_vec(),
            sender_did: did,
            timestamp_ms: 1000,
            signature: vec![0u8; 32], // too short
        };

        assert!(verify_message(&msg).is_err());
    }
}
