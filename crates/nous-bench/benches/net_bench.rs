use criterion::{Criterion, black_box, criterion_group, criterion_main};
use std::time::Duration;

use nous_net::gossip::{GossipConfig, GossipEngine, GossipMessage, VectorClock};

fn make_engine(id: &str, peers: usize) -> GossipEngine {
    let mut engine = GossipEngine::new(
        id.to_string(),
        GossipConfig {
            fanout: 6,
            max_message_age: Duration::from_secs(300),
            max_buffer_size: 10_000,
            sync_interval: Duration::from_secs(30),
            max_ttl: 8,
        },
    );
    for i in 0..peers {
        engine.add_peer(format!("peer-{i}"));
    }
    engine
}

fn bench_originate(c: &mut Criterion) {
    c.bench_function("gossip_originate", |b| {
        b.iter_with_setup(
            || make_engine("node-a", 10),
            |mut engine| {
                black_box(engine.originate(black_box(b"hello world".to_vec())));
            },
        );
    });
}

fn bench_receive_new(c: &mut Criterion) {
    c.bench_function("gossip_receive_new", |b| {
        b.iter_with_setup(
            || {
                let engine = make_engine("node-b", 10);
                let msg = GossipMessage::new(
                    "node-a".into(),
                    b"payload data".to_vec(),
                    VectorClock::new(),
                );
                (engine, msg)
            },
            |(mut engine, msg)| {
                black_box(engine.receive(black_box(msg), black_box("node-a")));
            },
        );
    });
}

fn bench_receive_duplicate(c: &mut Criterion) {
    c.bench_function("gossip_receive_duplicate", |b| {
        b.iter_with_setup(
            || {
                let mut engine = make_engine("node-b", 5);
                let msg =
                    GossipMessage::new("node-a".into(), b"dup data".to_vec(), VectorClock::new());
                engine.receive(msg.clone(), "node-a");
                (engine, msg)
            },
            |(mut engine, msg)| {
                black_box(engine.receive(black_box(msg), black_box("node-a")));
            },
        );
    });
}

fn bench_build_digest(c: &mut Criterion) {
    let mut engine = make_engine("node-a", 10);
    for i in 0..500 {
        engine.originate(format!("msg-{i}").into_bytes());
    }

    c.bench_function("gossip_build_digest_500", |b| {
        b.iter(|| black_box(engine.build_digest()));
    });
}

fn bench_handle_digest(c: &mut Criterion) {
    let mut engine = make_engine("node-a", 10);
    for i in 0..500 {
        engine.originate(format!("msg-{i}").into_bytes());
    }

    let empty_digest = nous_net::gossip::GossipDigest::new();

    c.bench_function("gossip_handle_digest_500", |b| {
        b.iter(|| black_box(engine.handle_digest(black_box(&empty_digest), black_box("node-b"))));
    });
}

fn bench_vector_clock_merge(c: &mut Criterion) {
    let mut c1 = VectorClock::new();
    for i in 0..50 {
        for _ in 0..10 {
            c1.increment(&format!("node-{i}"));
        }
    }

    let mut c2 = VectorClock::new();
    for i in 25..75 {
        for _ in 0..10 {
            c2.increment(&format!("node-{i}"));
        }
    }

    c.bench_function("vector_clock_merge_50_nodes", |b| {
        b.iter_with_setup(
            || c1.clone(),
            |mut clock| {
                clock.merge(black_box(&c2));
                black_box(clock);
            },
        );
    });
}

fn bench_vector_clock_precedes(c: &mut Criterion) {
    let mut c1 = VectorClock::new();
    let mut c2 = VectorClock::new();
    for i in 0..100 {
        c1.increment(&format!("node-{i}"));
        c2.increment(&format!("node-{i}"));
        c2.increment(&format!("node-{i}"));
    }

    c.bench_function("vector_clock_precedes_100_nodes", |b| {
        b.iter(|| black_box(c1.precedes(black_box(&c2))));
    });
}

fn bench_tick(c: &mut Criterion) {
    let mut engine = make_engine("node-a", 20);
    for i in 0..200 {
        engine.originate(format!("msg-{i}").into_bytes());
    }

    c.bench_function("gossip_tick_200_msgs", |b| {
        b.iter_with_setup(
            || engine.message_count(),
            |_| {
                black_box(engine.tick());
            },
        );
    });
}

criterion_group!(
    benches,
    bench_originate,
    bench_receive_new,
    bench_receive_duplicate,
    bench_build_digest,
    bench_handle_digest,
    bench_vector_clock_merge,
    bench_vector_clock_precedes,
    bench_tick,
);
criterion_main!(benches);
