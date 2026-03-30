use criterion::{Criterion, black_box, criterion_group, criterion_main};

use chrono::{Duration, Utc};
use nous_payments::escrow::Escrow;
use nous_payments::invoice::{Invoice, LineItem};
use nous_payments::stream::{PaymentStream, StreamConfig};
use nous_payments::swap::{SwapBook, SwapOrder};
use nous_payments::wallet::{Wallet, transfer};

// ── Wallet Operations ───────────────────────────────────────────

fn bench_wallet_create_and_credit(c: &mut Criterion) {
    c.bench_function("wallet_create_and_credit", |b| {
        b.iter(|| {
            let mut wallet = Wallet::new("did:key:bench");
            wallet.credit("ETH", 1_000_000_000);
            black_box(wallet.balance("ETH"));
        });
    });
}

fn bench_wallet_transfer(c: &mut Criterion) {
    c.bench_function("wallet_transfer_100", |b| {
        b.iter(|| {
            let mut alice = Wallet::new("did:key:alice");
            let mut bob = Wallet::new("did:key:bob");
            alice.credit("ETH", 1_000_000_000);

            for _ in 0..100 {
                let tx = transfer(&mut alice, &mut bob, "ETH", 1_000).unwrap();
                black_box(&tx);
            }
            black_box((alice.balance("ETH"), bob.balance("ETH")));
        });
    });
}

// ── Escrow ──────────────────────────────────────────────────────

fn bench_escrow_lifecycle(c: &mut Criterion) {
    c.bench_function("escrow_create_release", |b| {
        b.iter(|| {
            let mut escrow = Escrow::new(
                "did:key:buyer",
                "did:key:seller",
                "USDC",
                50_000,
                "bench",
                24,
            )
            .unwrap();
            escrow.release("did:key:buyer").unwrap();
            black_box(&escrow);
        });
    });
}

fn bench_escrow_dispute(c: &mut Criterion) {
    c.bench_function("escrow_dispute", |b| {
        b.iter(|| {
            let mut escrow = Escrow::new(
                "did:key:buyer",
                "did:key:seller",
                "USDC",
                50_000,
                "bench",
                24,
            )
            .unwrap();
            escrow.dispute("did:key:buyer").unwrap();
            black_box(&escrow);
        });
    });
}

// ── Payment Streams ─────────────────────────────────────────────

fn bench_stream_create_and_activate(c: &mut Criterion) {
    c.bench_function("stream_create_activate", |b| {
        b.iter(|| {
            let config = StreamConfig {
                rate_per_second: 1_000,
                token: "ETH".to_string(),
                continuous_claim: true,
                min_claim_interval_secs: 0,
                max_duration_secs: 0,
            };
            let mut stream =
                PaymentStream::create("did:key:payer", "did:key:payee", config, 1_000_000).unwrap();
            stream.activate().unwrap();
            black_box(&stream);
        });
    });
}

fn bench_stream_claim_cycle(c: &mut Criterion) {
    c.bench_function("stream_deposit_activate_claim", |b| {
        b.iter(|| {
            let config = StreamConfig {
                rate_per_second: 100,
                token: "USDC".to_string(),
                continuous_claim: true,
                min_claim_interval_secs: 0,
                max_duration_secs: 0,
            };
            let mut stream =
                PaymentStream::create("did:key:payer", "did:key:payee", config, 1_000_000).unwrap();
            stream.activate().unwrap();

            let future = Utc::now() + Duration::seconds(100);
            let receipt = stream.claim_at(future);
            let _ = black_box(receipt);
        });
    });
}

// ── Invoices ────────────────────────────────────────────────────

fn bench_invoice_create_with_items(c: &mut Criterion) {
    c.bench_function("invoice_create_10_items", |b| {
        b.iter(|| {
            let mut invoice = Invoice::new("did:key:seller", "did:key:buyer", "USDC", 30);
            for i in 0..10 {
                invoice.items.push(LineItem::new(
                    &format!("Item {i}"),
                    (i + 1) as u32,
                    1000 * (i + 1) as u128,
                ));
            }
            black_box(invoice);
        });
    });
}

// ── Swap Book ───────────────────────────────────────────────────

fn bench_swap_book_insert_and_match(c: &mut Criterion) {
    c.bench_function("swap_book_100_orders_find_match", |b| {
        b.iter(|| {
            let mut book = SwapBook::new();
            for i in 0..100 {
                let order = SwapOrder::new(
                    &format!("did:key:trader{i}"),
                    "ETH",
                    1_000 + i as u128,
                    "USDC",
                    2_000_000 + i as u128 * 1_000,
                    24,
                )
                .unwrap();
                book.add(order);
            }
            let found = book.find_match("USDC", "ETH", 2500.0);
            black_box(found);
        });
    });
}

fn bench_wallet_serialization(c: &mut Criterion) {
    c.bench_function("wallet_serialize_deserialize", |b| {
        let mut wallet = Wallet::new("did:key:bench");
        wallet.credit("ETH", 1_000_000_000);
        for i in 0..50 {
            wallet.debit("ETH", 100).unwrap();
            let _ = i;
        }

        b.iter(|| {
            let json = serde_json::to_vec(&wallet).unwrap();
            let restored: Wallet = serde_json::from_slice(&json).unwrap();
            black_box(restored);
        });
    });
}

criterion_group!(
    benches,
    bench_wallet_create_and_credit,
    bench_wallet_transfer,
    bench_escrow_lifecycle,
    bench_escrow_dispute,
    bench_stream_create_and_activate,
    bench_stream_claim_cycle,
    bench_invoice_create_with_items,
    bench_swap_book_insert_and_match,
    bench_wallet_serialization,
);

criterion_main!(benches);
