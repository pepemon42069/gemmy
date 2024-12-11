use criterion::{criterion_group, criterion_main, Criterion};
use gemmy::models::{OrderOperation, OrderType, Side};
use gemmy::orderbook::OrderBook;
use gemmy::orderrequest::OrderRequest;

fn criterion_benchmark(c: &mut Criterion) {
    let mut orderbook = OrderBook::default();
    c.bench_function("criterion_benchmark", |b| {
        b.iter(|| {
            for i in 0..100_000 {
                let operation = OrderOperation::Place(
                    OrderRequest::new(i as u128, 12345 + i, i, Side::Bid, OrderType::Limit));
                orderbook.execute(operation);
            }
        })
    });
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);

