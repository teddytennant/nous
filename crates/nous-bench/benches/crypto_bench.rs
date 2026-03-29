use criterion::{Criterion, black_box, criterion_group, criterion_main};

use nous_crypto::keys::KeyPair;
use nous_crypto::signing::{Signer, Verifier};
use nous_crypto::encryption::{encrypt, decrypt, derive_key};
use nous_crypto::zkp::{SchnorrProof, PedersenCommitment, schnorr_keygen};
use nous_identity::Identity;
use nous_governance::zkvote;
use nous_governance::vote::VoteChoice;

fn bench_keypair_generation(c: &mut Criterion) {
    c.bench_function("keypair_generate", |b| {
        b.iter(|| black_box(KeyPair::generate()));
    });
}

fn bench_identity_generation(c: &mut Criterion) {
    c.bench_function("identity_generate", |b| {
        b.iter(|| black_box(Identity::generate()));
    });
}

fn bench_sign(c: &mut Criterion) {
    let kp = KeyPair::generate();
    let signer = Signer::new(&kp);
    let message = b"benchmark signing operation";

    c.bench_function("ed25519_sign", |b| {
        b.iter(|| black_box(signer.sign(black_box(message))));
    });
}

fn bench_verify(c: &mut Criterion) {
    let kp = KeyPair::generate();
    let signer = Signer::new(&kp);
    let message = b"benchmark verification operation";
    let sig = signer.sign(message);

    c.bench_function("ed25519_verify", |b| {
        b.iter(|| {
            black_box(
                Verifier::verify(black_box(&kp.verifying_key()), black_box(message), black_box(&sig))
            )
        });
    });
}

fn bench_encrypt(c: &mut Criterion) {
    let mut key = [0u8; 32];
    rand::RngCore::fill_bytes(&mut rand::rngs::OsRng, &mut key);

    let mut group = c.benchmark_group("aes256gcm_encrypt");

    for size in [64, 1024, 16384, 262144] {
        let plaintext = vec![0xABu8; size];
        group.bench_with_input(
            criterion::BenchmarkId::from_parameter(size),
            &plaintext,
            |b, data| {
                b.iter(|| black_box(encrypt(black_box(&key), black_box(data)).unwrap()));
            },
        );
    }
    group.finish();
}

fn bench_decrypt(c: &mut Criterion) {
    let mut key = [0u8; 32];
    rand::RngCore::fill_bytes(&mut rand::rngs::OsRng, &mut key);

    let mut group = c.benchmark_group("aes256gcm_decrypt");

    for size in [64, 1024, 16384, 262144] {
        let plaintext = vec![0xABu8; size];
        let encrypted = encrypt(&key, &plaintext).unwrap();
        group.bench_with_input(
            criterion::BenchmarkId::from_parameter(size),
            &encrypted,
            |b, enc| {
                b.iter(|| black_box(decrypt(black_box(&key), black_box(enc)).unwrap()));
            },
        );
    }
    group.finish();
}

fn bench_hkdf_derive(c: &mut Criterion) {
    let mut secret = [0u8; 32];
    rand::RngCore::fill_bytes(&mut rand::rngs::OsRng, &mut secret);

    c.bench_function("hkdf_derive_key", |b| {
        b.iter(|| {
            black_box(derive_key(black_box(&secret), black_box(b"nous-bench-context")))
        });
    });
}

fn bench_schnorr_prove(c: &mut Criterion) {
    let (secret, public) = schnorr_keygen();
    let message = b"benchmark schnorr proof";

    c.bench_function("schnorr_prove", |b| {
        b.iter(|| {
            black_box(SchnorrProof::prove(
                black_box(&secret),
                black_box(&public),
                black_box(message),
            ))
        });
    });
}

fn bench_schnorr_verify(c: &mut Criterion) {
    let (secret, public) = schnorr_keygen();
    let message = b"benchmark schnorr verify";
    let proof = SchnorrProof::prove(&secret, &public, message);

    c.bench_function("schnorr_verify", |b| {
        b.iter(|| {
            black_box(proof.verify(black_box(&public), black_box(message)))
        });
    });
}

fn bench_pedersen_commit(c: &mut Criterion) {
    c.bench_function("pedersen_commit", |b| {
        b.iter(|| black_box(PedersenCommitment::commit(black_box(42))));
    });
}

fn bench_pedersen_verify(c: &mut Criterion) {
    let (commitment, opening) = PedersenCommitment::commit(42);

    c.bench_function("pedersen_verify", |b| {
        b.iter(|| {
            black_box(commitment.verify(black_box(&opening)))
        });
    });
}

fn bench_zk_vote_commit(c: &mut Criterion) {
    c.bench_function("zk_vote_commit", |b| {
        b.iter(|| {
            black_box(
                zkvote::commit_vote(
                    black_box("prop-bench"),
                    black_box("did:key:zBenchVoter"),
                    black_box(VoteChoice::For),
                    black_box(42),
                )
                .unwrap()
            )
        });
    });
}

fn bench_zk_vote_verify(c: &mut Criterion) {
    let (vote, _) = zkvote::commit_vote("prop-bench", "did:key:zBenchVoter", VoteChoice::For, 42).unwrap();

    c.bench_function("zk_vote_verify", |b| {
        b.iter(|| {
            black_box(zkvote::verify_committed_vote(black_box(&vote)).unwrap())
        });
    });
}

fn bench_did_key_generation(c: &mut Criterion) {
    c.bench_function("did_key_from_keypair", |b| {
        b.iter(|| {
            let kp = KeyPair::generate();
            black_box(nous_crypto::keys::public_key_to_did(&kp.verifying_key()))
        });
    });
}

criterion_group!(
    benches,
    bench_keypair_generation,
    bench_identity_generation,
    bench_sign,
    bench_verify,
    bench_encrypt,
    bench_decrypt,
    bench_hkdf_derive,
    bench_schnorr_prove,
    bench_schnorr_verify,
    bench_pedersen_commit,
    bench_pedersen_verify,
    bench_zk_vote_commit,
    bench_zk_vote_verify,
    bench_did_key_generation,
);
criterion_main!(benches);
