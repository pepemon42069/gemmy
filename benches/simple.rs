use criterion::{criterion_group, criterion_main, Criterion};
use gemmy::models::Order;
use gemmy::orderbook::OrderBook;

fn small_limit_ladder(c: &mut Criterion) {
    c.bench_function("small limit ladder", |b| {
        let mut orderbook = OrderBook::default();
        b.iter(|| {
            for i in 0..5_000 {
                orderbook.limit_bid_order(
                    12345 + i, Order { id: i as u128, quantity: i });
            }
        })
    });
}

fn big_limit_ladder(c: &mut Criterion) {
    c.bench_function("big limit ladder", |b| {
        let mut orderbook = OrderBook::default();
        b.iter(|| {
            for i in 0..100_000 {
                orderbook.limit_bid_order(
                    12345 + i, Order { id: i as u128, quantity: i });
            }
        })
    });
}

criterion_group!(benches, small_limit_ladder, big_limit_ladder);
criterion_main!(benches);

