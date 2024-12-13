use criterion::{criterion_group, criterion_main, Criterion};
use gemmy::models::{LimitOrder, Operation, Side};
use gemmy::orderbook::OrderBook;
use std::fs::File;

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

criterion_group!(benches, all_orders);
criterion_main!(benches);
