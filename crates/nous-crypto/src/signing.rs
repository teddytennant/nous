use ed25519_dalek::{Signer as DalekSigner, Verifier as DalekVerifier};
use serde::{Deserialize, Serialize};

use nous_core::{Error, Result};

use crate::keys::KeyPair;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Signature(pub Vec<u8>);

impl Signature {
    pub fn as_bytes(&self) -> &[u8] {
        &self.0
    }
}

pub struct Signer<'a> {
    keypair: &'a KeyPair,
}

impl<'a> Signer<'a> {
    pub fn new(keypair: &'a KeyPair) -> Self {
        Self { keypair }
    }

    pub fn sign(&self, message: &[u8]) -> Signature {
        let sig = self.keypair.signing_key().sign(message);
        Signature(sig.to_bytes().to_vec())
    }

    pub fn sign_json<T: Serialize>(&self, value: &T) -> Result<Signature> {
        let bytes = serde_json::to_vec(value)?;
        Ok(self.sign(&bytes))
    }
}

pub struct Verifier;

impl Verifier {
    pub fn verify(
        public_key: &ed25519_dalek::VerifyingKey,
        message: &[u8],
        signature: &Signature,
    ) -> Result<()> {
        let sig_bytes: [u8; 64] = signature
            .as_bytes()
            .try_into()
            .map_err(|_| Error::Crypto("signature must be 64 bytes".into()))?;

        let sig = ed25519_dalek::Signature::from_bytes(&sig_bytes);

        public_key
            .verify(message, &sig)
            .map_err(|e| Error::Crypto(format!("signature verification failed: {e}")))
    }

    pub fn verify_json<T: Serialize>(
        public_key: &ed25519_dalek::VerifyingKey,
        value: &T,
        signature: &Signature,
    ) -> Result<()> {
        let bytes = serde_json::to_vec(value)?;
        Self::verify(public_key, &bytes, signature)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sign_and_verify() {
        let kp = KeyPair::generate();
        let signer = Signer::new(&kp);

        let message = b"the overman creates his own values";
        let sig = signer.sign(message);

        assert!(Verifier::verify(&kp.verifying_key(), message, &sig).is_ok());
    }

    #[test]
    fn verify_rejects_wrong_message() {
        let kp = KeyPair::generate();
        let signer = Signer::new(&kp);

        let sig = signer.sign(b"original message");
        assert!(Verifier::verify(&kp.verifying_key(), b"tampered message", &sig).is_err());
    }

    #[test]
    fn verify_rejects_wrong_key() {
        let kp1 = KeyPair::generate();
        let kp2 = KeyPair::generate();
        let signer = Signer::new(&kp1);

        let message = b"signed by kp1";
        let sig = signer.sign(message);

        assert!(Verifier::verify(&kp2.verifying_key(), message, &sig).is_err());
    }

    #[test]
    fn sign_and_verify_json() {
        let kp = KeyPair::generate();
        let signer = Signer::new(&kp);

        let data = serde_json::json!({"action": "transfer", "amount": 100});
        let sig = signer.sign_json(&data).unwrap();

        assert!(Verifier::verify_json(&kp.verifying_key(), &data, &sig).is_ok());
    }

    #[test]
    fn signature_deterministic() {
        let kp = KeyPair::generate();
        let signer = Signer::new(&kp);
        let message = b"determinism test";

        let sig1 = signer.sign(message);
        let sig2 = signer.sign(message);

        // ed25519 is deterministic
        assert_eq!(sig1, sig2);
    }

    #[test]
    fn signature_is_64_bytes() {
        let kp = KeyPair::generate();
        let signer = Signer::new(&kp);
        let sig = signer.sign(b"test");
        assert_eq!(sig.as_bytes().len(), 64);
    }

    #[test]
    fn verify_rejects_truncated_signature() {
        let kp = KeyPair::generate();
        let bad_sig = Signature(vec![0u8; 32]); // too short
        assert!(Verifier::verify(&kp.verifying_key(), b"test", &bad_sig).is_err());
    }
}
