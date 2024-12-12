use criterion::{criterion_group, criterion_main, Criterion};
use gemmy::models::OrderOperation;
use gemmy::orderbook::OrderBook;
use gemmy::orderrequest::OrderRequest;

fn small_limit_ladder(c: &mut Criterion) {
    c.bench_function("small limit ladder", |b| {
        let mut orderbook = OrderBook::default();
        b.iter(|| {
            for i in 0..5_000 {
                orderbook.execute(OrderOperation::Place(OrderRequest::new(
                    i as u128,
                    12345 + i,
                    i,
                    gemmy::models::Side::Bid,
                    gemmy::models::OrderType::Limit
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
                    12345 + i,
                    i,
                    gemmy::models::Side::Bid,
                    gemmy::models::OrderType::Limit
                )));
            }
        })
    });
}

criterion_group!(benches, small_limit_ladder, big_limit_ladder);
criterion_main!(benches);

