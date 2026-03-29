use nous_core::Result;
use nous_crypto::encryption::{self, EncryptedPayload};
use nous_crypto::keys::{KeyPair, SharedSecret};
use x25519_dalek::PublicKey as X25519PublicKey;

pub struct Session {
    shared_key: [u8; 32],
    our_did: String,
    peer_did: String,
}

impl Session {
    pub fn establish(
        our_keypair: &KeyPair,
        our_did: &str,
        peer_exchange_pub: &[u8; 32],
        peer_did: &str,
    ) -> Self {
        let peer_pub = X25519PublicKey::from(*peer_exchange_pub);
        let shared = SharedSecret::derive(&our_keypair.exchange, &peer_pub);
        let shared_key = encryption::derive_key(shared.as_bytes(), b"nous-messaging-session-v1");

        Self {
            shared_key,
            our_did: our_did.to_string(),
            peer_did: peer_did.to_string(),
        }
    }

    pub fn encrypt(&self, plaintext: &[u8]) -> Result<EncryptedPayload> {
        encryption::encrypt(&self.shared_key, plaintext)
    }

    pub fn decrypt(&self, payload: &EncryptedPayload) -> Result<Vec<u8>> {
        encryption::decrypt(&self.shared_key, payload)
    }

    pub fn our_did(&self) -> &str {
        &self.our_did
    }

    pub fn peer_did(&self) -> &str {
        &self.peer_did
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use nous_identity::Identity;

    fn create_session_pair() -> (Session, Session) {
        let alice = Identity::generate();
        let bob = Identity::generate();

        let alice_session = Session::establish(
            alice.keypair(),
            alice.did(),
            &bob.keypair().exchange_public_bytes(),
            bob.did(),
        );

        let bob_session = Session::establish(
            bob.keypair(),
            bob.did(),
            &alice.keypair().exchange_public_bytes(),
            alice.did(),
        );

        (alice_session, bob_session)
    }

    #[test]
    fn session_encrypt_decrypt() {
        let (alice, bob) = create_session_pair();

        let plaintext = b"the eternal return";
        let encrypted = alice.encrypt(plaintext).unwrap();
        let decrypted = bob.decrypt(&encrypted).unwrap();

        assert_eq!(decrypted, plaintext);
    }

    #[test]
    fn session_bidirectional() {
        let (alice, bob) = create_session_pair();

        let msg1 = alice.encrypt(b"from alice").unwrap();
        let msg2 = bob.encrypt(b"from bob").unwrap();

        assert_eq!(bob.decrypt(&msg1).unwrap(), b"from alice");
        assert_eq!(alice.decrypt(&msg2).unwrap(), b"from bob");
    }

    #[test]
    fn session_tracks_dids() {
        let alice = Identity::generate();
        let bob = Identity::generate();

        let session = Session::establish(
            alice.keypair(),
            alice.did(),
            &bob.keypair().exchange_public_bytes(),
            bob.did(),
        );

        assert_eq!(session.our_did(), alice.did());
        assert_eq!(session.peer_did(), bob.did());
    }

    #[test]
    fn different_sessions_different_keys() {
        let alice = Identity::generate();
        let bob = Identity::generate();
        let charlie = Identity::generate();

        let session_ab = Session::establish(
            alice.keypair(),
            alice.did(),
            &bob.keypair().exchange_public_bytes(),
            bob.did(),
        );

        let session_ac = Session::establish(
            alice.keypair(),
            alice.did(),
            &charlie.keypair().exchange_public_bytes(),
            charlie.did(),
        );

        let encrypted = session_ab.encrypt(b"for bob only").unwrap();
        assert!(session_ac.decrypt(&encrypted).is_err());
    }
}
