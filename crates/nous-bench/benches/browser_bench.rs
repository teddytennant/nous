use criterion::{Criterion, black_box, criterion_group, criterion_main};

use nous_browser::content_filter::{ContentFilter, FilterRule, RuleCategory};
use nous_browser::history::{BrowsingHistory, HistoryEntry};
use nous_browser::tab::TabManager;

fn bench_filter_rule_match(c: &mut Criterion) {
    let rule = FilterRule::block("||doubleclick.net", RuleCategory::Ads);

    c.bench_function("filter_rule_domain_match", |b| {
        b.iter(|| {
            black_box(rule.matches(black_box("https://ad.doubleclick.net/ddm/trackclk/N123")))
        });
    });
}

fn bench_filter_rule_wildcard(c: &mut Criterion) {
    let rule = FilterRule::block("*tracking*", RuleCategory::Trackers);

    c.bench_function("filter_rule_wildcard_match", |b| {
        b.iter(|| {
            black_box(rule.matches(black_box("https://example.com/api/v2/tracking/pixel.gif")))
        });
    });
}

fn bench_filter_rule_miss(c: &mut Criterion) {
    let rule = FilterRule::block("||doubleclick.net", RuleCategory::Ads);

    c.bench_function("filter_rule_no_match", |b| {
        b.iter(|| black_box(rule.matches(black_box("https://example.com/products/widget"))));
    });
}

fn bench_content_filter_default(c: &mut Criterion) {
    let mut filter = ContentFilter::with_default_rules();

    c.bench_function("content_filter_default_block", |b| {
        b.iter(|| black_box(filter.should_block(black_box("https://ad.doubleclick.net/page"))));
    });
}

fn bench_content_filter_default_pass(c: &mut Criterion) {
    let mut filter = ContentFilter::with_default_rules();

    c.bench_function("content_filter_default_pass", |b| {
        b.iter(|| black_box(filter.should_block(black_box("https://example.com/page"))));
    });
}

fn bench_content_filter_many_rules(c: &mut Criterion) {
    let mut filter = ContentFilter::new();
    for i in 0..500 {
        filter.add_rule(FilterRule::block(
            format!("||ads{i}.example.com"),
            RuleCategory::Ads,
        ));
    }

    c.bench_function("content_filter_500_rules_miss", |b| {
        b.iter(|| black_box(filter.should_block(black_box("https://safe.example.org/page"))));
    });
}

fn bench_content_filter_allow_override(c: &mut Criterion) {
    let mut filter = ContentFilter::new();
    filter.add_rule(FilterRule::block("||example.com", RuleCategory::Ads));
    filter.add_rule(FilterRule::allow("||example.com/safe"));

    c.bench_function("content_filter_allow_override", |b| {
        b.iter(|| black_box(filter.should_block(black_box("https://example.com/safe/page"))));
    });
}

fn bench_tab_manager_open(c: &mut Criterion) {
    c.bench_function("tab_manager_open_tab", |b| {
        b.iter_with_setup(TabManager::new, |mut mgr| {
            black_box(mgr.open(black_box("https://example.com"), black_box("Example")));
        });
    });
}

fn bench_tab_manager_open_many(c: &mut Criterion) {
    c.bench_function("tab_manager_open_50_tabs", |b| {
        b.iter(|| {
            let mut mgr = TabManager::new();
            for i in 0..50 {
                mgr.open(format!("https://site{i}.com"), format!("Site {i}"));
            }
            black_box(mgr);
        });
    });
}

fn bench_history_add(c: &mut Criterion) {
    c.bench_function("history_add_entry", |b| {
        b.iter_with_setup(
            || BrowsingHistory::new(10_000),
            |mut history| {
                history.record(HistoryEntry::new(
                    black_box("https://example.com/page"),
                    black_box("Example Page"),
                ));
                black_box(&history);
            },
        );
    });
}

fn bench_history_search(c: &mut Criterion) {
    let mut history = BrowsingHistory::new(10_000);
    for i in 0..5000 {
        history.record(HistoryEntry::new(
            format!("https://site{}.com/page/{}", i % 100, i),
            format!("Page {i} about topic {}", i % 50),
        ));
    }

    c.bench_function("history_search_5000", |b| {
        b.iter(|| black_box(history.search(black_box("topic 25"))));
    });
}

fn bench_history_domain_stats(c: &mut Criterion) {
    let mut history = BrowsingHistory::new(10_000);
    for i in 0..5000 {
        history.record(HistoryEntry::new(
            format!("https://site{}.com/page/{}", i % 100, i),
            format!("Page {i}"),
        ));
    }

    c.bench_function("history_domain_stats_5000", |b| {
        b.iter(|| black_box(history.unique_domains()));
    });
}

fn bench_filter_serde(c: &mut Criterion) {
    let filter = ContentFilter::with_default_rules();
    let json = serde_json::to_vec(&filter).unwrap();

    c.bench_function("content_filter_serialize", |b| {
        b.iter(|| black_box(serde_json::to_vec(black_box(&filter)).unwrap()));
    });

    c.bench_function("content_filter_deserialize", |b| {
        b.iter(|| black_box(serde_json::from_slice::<ContentFilter>(black_box(&json)).unwrap()));
    });
}

criterion_group!(
    benches,
    bench_filter_rule_match,
    bench_filter_rule_wildcard,
    bench_filter_rule_miss,
    bench_content_filter_default,
    bench_content_filter_default_pass,
    bench_content_filter_many_rules,
    bench_content_filter_allow_override,
    bench_tab_manager_open,
    bench_tab_manager_open_many,
    bench_history_add,
    bench_history_search,
    bench_history_domain_stats,
    bench_filter_serde,
);
criterion_main!(benches);
