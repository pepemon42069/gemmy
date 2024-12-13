use criterion::{criterion_group, criterion_main, Criterion};
use gemmy::models::{OrderOperation, OrderRequest, OrderType, Side};
use gemmy::orderbook::OrderBook;

fn small_limit_ladder(c: &mut Criterion) {
    c.bench_function("small limit ladder", |b| {
        let mut orderbook = OrderBook::default();
        b.iter(|| {
            for i in 0..5_000 {
                orderbook.execute(OrderOperation::Place(OrderRequest::new(
                    i as u128,
                    Some(12345 + i),
                    i,
                    Side::Bid,
                    OrderType::Limit,
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
                orderbook.execute(OrderOperation::Place(OrderRequest::new(
                    i as u128,
                    Some(12345 + i),
                    i,
                    Side::Bid,
                    OrderType::Limit,
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
                let order =
                    OrderRequest::new(i as u128, Some(12345 + i), i, Side::Bid, OrderType::Limit);
                book.execute(OrderOperation::Place(order));
            }
            for i in 1..5000u64 {
                let order =
                    OrderRequest::new(i as u128, Some(12345 + i), i, Side::Bid, OrderType::Limit);
                book.execute(OrderOperation::Cancel(order));
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
