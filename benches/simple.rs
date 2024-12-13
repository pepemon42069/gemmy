use criterion::{criterion_group, criterion_main, Criterion};
use gemmy::models::{LimitOrder, Operation, Side};
use gemmy::orderbook::OrderBook;

fn small_limit_ladder(c: &mut Criterion) {
    c.bench_function("small limit ladder", |b| {
        let mut orderbook = OrderBook::default();
        b.iter(|| {
            for i in 0..5_000 {
                orderbook.execute(Operation::Limit(LimitOrder::new(
                    i as u128,
                    12345 + i,
                    i,
                    Side::Bid,
                )));
            }
        })
    });
}

fn big_limit_ladder(c: &mut Criterion) {
    c.bench_function("big limit ladder", |b| {
        let mut orderbook = OrderBook::default();
        b.iter(|| {
            for i in 0..100_000 {
                orderbook.execute(Operation::Limit(LimitOrder::new(
                    i as u128,
                    12345 + i,
                    i,
                    Side::Bid,
                )));
            }
        })
    });
}
fn insert_and_remove_small_limit_ladder(c: &mut Criterion) {
    c.bench_function("insert and remove small limit ladder", |b| {
        let mut book = OrderBook::default();
        b.iter(|| {
            for i in 1..5000u64 {
                let order = LimitOrder::new(i as u128, 12345 + i, i, Side::Bid);
                book.execute(Operation::Limit(order));
            }
            for i in 1..5000u128 {
                book.execute(Operation::Cancel(i));
            }
        })
    });
}

criterion_group!(
    benches,
    insert_and_remove_small_limit_ladder,
    small_limit_ladder,
    big_limit_ladder
);
criterion_main!(benches);
