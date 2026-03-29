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

// Custom Serialize/Deserialize for KeyPair.
//
// We persist the 32-byte signing secret and 32-byte exchange secret as hex
// strings.  On deserialization we reconstruct the full KeyPair (including
// derived public keys) from these bytes.  The signing key is the ed25519
// secret scalar; the exchange key is the x25519 static secret.
//
// SECURITY NOTE: the serialized form contains private key material.
// Callers must ensure it is stored in an encrypted or access-controlled
// backend (e.g. the SQLite KV store with appropriate file permissions).

impl Serialize for KeyPair {
    fn serialize<S: serde::Serializer>(
        &self,
        serializer: S,
    ) -> std::result::Result<S::Ok, S::Error> {
        use serde::ser::SerializeStruct;
        let mut state = serializer.serialize_struct("KeyPair", 2)?;
        state.serialize_field("signing", &hex::encode(self.signing.to_bytes()))?;
        state.serialize_field("exchange", &hex::encode(self.exchange.to_bytes()))?;
        state.end()
    }
}

impl<'de> Deserialize<'de> for KeyPair {
    fn deserialize<D: serde::Deserializer<'de>>(
        deserializer: D,
    ) -> std::result::Result<Self, D::Error> {
        #[derive(Deserialize)]
        struct Raw {
            signing: String,
            exchange: String,
        }

        let raw = Raw::deserialize(deserializer)?;

        let signing_bytes: [u8; 32] = hex::decode(&raw.signing)
            .map_err(serde::de::Error::custom)?
            .try_into()
            .map_err(|_| serde::de::Error::custom("signing key must be 32 bytes"))?;

        let exchange_bytes: [u8; 32] = hex::decode(&raw.exchange)
            .map_err(serde::de::Error::custom)?
            .try_into()
            .map_err(|_| serde::de::Error::custom("exchange key must be 32 bytes"))?;

        let signing = SigningKey::from_bytes(&signing_bytes);
        let exchange = StaticSecret::from(exchange_bytes);
        let exchange_public = X25519PublicKey::from(&exchange);

        Ok(Self {
            signing,
            exchange,
            exchange_public,
        })
    }
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

    #[test]
    fn keypair_serde_roundtrip() {
        let kp = KeyPair::generate();
        let signing_pub = kp.signing_public_bytes();
        let exchange_pub = kp.exchange_public_bytes();

        let json = serde_json::to_string(&kp).unwrap();
        let restored: KeyPair = serde_json::from_str(&json).unwrap();

        assert_eq!(restored.signing_public_bytes(), signing_pub);
        assert_eq!(restored.exchange_public_bytes(), exchange_pub);
        assert_eq!(restored.signing_secret_bytes(), kp.signing_secret_bytes());
    }

    #[test]
    fn keypair_serde_preserves_signing_capability() {
        use crate::signing::{Signer, Verifier};

        let kp = KeyPair::generate();
        let json = serde_json::to_string(&kp).unwrap();
        let restored: KeyPair = serde_json::from_str(&json).unwrap();

        let message = b"serialization roundtrip signing test";
        let sig = Signer::new(&restored).sign(message);
        assert!(Verifier::verify(&kp.verifying_key(), message, &sig).is_ok());
    }
}
