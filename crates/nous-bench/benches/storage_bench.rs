use criterion::{Criterion, black_box, criterion_group, criterion_main};

use nous_storage::merkle::MerkleTree;

fn make_leaves(n: usize) -> Vec<Vec<u8>> {
    (0..n)
        .map(|i| format!("leaf-data-{i}").into_bytes())
        .collect()
}

fn bench_merkle_build_small(c: &mut Criterion) {
    let data = make_leaves(8);
    let refs: Vec<&[u8]> = data.iter().map(|d| d.as_slice()).collect();

    c.bench_function("merkle_build_8_leaves", |b| {
        b.iter(|| black_box(MerkleTree::from_leaves(black_box(&refs))));
    });
}

fn bench_merkle_build_medium(c: &mut Criterion) {
    let data = make_leaves(256);
    let refs: Vec<&[u8]> = data.iter().map(|d| d.as_slice()).collect();

    c.bench_function("merkle_build_256_leaves", |b| {
        b.iter(|| black_box(MerkleTree::from_leaves(black_box(&refs))));
    });
}

fn bench_merkle_build_large(c: &mut Criterion) {
    let data = make_leaves(4096);
    let refs: Vec<&[u8]> = data.iter().map(|d| d.as_slice()).collect();

    c.bench_function("merkle_build_4096_leaves", |b| {
        b.iter(|| black_box(MerkleTree::from_leaves(black_box(&refs))));
    });
}

fn bench_merkle_proof(c: &mut Criterion) {
    let data = make_leaves(1024);
    let refs: Vec<&[u8]> = data.iter().map(|d| d.as_slice()).collect();
    let tree = MerkleTree::from_leaves(&refs);

    c.bench_function("merkle_proof_1024_leaves", |b| {
        b.iter(|| black_box(tree.proof(black_box(512))));
    });
}

fn bench_merkle_verify(c: &mut Criterion) {
    let data = make_leaves(1024);
    let refs: Vec<&[u8]> = data.iter().map(|d| d.as_slice()).collect();
    let tree = MerkleTree::from_leaves(&refs);
    let proof = tree.proof(512).unwrap();

    c.bench_function("merkle_verify_1024_leaves", |b| {
        b.iter(|| black_box(proof.verify()));
    });
}

fn bench_merkle_update_leaf(c: &mut Criterion) {
    let data = make_leaves(1024);
    let refs: Vec<&[u8]> = data.iter().map(|d| d.as_slice()).collect();

    c.bench_function("merkle_update_leaf_1024", |b| {
        b.iter_with_setup(
            || MerkleTree::from_leaves(&refs),
            |mut tree| {
                tree.update_leaf(black_box(500), black_box(b"updated-data"));
                black_box(&tree);
            },
        );
    });
}

fn bench_merkle_diff(c: &mut Criterion) {
    let data1 = make_leaves(256);
    let mut data2 = make_leaves(256);
    // Modify 10 leaves.
    for i in [0, 25, 50, 75, 100, 125, 150, 175, 200, 225] {
        data2[i] = format!("modified-{i}").into_bytes();
    }

    let refs1: Vec<&[u8]> = data1.iter().map(|d| d.as_slice()).collect();
    let refs2: Vec<&[u8]> = data2.iter().map(|d| d.as_slice()).collect();

    let t1 = MerkleTree::from_leaves(&refs1);
    let t2 = MerkleTree::from_leaves(&refs2);

    c.bench_function("merkle_diff_256_10_changes", |b| {
        b.iter(|| black_box(t1.diff(black_box(&t2))));
    });
}

fn bench_merkle_diff_identical(c: &mut Criterion) {
    let data = make_leaves(1024);
    let refs: Vec<&[u8]> = data.iter().map(|d| d.as_slice()).collect();
    let t1 = MerkleTree::from_leaves(&refs);
    let t2 = MerkleTree::from_leaves(&refs);

    c.bench_function("merkle_diff_1024_identical", |b| {
        b.iter(|| black_box(t1.diff(black_box(&t2))));
    });
}

criterion_group!(
    benches,
    bench_merkle_build_small,
    bench_merkle_build_medium,
    bench_merkle_build_large,
    bench_merkle_proof,
    bench_merkle_verify,
    bench_merkle_update_leaf,
    bench_merkle_diff,
    bench_merkle_diff_identical,
);
criterion_main!(benches);
