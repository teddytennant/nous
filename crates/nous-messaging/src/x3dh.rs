//! X3DH (Extended Triple Diffie-Hellman) Key Agreement Protocol.
//!
//! Establishes a shared secret between two parties where the responder
//! may be offline, using pre-published key bundles.

use ed25519_dalek::VerifyingKey;
use hkdf::Hkdf;
use rand::rngs::OsRng;
use serde::{Deserialize, Serialize};
use sha2::Sha256;
use x25519_dalek::{PublicKey as X25519PublicKey, StaticSecret};
use zeroize::Zeroize;

use nous_core::{Error, Result};
use nous_crypto::keys::KeyPair;
use nous_crypto::signing::{Signature, Signer, Verifier};

/// Signed pre-key: medium-term x25519 key signed by the identity's ed25519 key.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SignedPreKey {
    pub key: [u8; 32],
    pub signature: Signature,
}

/// One-time pre-key: single-use x25519 key for additional forward secrecy.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OneTimePreKey {
    pub id: u32,
    pub key: [u8; 32],
}

/// Pre-key bundle published by a user for asynchronous session initiation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PreKeyBundle {
    pub identity_key: [u8; 32],
    pub signing_key: [u8; 32],
    pub signed_pre_key: SignedPreKey,
    pub one_time_pre_key: Option<OneTimePreKey>,
}

impl PreKeyBundle {
    /// Create a pre-key bundle from an identity's keypair.
    pub fn create(
        keypair: &KeyPair,
        spk_secret: &StaticSecret,
        opk: Option<(u32, &StaticSecret)>,
    ) -> Self {
        let spk_pub = X25519PublicKey::from(spk_secret);
        let signer = Signer::new(keypair);
        let spk_sig = signer.sign(spk_pub.as_bytes());

        let opk_entry = opk.map(|(id, secret)| OneTimePreKey {
            id,
            key: X25519PublicKey::from(secret).to_bytes(),
        });

        Self {
            identity_key: keypair.exchange_public_bytes(),
            signing_key: keypair.signing_public_bytes(),
            signed_pre_key: SignedPreKey {
                key: spk_pub.to_bytes(),
                signature: spk_sig,
            },
            one_time_pre_key: opk_entry,
        }
    }

    /// Verify the signed pre-key signature against the bundle's signing key.
    pub fn verify(&self) -> Result<()> {
        let vk = VerifyingKey::from_bytes(&self.signing_key)
            .map_err(|e| Error::Crypto(format!("invalid signing key: {e}")))?;
        Verifier::verify(
            &vk,
            &self.signed_pre_key.key,
            &self.signed_pre_key.signature,
        )
    }
}

/// Output of the X3DH initiator side.
pub struct X3dhOutput {
    pub shared_secret: [u8; 32],
    pub ephemeral_key: [u8; 32],
    pub one_time_pre_key_id: Option<u32>,
    pub identity_key: [u8; 32],
}

/// Perform X3DH as the initiator.
///
/// Computes DH1 = DH(IK_a, SPK_b), DH2 = DH(EK_a, IK_b),
/// DH3 = DH(EK_a, SPK_b), DH4 = DH(EK_a, OPK_b) if available.
/// SK = KDF(DH1 || DH2 || DH3 || DH4)
pub fn initiate(our_keypair: &KeyPair, peer_bundle: &PreKeyBundle) -> Result<X3dhOutput> {
    peer_bundle.verify()?;

    let ek = StaticSecret::random_from_rng(OsRng);
    let ek_pub = X25519PublicKey::from(&ek);

    let peer_ik = X25519PublicKey::from(peer_bundle.identity_key);
    let peer_spk = X25519PublicKey::from(peer_bundle.signed_pre_key.key);

    let dh1 = our_keypair.exchange.diffie_hellman(&peer_spk);
    let dh2 = ek.diffie_hellman(&peer_ik);
    let dh3 = ek.diffie_hellman(&peer_spk);

    let mut ikm = Vec::with_capacity(128);
    ikm.extend_from_slice(dh1.as_bytes());
    ikm.extend_from_slice(dh2.as_bytes());
    ikm.extend_from_slice(dh3.as_bytes());

    let opk_id = if let Some(ref opk) = peer_bundle.one_time_pre_key {
        let dh4 = ek.diffie_hellman(&X25519PublicKey::from(opk.key));
        ikm.extend_from_slice(dh4.as_bytes());
        Some(opk.id)
    } else {
        None
    };

    let shared_secret = kdf_x3dh(&ikm);
    ikm.zeroize();

    Ok(X3dhOutput {
        shared_secret,
        ephemeral_key: ek_pub.to_bytes(),
        one_time_pre_key_id: opk_id,
        identity_key: our_keypair.exchange_public_bytes(),
    })
}

/// Accept X3DH as the responder.
///
/// Derives the same shared secret from the initiator's identity and ephemeral keys.
pub fn accept(
    our_keypair: &KeyPair,
    spk_secret: &StaticSecret,
    opk_secret: Option<&StaticSecret>,
    initiator_ik: &[u8; 32],
    initiator_ek: &[u8; 32],
) -> Result<[u8; 32]> {
    let peer_ik = X25519PublicKey::from(*initiator_ik);
    let peer_ek = X25519PublicKey::from(*initiator_ek);

    let dh1 = spk_secret.diffie_hellman(&peer_ik);
    let dh2 = our_keypair.exchange.diffie_hellman(&peer_ek);
    let dh3 = spk_secret.diffie_hellman(&peer_ek);

    let mut ikm = Vec::with_capacity(128);
    ikm.extend_from_slice(dh1.as_bytes());
    ikm.extend_from_slice(dh2.as_bytes());
    ikm.extend_from_slice(dh3.as_bytes());

    if let Some(opk) = opk_secret {
        let dh4 = opk.diffie_hellman(&peer_ek);
        ikm.extend_from_slice(dh4.as_bytes());
    }

    let shared_secret = kdf_x3dh(&ikm);
    ikm.zeroize();

    Ok(shared_secret)
}

fn kdf_x3dh(ikm: &[u8]) -> [u8; 32] {
    let mut padded = vec![0xFF; 32];
    padded.extend_from_slice(ikm);

    let hk = Hkdf::<Sha256>::new(Some(b"nous-x3dh-v1"), &padded);
    let mut out = [0u8; 32];
    hk.expand(b"x3dh-shared-secret", &mut out)
        .expect("HKDF expand to 32 bytes always succeeds");

    padded.zeroize();
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use nous_identity::Identity;

    fn setup_bob() -> (Identity, StaticSecret, StaticSecret) {
        let bob = Identity::generate();
        let spk = StaticSecret::random_from_rng(OsRng);
        let opk = StaticSecret::random_from_rng(OsRng);
        (bob, spk, opk)
    }

    #[test]
    fn prekey_bundle_verifies() {
        let (bob, spk, opk) = setup_bob();
        let bundle = PreKeyBundle::create(bob.keypair(), &spk, Some((1, &opk)));
        assert!(bundle.verify().is_ok());
    }

    #[test]
    fn prekey_bundle_rejects_tampered_signature() {
        let (bob, spk, _) = setup_bob();
        let mut bundle = PreKeyBundle::create(bob.keypair(), &spk, None);
        bundle.signed_pre_key.key[0] ^= 0xFF;
        assert!(bundle.verify().is_err());
    }

    #[test]
    fn x3dh_shared_secret_matches_with_opk() {
        let alice = Identity::generate();
        let (bob, spk, opk) = setup_bob();
        let bundle = PreKeyBundle::create(bob.keypair(), &spk, Some((1, &opk)));

        let alice_out = initiate(alice.keypair(), &bundle).unwrap();
        let bob_secret = accept(
            bob.keypair(),
            &spk,
            Some(&opk),
            &alice_out.identity_key,
            &alice_out.ephemeral_key,
        )
        .unwrap();

        assert_eq!(alice_out.shared_secret, bob_secret);
        assert_eq!(alice_out.one_time_pre_key_id, Some(1));
    }

    #[test]
    fn x3dh_shared_secret_matches_without_opk() {
        let alice = Identity::generate();
        let (bob, spk, _) = setup_bob();
        let bundle = PreKeyBundle::create(bob.keypair(), &spk, None);

        let alice_out = initiate(alice.keypair(), &bundle).unwrap();
        let bob_secret = accept(
            bob.keypair(),
            &spk,
            None,
            &alice_out.identity_key,
            &alice_out.ephemeral_key,
        )
        .unwrap();

        assert_eq!(alice_out.shared_secret, bob_secret);
        assert_eq!(alice_out.one_time_pre_key_id, None);
    }

    #[test]
    fn x3dh_different_initiators_different_secrets() {
        let alice1 = Identity::generate();
        let alice2 = Identity::generate();
        let (bob, spk, _) = setup_bob();
        let bundle = PreKeyBundle::create(bob.keypair(), &spk, None);

        let out1 = initiate(alice1.keypair(), &bundle).unwrap();
        let out2 = initiate(alice2.keypair(), &bundle).unwrap();

        assert_ne!(out1.shared_secret, out2.shared_secret);
    }

    #[test]
    fn x3dh_ephemeral_key_unique_per_session() {
        let alice = Identity::generate();
        let (bob, spk, _) = setup_bob();
        let bundle = PreKeyBundle::create(bob.keypair(), &spk, None);

        let out1 = initiate(alice.keypair(), &bundle).unwrap();
        let out2 = initiate(alice.keypair(), &bundle).unwrap();

        assert_ne!(out1.ephemeral_key, out2.ephemeral_key);
    }

    #[test]
    fn prekey_bundle_serializes() {
        let (bob, spk, opk) = setup_bob();
        let bundle = PreKeyBundle::create(bob.keypair(), &spk, Some((42, &opk)));

        let json = serde_json::to_string(&bundle).unwrap();
        let restored: PreKeyBundle = serde_json::from_str(&json).unwrap();

        assert_eq!(restored.identity_key, bundle.identity_key);
        assert_eq!(restored.one_time_pre_key.as_ref().unwrap().id, 42);
        assert!(restored.verify().is_ok());
    }

    #[test]
    fn x3dh_wrong_spk_produces_different_secret() {
        let alice = Identity::generate();
        let (bob, spk, _) = setup_bob();
        let bundle = PreKeyBundle::create(bob.keypair(), &spk, None);

        let alice_out = initiate(alice.keypair(), &bundle).unwrap();

        let wrong_spk = StaticSecret::random_from_rng(OsRng);
        let bob_secret = accept(
            bob.keypair(),
            &wrong_spk,
            None,
            &alice_out.identity_key,
            &alice_out.ephemeral_key,
        )
        .unwrap();

        assert_ne!(alice_out.shared_secret, bob_secret);
    }

    #[test]
    fn x3dh_identity_key_in_output() {
        let alice = Identity::generate();
        let (bob, spk, _) = setup_bob();
        let bundle = PreKeyBundle::create(bob.keypair(), &spk, None);

        let out = initiate(alice.keypair(), &bundle).unwrap();
        assert_eq!(out.identity_key, alice.keypair().exchange_public_bytes());
    }
}
