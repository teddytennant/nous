use ed25519_dalek::{SigningKey, VerifyingKey};
use rand::rngs::OsRng;
use serde::{Deserialize, Serialize};
use x25519_dalek::{EphemeralSecret, PublicKey as X25519PublicKey, StaticSecret};
use zeroize::Zeroize;

use nous_core::{Error, Result};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PublicKeyBundle {
    pub signing: Vec<u8>,
    pub exchange: Vec<u8>,
}

pub struct KeyPair {
    signing: SigningKey,
    pub exchange: StaticSecret,
    exchange_public: X25519PublicKey,
}

impl KeyPair {
    pub fn generate() -> Self {
        let signing = SigningKey::generate(&mut OsRng);
        let exchange = StaticSecret::random_from_rng(OsRng);
        let exchange_public = X25519PublicKey::from(&exchange);
        Self {
            signing,
            exchange,
            exchange_public,
        }
    }

    pub fn from_signing_bytes(bytes: &[u8]) -> Result<Self> {
        let bytes: [u8; 32] = bytes
            .try_into()
            .map_err(|_| Error::InvalidKey("signing key must be 32 bytes".into()))?;
        let signing = SigningKey::from_bytes(&bytes);
        let exchange = StaticSecret::random_from_rng(OsRng);
        let exchange_public = X25519PublicKey::from(&exchange);
        Ok(Self {
            signing,
            exchange,
            exchange_public,
        })
    }

    pub fn signing_key(&self) -> &SigningKey {
        &self.signing
    }

    pub fn verifying_key(&self) -> VerifyingKey {
        self.signing.verifying_key()
    }

    pub fn signing_public_bytes(&self) -> [u8; 32] {
        self.verifying_key().to_bytes()
    }

    pub fn exchange_public_bytes(&self) -> [u8; 32] {
        self.exchange_public.to_bytes()
    }

    pub fn public_bundle(&self) -> PublicKeyBundle {
        PublicKeyBundle {
            signing: self.signing_public_bytes().to_vec(),
            exchange: self.exchange_public_bytes().to_vec(),
        }
    }

    pub fn signing_secret_bytes(&self) -> [u8; 32] {
        self.signing.to_bytes()
    }
}

impl Drop for KeyPair {
    fn drop(&mut self) {
        let mut bytes = self.signing.to_bytes();
        bytes.zeroize();
    }
}

pub struct SharedSecret {
    bytes: [u8; 32],
}

impl SharedSecret {
    pub fn derive(our_secret: &StaticSecret, their_public: &X25519PublicKey) -> Self {
        let shared = our_secret.diffie_hellman(their_public);
        Self {
            bytes: *shared.as_bytes(),
        }
    }

    pub fn derive_ephemeral(their_public: &X25519PublicKey) -> (Self, X25519PublicKey) {
        let ephemeral = EphemeralSecret::random_from_rng(OsRng);
        let our_public = X25519PublicKey::from(&ephemeral);
        let shared = ephemeral.diffie_hellman(their_public);
        (
            Self {
                bytes: *shared.as_bytes(),
            },
            our_public,
        )
    }

    pub fn as_bytes(&self) -> &[u8; 32] {
        &self.bytes
    }
}

impl Drop for SharedSecret {
    fn drop(&mut self) {
        self.bytes.zeroize();
    }
}

pub fn public_key_to_did(verifying_key: &VerifyingKey) -> String {
    let mut multicodec = vec![0xed, 0x01];
    multicodec.extend_from_slice(&verifying_key.to_bytes());
    format!("did:key:z{}", bs58::encode(&multicodec).into_string())
}

pub fn did_to_public_key(did: &str) -> Result<VerifyingKey> {
    let z_part = did
        .strip_prefix("did:key:z")
        .ok_or_else(|| Error::InvalidKey("invalid DID:key format".into()))?;

    let decoded = bs58::decode(z_part)
        .into_vec()
        .map_err(|e| Error::InvalidKey(format!("base58 decode failed: {e}")))?;

    if decoded.len() < 34 || decoded[0] != 0xed || decoded[1] != 0x01 {
        return Err(Error::InvalidKey(
            "invalid multicodec prefix for ed25519".into(),
        ));
    }

    let key_bytes: [u8; 32] = decoded[2..34]
        .try_into()
        .map_err(|_| Error::InvalidKey("key must be 32 bytes".into()))?;

    VerifyingKey::from_bytes(&key_bytes)
        .map_err(|e| Error::InvalidKey(format!("invalid ed25519 public key: {e}")))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn generate_keypair() {
        let kp = KeyPair::generate();
        assert_eq!(kp.signing_public_bytes().len(), 32);
        assert_eq!(kp.exchange_public_bytes().len(), 32);
    }

    #[test]
    fn keypair_from_bytes_roundtrip() {
        let kp = KeyPair::generate();
        let bytes = kp.signing_secret_bytes();
        let restored = KeyPair::from_signing_bytes(&bytes).unwrap();
        assert_eq!(kp.signing_public_bytes(), restored.signing_public_bytes());
    }

    #[test]
    fn keypair_from_bytes_rejects_wrong_length() {
        assert!(KeyPair::from_signing_bytes(&[0u8; 16]).is_err());
    }

    #[test]
    fn shared_secret_symmetric() {
        let alice = KeyPair::generate();
        let bob = KeyPair::generate();

        let alice_shared = SharedSecret::derive(
            &alice.exchange,
            &X25519PublicKey::from(bob.exchange_public_bytes()),
        );
        let bob_shared = SharedSecret::derive(
            &bob.exchange,
            &X25519PublicKey::from(alice.exchange_public_bytes()),
        );

        assert_eq!(alice_shared.as_bytes(), bob_shared.as_bytes());
    }

    #[test]
    fn ephemeral_shared_secret() {
        let bob = KeyPair::generate();
        let bob_x25519_pub = X25519PublicKey::from(bob.exchange_public_bytes());

        let (alice_shared, alice_ephemeral_pub) = SharedSecret::derive_ephemeral(&bob_x25519_pub);
        let bob_shared = SharedSecret::derive(&bob.exchange, &alice_ephemeral_pub);

        assert_eq!(alice_shared.as_bytes(), bob_shared.as_bytes());
    }

    #[test]
    fn did_key_roundtrip() {
        let kp = KeyPair::generate();
        let did = public_key_to_did(&kp.verifying_key());
        assert!(did.starts_with("did:key:z"));

        let recovered = did_to_public_key(&did).unwrap();
        assert_eq!(recovered.to_bytes(), kp.signing_public_bytes());
    }

    #[test]
    fn did_key_rejects_invalid_prefix() {
        assert!(did_to_public_key("did:web:example.com").is_err());
    }

    #[test]
    fn did_key_rejects_invalid_base58() {
        assert!(did_to_public_key("did:key:z!!!invalid").is_err());
    }

    #[test]
    fn public_bundle_contains_both_keys() {
        let kp = KeyPair::generate();
        let bundle = kp.public_bundle();
        assert_eq!(bundle.signing.len(), 32);
        assert_eq!(bundle.exchange.len(), 32);
    }
}
