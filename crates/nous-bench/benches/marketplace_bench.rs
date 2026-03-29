use criterion::{Criterion, black_box, criterion_group, criterion_main};

use nous_marketplace::listing::{Listing, ListingCategory};
use nous_marketplace::order::Order;
use nous_marketplace::search::{SearchQuery, SortOrder, search};

fn make_listings(n: usize) -> Vec<Listing> {
    let categories = [
        ListingCategory::Physical,
        ListingCategory::Digital,
        ListingCategory::Service,
        ListingCategory::NFT,
        ListingCategory::Data,
    ];
    let tokens = ["ETH", "BTC", "USDC", "SOL"];

    (0..n)
        .map(|i| {
            Listing::new(
                format!("did:key:seller{}", i % 20),
                format!("Item #{i}"),
                format!("Description for item {i} with searchable keywords"),
                categories[i % categories.len()],
                tokens[i % tokens.len()],
                ((i as u128 + 1) * 10),
            )
            .unwrap()
            .with_tag(format!("tag{}", i % 10))
            .with_tag("marketplace")
        })
        .collect()
}

fn bench_listing_create(c: &mut Criterion) {
    c.bench_function("listing_create", |b| {
        b.iter(|| {
            black_box(
                Listing::new(
                    black_box("did:key:seller"),
                    black_box("Widget"),
                    black_box("A fine widget"),
                    black_box(ListingCategory::Physical),
                    black_box("ETH"),
                    black_box(100),
                )
                .unwrap(),
            )
        });
    });
}

fn bench_search_small(c: &mut Criterion) {
    let listings = make_listings(100);
    let query = SearchQuery::new().text("Item #5");

    c.bench_function("search_100_listings", |b| {
        b.iter(|| black_box(search(black_box(&listings), black_box(&query))));
    });
}

fn bench_search_medium(c: &mut Criterion) {
    let listings = make_listings(1000);
    let query = SearchQuery::new().text("searchable");

    c.bench_function("search_1000_listings", |b| {
        b.iter(|| black_box(search(black_box(&listings), black_box(&query))));
    });
}

fn bench_search_large(c: &mut Criterion) {
    let listings = make_listings(10_000);
    let query = SearchQuery::new().text("Item #999");

    c.bench_function("search_10000_listings", |b| {
        b.iter(|| black_box(search(black_box(&listings), black_box(&query))));
    });
}

fn bench_search_filtered(c: &mut Criterion) {
    let listings = make_listings(5000);
    let query = SearchQuery::new()
        .category(ListingCategory::Digital)
        .price_range(10, 500)
        .tag("tag3")
        .sort_by(SortOrder::PriceLow)
        .paginate(20, 0);

    c.bench_function("search_5000_filtered", |b| {
        b.iter(|| black_box(search(black_box(&listings), black_box(&query))));
    });
}

fn bench_search_sort_price(c: &mut Criterion) {
    let listings = make_listings(5000);
    let query = SearchQuery::new().sort_by(SortOrder::PriceHigh);

    c.bench_function("search_5000_sort_price", |b| {
        b.iter(|| black_box(search(black_box(&listings), black_box(&query))));
    });
}

fn bench_order_create(c: &mut Criterion) {
    c.bench_function("order_create", |b| {
        b.iter(|| {
            black_box(
                Order::new(
                    black_box("listing:abc"),
                    black_box("did:key:buyer"),
                    black_box("did:key:seller"),
                    black_box("ETH"),
                    black_box(1000),
                    black_box(1),
                )
                .unwrap(),
            )
        });
    });
}

fn bench_order_full_lifecycle(c: &mut Criterion) {
    c.bench_function("order_full_lifecycle", |b| {
        b.iter(|| {
            let mut order = Order::new(
                "listing:abc",
                "did:key:buyer",
                "did:key:seller",
                "ETH",
                1000,
                1,
            )
            .unwrap();
            order.fund_escrow("escrow:123").unwrap();
            order.ship("did:key:seller", "FedEx", "TRACK001").unwrap();
            order.confirm_delivery("did:key:buyer").unwrap();
            order.complete("did:key:buyer").unwrap();
            black_box(order);
        });
    });
}

fn bench_listing_search_match(c: &mut Criterion) {
    let listing = Listing::new(
        "did:key:seller",
        "Handcrafted Leather Wallet",
        "Premium full-grain leather bifold wallet with RFID protection",
        ListingCategory::Physical,
        "ETH",
        250,
    )
    .unwrap()
    .with_tag("leather")
    .with_tag("accessories")
    .with_tag("handmade");

    c.bench_function("listing_matches_search", |b| {
        b.iter(|| black_box(listing.matches_search(black_box("leather"))));
    });
}

fn bench_listing_serde(c: &mut Criterion) {
    let listing = Listing::new(
        "did:key:seller",
        "Widget",
        "A fine widget",
        ListingCategory::Physical,
        "ETH",
        100,
    )
    .unwrap()
    .with_tag("electronics")
    .with_tag("gadgets");

    c.bench_function("listing_serialize", |b| {
        b.iter(|| black_box(serde_json::to_vec(black_box(&listing)).unwrap()));
    });

    let json = serde_json::to_vec(&listing).unwrap();

    c.bench_function("listing_deserialize", |b| {
        b.iter(|| black_box(serde_json::from_slice::<Listing>(black_box(&json)).unwrap()));
    });
}

criterion_group!(
    benches,
    bench_listing_create,
    bench_search_small,
    bench_search_medium,
    bench_search_large,
    bench_search_filtered,
    bench_search_sort_price,
    bench_order_create,
    bench_order_full_lifecycle,
    bench_listing_search_match,
    bench_listing_serde,
);
criterion_main!(benches);
