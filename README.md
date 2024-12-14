# Gemmy

My goal with writing gemmy is to create a useful, and production ready high performance orderbook written in rust.

For now the work is ongoing, and the implementation is bound to be full of bugs. I will be adding more to this readme as I keep doing more.

# To Dos

- [ ] Add exhaustive integration tests.
- [ ] Add unit tests for a few remaining methods.
- [x] Complete the documentation of code.
- [ ] Replace this readme with a summary of everything.
- [x] Publish the crate.
- [ ] Add stats tracking (volume, last_trade, etc.)

# Usage
Using gemmy is pretty straightforward, you can use this example as a test.
```rust
#[test]
fn example() {
    // create the orderbook
    let mut orderbook = OrderBook::default();
    
    // create an order and wrap it in the corresponding operation
    let order_ask = LimitOrder::new(1, 100, 100, Side::Ask);
    let operation_limit_ask = Operation::Limit(order_ask);
    
    // call the execute method with the operation
    match orderbook.execute(operation_limit_ask) {
        
        // this results in an execution result, which is creation of a limit ask order
        ExecutionResult::Executed(FillResult::Created(created_order)) => {
            println!("created_order: {:#?}", created_order);
            
            // you can query the orderbook using other methods to know its state
            println!("min_ask: {}", orderbook.get_min_ask().unwrap());
            println!("depth: {:#?}",orderbook.depth(1));
        }
        _ => panic!("expected order to be created"),
    }
    
    // placing another order here that can fill completely
    let order_bid = MarketOrder::new(2, 50, Side::Bid);
    let operation_market_bid = Operation::Market(order_bid);
    match orderbook.execute(operation_market_bid) {
        // this time we can see how exactly the order got matched
        ExecutionResult::Executed(FillResult::Filled(order_fills)) => {
            println!("order_fills: {:#?}", order_fills);
            println!("depth: {:#?}",orderbook.depth(1));
        }
        _ => panic!("expected order to be filled"),
    }
    
    // we create a thord and final order to see a better view of the book
    let order_bid_second = LimitOrder::new(3, 50, 100, Side::Bid);
    let operation_limit_bid = Operation::Limit(order_bid_second);
    match orderbook.execute(operation_limit_bid) {
        ExecutionResult::Executed(FillResult::Created(created_order)) => {
            println!("created_order: {:#?}", created_order);
            println!("max_bid: {}", orderbook.get_max_bid().unwrap());
            println!("depth: {:#?}",orderbook.depth(1));
        }
        _ => panic!("expected order to be created"),
    }
}
```

The following is the output of the above snippet.
```
// ask order created
created_order: LimitOrder {
    id: 1,
    price: 100,
    quantity: 100,
    side: Ask,
}

// orderbook state
min_ask: 100
depth: Depth {
    levels: 1,
    bids: [],
    asks: [
        Level {
            price: 100,
            quantity: 100,
        },
    ],
}

// market bid filled
order_fills: [
    FillMetaData {
        order_id: 2,
        matched_order_id: 1,
        taker_side: Bid,
        price: 100,
        quantity: 50,
    },
]

// orderbook state
depth: Depth {
    levels: 1,
    bids: [],
    asks: [
        Level {
            price: 100,
            quantity: 50, // a fill of 50 reflected
        },
    ],
}

// a new bid side limit order placed at a lower price than ask
created_order: LimitOrder {
    id: 3,
    price: 50,
    quantity: 100,
    side: Bid,
}

// orderbook state
max_bid: 50
depth: Depth {
    levels: 1,
    bids: [
        Level {
            price: 50,
            quantity: 100,
        },
    ],
    asks: [
        Level {
            price: 100,
            quantity: 50,
        },
    ],
}
```