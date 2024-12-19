use std::fs::File;
use criterion::{criterion_group, criterion_main, Criterion};
use gemmy::core::{
    models::{LimitOrder, Operation, Side},
    orderbook::OrderBook
};

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

fn load_operations(path: &str) -> Vec<Operation> {
    let file = File::open(path).unwrap();
    let mut operations = Vec::new();
    let mut rdr = csv::ReaderBuilder::new()
        .has_headers(true)
        .from_reader(file);
    let mut id = 0;
    for record in rdr.deserialize::<(u64, Side, u64, u64)>() {
        match record {
            Ok((_, side, price, quantity)) => {
                operations.push(Operation::Limit(LimitOrder::new(id, price, quantity, side)));
                id += 1;
            }
            Err(e) => println!("Error parsing line: {}", e),
        }
    }
    operations
}

fn all_orders(c: &mut Criterion) {
    c.bench_function("all orders", |b| {
        let orders: Vec<Operation> = load_operations("resources/orders.csv");
        let mut orderbook = OrderBook::default();
        b.iter(|| {
            for ord in &orders {
                orderbook.execute(*ord);
            }
        });
    });
}

criterion_group!(
    benches,
    small_limit_ladder,
    insert_and_remove_small_limit_ladder,
    big_limit_ladder,
    all_orders
);
criterion_main!(benches);
