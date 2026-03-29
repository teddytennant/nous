//! Double Ratchet Algorithm.
//!
//! Provides forward secrecy and break-in recovery for encrypted messaging.
//! Each message uses a unique key derived through a ratcheting KDF chain.

use std::collections::HashMap;

use hkdf::Hkdf;
use hmac::{Hmac, Mac};
use rand::rngs::OsRng;
use serde::{Deserialize, Serialize};
use sha2::Sha256;
use x25519_dalek::{PublicKey as X25519PublicKey, StaticSecret};
use zeroize::Zeroize;

use nous_core::{Error, Result};
use nous_crypto::encryption::{self, EncryptedPayload};

const MAX_SKIP: u32 = 256;

/// Header attached to each ratchet-encrypted message.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RatchetHeader {
    pub dh_public: [u8; 32],
    pub message_num: u32,
    pub prev_chain_len: u32,
}

/// A complete ratchet message: header + encrypted payload.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RatchetMessage {
    pub header: RatchetHeader,
    pub payload: EncryptedPayload,
}

/// Double Ratchet state machine for one side of a conversation.
pub struct DoubleRatchet {
    root_key: [u8; 32],
    dh_self: StaticSecret,
    dh_self_pub: X25519PublicKey,
    dh_remote: Option<X25519PublicKey>,
    chain_send: Option<[u8; 32]>,
    chain_recv: Option<[u8; 32]>,
    send_count: u32,
    recv_count: u32,
    prev_send_count: u32,
    skipped: HashMap<([u8; 32], u32), [u8; 32]>,
}

impl DoubleRatchet {
    /// Initialize as the session initiator (after X3DH).
    ///
    /// The peer's signed pre-key serves as the initial remote DH key.
    pub fn init_initiator(shared_secret: [u8; 32], peer_spk: &[u8; 32]) -> Self {
        let dh_self = StaticSecret::random_from_rng(OsRng);
        let dh_self_pub = X25519PublicKey::from(&dh_self);
        let remote = X25519PublicKey::from(*peer_spk);

        let dh_out = dh_self.diffie_hellman(&remote);
        let (root_key, chain_send) = kdf_root(&shared_secret, dh_out.as_bytes());

        Self {
            root_key,
            dh_self,
            dh_self_pub,
            dh_remote: Some(remote),
            chain_send: Some(chain_send),
            chain_recv: None,
            send_count: 0,
            recv_count: 0,
            prev_send_count: 0,
            skipped: HashMap::new(),
        }
    }

    /// Initialize as the session responder (after X3DH).
    ///
    /// Uses the signed pre-key secret as the initial DH ratchet key.
    pub fn init_responder(shared_secret: [u8; 32], spk_secret: StaticSecret) -> Self {
        let dh_self_pub = X25519PublicKey::from(&spk_secret);

        Self {
            root_key: shared_secret,
            dh_self: spk_secret,
            dh_self_pub,
            dh_remote: None,
            chain_send: None,
            chain_recv: None,
            send_count: 0,
            recv_count: 0,
            prev_send_count: 0,
            skipped: HashMap::new(),
        }
    }

    /// Encrypt a plaintext message, advancing the sending chain.
    pub fn encrypt(&mut self, plaintext: &[u8]) -> Result<RatchetMessage> {
        let ck = self
            .chain_send
            .ok_or_else(|| Error::Crypto("no sending chain".into()))?;

        let (new_ck, mut mk) = kdf_chain(&ck);
        self.chain_send = Some(new_ck);

        let header = RatchetHeader {
            dh_public: self.dh_self_pub.to_bytes(),
            message_num: self.send_count,
            prev_chain_len: self.prev_send_count,
        };

        let payload = encryption::encrypt(&mk, plaintext)?;
        mk.zeroize();
        self.send_count += 1;

        Ok(RatchetMessage { header, payload })
    }

    /// Decrypt a ratchet message, performing DH ratchet steps as needed.
    pub fn decrypt(&mut self, msg: &RatchetMessage) -> Result<Vec<u8>> {
        let pub_bytes = msg.header.dh_public;
        if let Some(mut mk) = self.skipped.remove(&(pub_bytes, msg.header.message_num)) {
            let result = encryption::decrypt(&mk, &msg.payload);
            mk.zeroize();
            return result;
        }

        let remote = X25519PublicKey::from(msg.header.dh_public);
        let need_ratchet = match &self.dh_remote {
            None => true,
            Some(current) => current.as_bytes() != remote.as_bytes(),
        };

        if need_ratchet {
            if self.chain_recv.is_some() {
                self.store_skipped_keys(msg.header.prev_chain_len)?;
            }
            self.dh_ratchet(&remote)?;
        }

        self.store_skipped_keys(msg.header.message_num)?;

        let ck = self
            .chain_recv
            .ok_or_else(|| Error::Crypto("no receiving chain".into()))?;
        let (new_ck, mut mk) = kdf_chain(&ck);
        self.chain_recv = Some(new_ck);
        self.recv_count += 1;

        let result = encryption::decrypt(&mk, &msg.payload);
        mk.zeroize();
        result
    }

    fn dh_ratchet(&mut self, remote: &X25519PublicKey) -> Result<()> {
        self.prev_send_count = self.send_count;
        self.send_count = 0;
        self.recv_count = 0;
        self.dh_remote = Some(*remote);

        let dh_recv = self.dh_self.diffie_hellman(remote);
        let (rk, chain_recv) = kdf_root(&self.root_key, dh_recv.as_bytes());
        self.root_key = rk;
        self.chain_recv = Some(chain_recv);

        self.dh_self = StaticSecret::random_from_rng(OsRng);
        self.dh_self_pub = X25519PublicKey::from(&self.dh_self);

        let dh_send = self.dh_self.diffie_hellman(remote);
        let (rk, chain_send) = kdf_root(&self.root_key, dh_send.as_bytes());
        self.root_key = rk;
        self.chain_send = Some(chain_send);

        Ok(())
    }

    fn store_skipped_keys(&mut self, until: u32) -> Result<()> {
        if until > self.recv_count + MAX_SKIP {
            return Err(Error::Crypto(format!(
                "skip limit exceeded: {} > {MAX_SKIP}",
                until - self.recv_count,
            )));
        }

        if let Some(mut ck) = self.chain_recv {
            let pub_bytes = self
                .dh_remote
                .map(|pk| pk.to_bytes())
                .unwrap_or([0u8; 32]);

            while self.recv_count < until {
                let (new_ck, mk) = kdf_chain(&ck);
                self.skipped.insert((pub_bytes, self.recv_count), mk);
                ck = new_ck;
                self.recv_count += 1;
            }
            self.chain_recv = Some(ck);
        }

        Ok(())
    }

    /// Number of stored skipped message keys.
    pub fn skipped_count(&self) -> usize {
        self.skipped.len()
    }

    /// Current sending ratchet public key.
    pub fn public_key(&self) -> [u8; 32] {
        self.dh_self_pub.to_bytes()
    }
}

impl Drop for DoubleRatchet {
    fn drop(&mut self) {
        self.root_key.zeroize();
        if let Some(ref mut k) = self.chain_send {
            k.zeroize();
        }
        if let Some(ref mut k) = self.chain_recv {
            k.zeroize();
        }
        for v in self.skipped.values_mut() {
            v.zeroize();
        }
    }
}

fn kdf_root(root_key: &[u8; 32], dh_output: &[u8; 32]) -> ([u8; 32], [u8; 32]) {
    let hk = Hkdf::<Sha256>::new(Some(root_key), dh_output);
    let mut out = [0u8; 64];
    hk.expand(b"nous-ratchet-root-v1", &mut out)
        .expect("HKDF expand to 64 bytes succeeds");

    let mut rk = [0u8; 32];
    let mut ck = [0u8; 32];
    rk.copy_from_slice(&out[..32]);
    ck.copy_from_slice(&out[32..]);
    out.zeroize();
    (rk, ck)
}

fn kdf_chain(chain_key: &[u8; 32]) -> ([u8; 32], [u8; 32]) {
    let mut mac_ck =
        Hmac::<Sha256>::new_from_slice(chain_key).expect("HMAC accepts any key length");
    mac_ck.update(&[0x01]);
    let ck_out = mac_ck.finalize().into_bytes();
    let mut new_ck = [0u8; 32];
    new_ck.copy_from_slice(&ck_out);

    let mut mac_mk =
        Hmac::<Sha256>::new_from_slice(chain_key).expect("HMAC accepts any key length");
    mac_mk.update(&[0x02]);
    let mk_out = mac_mk.finalize().into_bytes();
    let mut mk = [0u8; 32];
    mk.copy_from_slice(&mk_out);

    (new_ck, mk)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_pair() -> (DoubleRatchet, DoubleRatchet) {
        let shared = [42u8; 32];
        let bob_spk = StaticSecret::random_from_rng(OsRng);
        let bob_spk_pub = X25519PublicKey::from(&bob_spk).to_bytes();

        let alice = DoubleRatchet::init_initiator(shared, &bob_spk_pub);
        let bob = DoubleRatchet::init_responder(shared, bob_spk);
        (alice, bob)
    }

    #[test]
    fn single_message() {
        let (mut alice, mut bob) = create_pair();
        let msg = alice.encrypt(b"hello").unwrap();
        assert_eq!(bob.decrypt(&msg).unwrap(), b"hello");
    }

    #[test]
    fn multiple_messages_one_direction() {
        let (mut alice, mut bob) = create_pair();
        for i in 0..10u32 {
            let text = format!("msg {i}");
            let msg = alice.encrypt(text.as_bytes()).unwrap();
            assert_eq!(bob.decrypt(&msg).unwrap(), text.as_bytes());
        }
    }

    #[test]
    fn bidirectional_conversation() {
        let (mut alice, mut bob) = create_pair();

        let m1 = alice.encrypt(b"alice 1").unwrap();
        assert_eq!(bob.decrypt(&m1).unwrap(), b"alice 1");

        let m2 = bob.encrypt(b"bob 1").unwrap();
        assert_eq!(alice.decrypt(&m2).unwrap(), b"bob 1");

        let m3 = alice.encrypt(b"alice 2").unwrap();
        assert_eq!(bob.decrypt(&m3).unwrap(), b"alice 2");

        let m4 = bob.encrypt(b"bob 2").unwrap();
        assert_eq!(alice.decrypt(&m4).unwrap(), b"bob 2");
    }

    #[test]
    fn out_of_order_delivery() {
        let (mut alice, mut bob) = create_pair();

        let m0 = alice.encrypt(b"first").unwrap();
        let m1 = alice.encrypt(b"second").unwrap();
        let m2 = alice.encrypt(b"third").unwrap();

        assert_eq!(bob.decrypt(&m0).unwrap(), b"first");
        assert_eq!(bob.decrypt(&m2).unwrap(), b"third");
        assert_eq!(bob.skipped_count(), 1);
        assert_eq!(bob.decrypt(&m1).unwrap(), b"second");
        assert_eq!(bob.skipped_count(), 0);
    }

    #[test]
    fn ratchet_advances_public_key() {
        let (mut alice, mut bob) = create_pair();
        let pk1 = alice.public_key();

        let m = alice.encrypt(b"msg").unwrap();
        bob.decrypt(&m).unwrap();

        let reply = bob.encrypt(b"reply").unwrap();
        alice.decrypt(&reply).unwrap();

        assert_ne!(pk1, alice.public_key());
    }

    #[test]
    fn forward_secrecy_via_ratchet() {
        let (mut alice, mut bob) = create_pair();

        let m1 = alice.encrypt(b"epoch 1").unwrap();
        assert_eq!(bob.decrypt(&m1).unwrap(), b"epoch 1");

        let m2 = bob.encrypt(b"reply").unwrap();
        assert_eq!(alice.decrypt(&m2).unwrap(), b"reply");

        let m3 = alice.encrypt(b"epoch 2").unwrap();
        assert_eq!(bob.decrypt(&m3).unwrap(), b"epoch 2");
    }

    #[test]
    fn message_serializes() {
        let (mut alice, _) = create_pair();
        let msg = alice.encrypt(b"serde").unwrap();

        let json = serde_json::to_string(&msg).unwrap();
        let restored: RatchetMessage = serde_json::from_str(&json).unwrap();

        assert_eq!(restored.header.message_num, msg.header.message_num);
        assert_eq!(restored.header.dh_public, msg.header.dh_public);
    }

    #[test]
    fn wrong_shared_secret_fails() {
        let spk = StaticSecret::random_from_rng(OsRng);
        let spk_pub = X25519PublicKey::from(&spk).to_bytes();

        let mut alice = DoubleRatchet::init_initiator([1u8; 32], &spk_pub);
        let mut eve = DoubleRatchet::init_responder([2u8; 32], StaticSecret::random_from_rng(OsRng));

        let msg = alice.encrypt(b"secret").unwrap();
        assert!(eve.decrypt(&msg).is_err());
    }

    #[test]
    fn empty_plaintext() {
        let (mut alice, mut bob) = create_pair();
        let msg = alice.encrypt(b"").unwrap();
        assert!(bob.decrypt(&msg).unwrap().is_empty());
    }

    #[test]
    fn large_plaintext() {
        let (mut alice, mut bob) = create_pair();
        let data = vec![0xAB; 100_000];
        let msg = alice.encrypt(&data).unwrap();
        assert_eq!(bob.decrypt(&msg).unwrap(), data);
    }

    #[test]
    fn long_conversation() {
        let (mut alice, mut bob) = create_pair();
        for i in 0..50u32 {
            let t1 = format!("a{i}");
            let m = alice.encrypt(t1.as_bytes()).unwrap();
            assert_eq!(bob.decrypt(&m).unwrap(), t1.as_bytes());

            let t2 = format!("b{i}");
            let m = bob.encrypt(t2.as_bytes()).unwrap();
            assert_eq!(alice.decrypt(&m).unwrap(), t2.as_bytes());
        }
    }

    #[test]
    fn replay_fails() {
        let (mut alice, mut bob) = create_pair();
        let msg = alice.encrypt(b"once").unwrap();
        assert_eq!(bob.decrypt(&msg).unwrap(), b"once");
        assert!(bob.decrypt(&msg).is_err());
    }

    #[test]
    fn responder_cannot_send_before_receiving() {
        let shared = [0u8; 32];
        let spk = StaticSecret::random_from_rng(OsRng);
        let mut bob = DoubleRatchet::init_responder(shared, spk);
        assert!(bob.encrypt(b"premature").is_err());
    }
}
