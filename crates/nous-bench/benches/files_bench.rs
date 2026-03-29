use criterion::{Criterion, black_box, criterion_group, criterion_main};

use nous_files::chunk::{chunk_data, reassemble, ContentId};
use nous_files::store::FileStore;
use nous_files::vault::Vault;
use nous_files::manifest::FileManifest;

fn bench_chunk_small(c: &mut Criterion) {
    let data = vec![0xABu8; 1024]; // 1 KiB

    c.bench_function("chunk_1kib", |b| {
        b.iter(|| black_box(chunk_data(black_box(&data)).unwrap()));
    });
}

fn bench_chunk_medium(c: &mut Criterion) {
    let data = vec![0xABu8; 256 * 1024]; // 256 KiB

    c.bench_function("chunk_256kib", |b| {
        b.iter(|| black_box(chunk_data(black_box(&data)).unwrap()));
    });
}

fn bench_chunk_large(c: &mut Criterion) {
    let data = vec![0xABu8; 1024 * 1024]; // 1 MiB

    c.bench_function("chunk_1mib", |b| {
        b.iter(|| black_box(chunk_data(black_box(&data)).unwrap()));
    });
}

fn bench_reassemble(c: &mut Criterion) {
    let data = vec![0xABu8; 256 * 1024];
    let chunks = chunk_data(&data).unwrap();

    c.bench_function("reassemble_256kib", |b| {
        b.iter(|| black_box(reassemble(black_box(&chunks))));
    });
}

fn bench_content_id(c: &mut Criterion) {
    let mut group = c.benchmark_group("content_id");

    for size in [64, 1024, 65536, 262144] {
        let data = vec![0xABu8; size];
        group.bench_with_input(
            criterion::BenchmarkId::from_parameter(size),
            &data,
            |b, data| {
                b.iter(|| black_box(ContentId::from_bytes(black_box(data))));
            },
        );
    }
    group.finish();
}

fn bench_store_put(c: &mut Criterion) {
    let data = vec![0xABu8; 64 * 1024]; // 64 KiB

    c.bench_function("store_put_64kib", |b| {
        b.iter_with_setup(
            || FileStore::new(),
            |mut store| {
                black_box(
                    store
                        .put("bench.bin", "application/octet-stream", &data, "did:key:zBench")
                        .unwrap(),
                );
            },
        );
    });
}

fn bench_store_get(c: &mut Criterion) {
    let data = vec![0xABu8; 64 * 1024];
    let mut store = FileStore::new();
    let manifest = store
        .put("bench.bin", "application/octet-stream", &data, "did:key:zBench")
        .unwrap();

    c.bench_function("store_get_64kib", |b| {
        b.iter(|| black_box(store.get(black_box(&manifest.id.0)).unwrap()));
    });
}

fn bench_store_dedup(c: &mut Criterion) {
    let data = vec![0xABu8; 64 * 1024];

    c.bench_function("store_put_dedup", |b| {
        b.iter_with_setup(
            || {
                let mut store = FileStore::new();
                store
                    .put("original.bin", "application/octet-stream", &data, "did:key:zBench")
                    .unwrap();
                store
            },
            |mut store| {
                black_box(
                    store
                        .put("duplicate.bin", "application/octet-stream", &data, "did:key:zBench")
                        .unwrap(),
                );
            },
        );
    });
}

fn bench_vault_encrypt(c: &mut Criterion) {
    let vault = Vault::create("bench-vault", b"benchmark-password").unwrap();
    let key = vault.unlock(b"benchmark-password").unwrap();

    let mut group = c.benchmark_group("vault_encrypt");

    for size in [64, 1024, 65536] {
        let data = vec![0xABu8; size];
        group.bench_with_input(
            criterion::BenchmarkId::from_parameter(size),
            &data,
            |b, data| {
                b.iter(|| {
                    black_box(
                        vault
                            .encrypt_file(black_box(&key), "bench.bin", "application/octet-stream", black_box(data))
                            .unwrap(),
                    )
                });
            },
        );
    }
    group.finish();
}

fn bench_vault_decrypt(c: &mut Criterion) {
    let vault = Vault::create("bench-vault", b"benchmark-password").unwrap();
    let key = vault.unlock(b"benchmark-password").unwrap();

    let mut group = c.benchmark_group("vault_decrypt");

    for size in [64, 1024, 65536] {
        let data = vec![0xABu8; size];
        let entry = vault
            .encrypt_file(&key, "bench.bin", "application/octet-stream", &data)
            .unwrap();
        group.bench_with_input(
            criterion::BenchmarkId::from_parameter(size),
            &entry,
            |b, entry| {
                b.iter(|| {
                    black_box(vault.decrypt_file(black_box(&key), black_box(entry)).unwrap())
                });
            },
        );
    }
    group.finish();
}

fn bench_manifest_serde(c: &mut Criterion) {
    let data = vec![0xABu8; 256 * 1024];
    let mut store = FileStore::new();
    let manifest = store
        .put("bench.bin", "application/octet-stream", &data, "did:key:zBench")
        .unwrap();

    c.bench_function("manifest_serialize", |b| {
        b.iter(|| black_box(serde_json::to_vec(black_box(&manifest)).unwrap()));
    });

    let json = serde_json::to_vec(&manifest).unwrap();

    c.bench_function("manifest_deserialize", |b| {
        b.iter(|| {
            black_box(serde_json::from_slice::<FileManifest>(black_box(&json)).unwrap())
        });
    });
}

criterion_group!(
    benches,
    bench_chunk_small,
    bench_chunk_medium,
    bench_chunk_large,
    bench_reassemble,
    bench_content_id,
    bench_store_put,
    bench_store_get,
    bench_store_dedup,
    bench_vault_encrypt,
    bench_vault_decrypt,
    bench_manifest_serde,
);
criterion_main!(benches);
