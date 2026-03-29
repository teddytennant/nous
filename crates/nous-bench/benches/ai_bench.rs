use criterion::{Criterion, black_box, criterion_group, criterion_main};

use nous_ai::chunking::{ChunkOptions, ChunkStrategy, chunk_text};
use nous_ai::conversation::{Conversation, Message};
use nous_ai::embedding::{Embedding, EmbeddingIndex};
use nous_ai::knowledge::KnowledgeBase;

fn generate_text(paragraphs: usize) -> String {
    (0..paragraphs)
        .map(|i| {
            format!(
                "Paragraph {i}. This is a sample paragraph with enough text to be meaningful. \
                 It contains multiple sentences that discuss various topics including governance, \
                 identity management, and decentralized systems. The purpose is to benchmark \
                 how text chunking performs on realistic document content."
            )
        })
        .collect::<Vec<_>>()
        .join("\n\n")
}

fn bench_chunk_paragraph(c: &mut Criterion) {
    let text = generate_text(50);
    let options = ChunkOptions::default();

    c.bench_function("chunk_paragraph_50", |b| {
        b.iter(|| black_box(chunk_text(black_box(&text), black_box(&options))));
    });
}

fn bench_chunk_paragraph_large(c: &mut Criterion) {
    let text = generate_text(500);
    let options = ChunkOptions::default();

    c.bench_function("chunk_paragraph_500", |b| {
        b.iter(|| black_box(chunk_text(black_box(&text), black_box(&options))));
    });
}

fn bench_chunk_fixed(c: &mut Criterion) {
    let text = generate_text(100);
    let options = ChunkOptions {
        max_chars: 256,
        overlap: 32,
        strategy: ChunkStrategy::FixedSize,
    };

    c.bench_function("chunk_fixed_256_overlap32", |b| {
        b.iter(|| black_box(chunk_text(black_box(&text), black_box(&options))));
    });
}

fn bench_chunk_sentence(c: &mut Criterion) {
    let text = generate_text(100);
    let options = ChunkOptions {
        max_chars: 512,
        overlap: 0,
        strategy: ChunkStrategy::Sentence,
    };

    c.bench_function("chunk_sentence_100", |b| {
        b.iter(|| black_box(chunk_text(black_box(&text), black_box(&options))));
    });
}

fn make_embedding(dim: usize, seed: u32) -> Embedding {
    let vector: Vec<f32> = (0..dim)
        .map(|i| ((i as f32 + seed as f32) * 0.1).sin())
        .collect();
    Embedding::new(vector, "bench")
}

fn bench_cosine_similarity(c: &mut Criterion) {
    let a = make_embedding(384, 1);
    let b = make_embedding(384, 2);

    c.bench_function("cosine_similarity_384d", |b_iter| {
        b_iter.iter(|| black_box(a.cosine_similarity(black_box(&b))));
    });
}

fn bench_embedding_index_insert(c: &mut Criterion) {
    c.bench_function("embedding_index_insert", |b| {
        b.iter_with_setup(EmbeddingIndex::new, |mut index| {
            let emb = make_embedding(384, 42);
            index.insert("doc-bench", emb, "benchmark text");
            black_box(&index);
        });
    });
}

fn bench_embedding_search_100(c: &mut Criterion) {
    let mut index = EmbeddingIndex::new();
    for i in 0..100 {
        let emb = make_embedding(384, i);
        index.insert(format!("doc-{i}"), emb, format!("text for doc {i}"));
    }
    let query = make_embedding(384, 999);

    c.bench_function("embedding_search_100_384d", |b| {
        b.iter(|| black_box(index.search(black_box(&query), black_box(5))));
    });
}

fn bench_embedding_search_1000(c: &mut Criterion) {
    let mut index = EmbeddingIndex::new();
    for i in 0..1000 {
        let emb = make_embedding(384, i);
        index.insert(format!("doc-{i}"), emb, format!("text {i}"));
    }
    let query = make_embedding(384, 9999);

    c.bench_function("embedding_search_1000_384d", |b| {
        b.iter(|| black_box(index.search(black_box(&query), black_box(10))));
    });
}

fn bench_knowledge_ingest(c: &mut Criterion) {
    let text = generate_text(20);
    let options = ChunkOptions::default();

    c.bench_function("knowledge_ingest_20p", |b| {
        b.iter_with_setup(
            || KnowledgeBase::in_memory().unwrap(),
            |mut kb| {
                black_box(
                    kb.ingest(
                        black_box("Bench Doc"),
                        black_box(&text),
                        black_box("bench"),
                        black_box(&options),
                    )
                    .unwrap(),
                );
            },
        );
    });
}

fn bench_conversation_token_estimate(c: &mut Criterion) {
    let mut conv = Conversation::new("bench-agent");
    conv.add_message(Message::system(
        "You are a helpful assistant for governance analysis.",
    ));
    for i in 0..50 {
        conv.add_message(Message::user(format!(
            "Question {i}: What is the impact of proposal {i} on token holders?"
        )));
        conv.add_message(Message::assistant(format!(
            "The proposal {i} would affect token holders by modifying the reward distribution."
        )));
    }

    c.bench_function("conversation_token_estimate_100msg", |b| {
        b.iter(|| black_box(conv.total_tokens_estimate()));
    });
}

fn bench_conversation_truncate(c: &mut Criterion) {
    let mut conv = Conversation::new("bench-agent");
    for i in 0..200 {
        conv.add_message(Message::user(format!("Message {i} with moderate length.")));
    }

    c.bench_function("conversation_truncate_200msg", |b| {
        b.iter(|| black_box(conv.truncate_to_tokens(black_box(4096))));
    });
}

criterion_group!(
    benches,
    bench_chunk_paragraph,
    bench_chunk_paragraph_large,
    bench_chunk_fixed,
    bench_chunk_sentence,
    bench_cosine_similarity,
    bench_embedding_index_insert,
    bench_embedding_search_100,
    bench_embedding_search_1000,
    bench_knowledge_ingest,
    bench_conversation_token_estimate,
    bench_conversation_truncate,
);
criterion_main!(benches);
