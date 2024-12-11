use rand::Rng;
use criterion::{black_box, criterion_group, criterion_main, Criterion};
use gemmy::models::{OrderOperation, OrderType, Side};
use gemmy::orderbook::OrderBook;
use gemmy::orderrequest::OrderRequest;

fn generate_op() -> OrderOperation {
    let mut rng = rand::thread_rng();
    let case = rng.gen_range(0..2);
    let quantity: u64  = rng.gen_range(1000..=2000);
    match case {
        0 => OrderOperation::Place(OrderRequest::new(rng.gen_range(0..1000), quantity, Side::Bid, OrderType::Limit)),
        1 => OrderOperation::Place(OrderRequest::new(rng.gen_range(1000..2000), quantity, Side::Ask, OrderType::Limit)),
        _ => panic!()
    }
}

fn criterion_benchmark(c: &mut Criterion) {
    let mut orderbook = OrderBook::new();
    c.bench_function("criterion_benchmark", |b| {
        b.iter(|| {
            for _ in 0..10 {
                let operation = generate_op();
                black_box(orderbook.execute(operation));
            }
        })
    });
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);

