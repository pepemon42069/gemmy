#[cfg(test)]
mod integration_tests {
    use gemmy::models::{ExecutionResult, FillResult, LimitOrder, Operation, Side};
    use gemmy::orderbook::OrderBook;

    #[test]
    fn orderbook_flow_place_limit_ask_order() {
        let mut orderbook = OrderBook::default();

        let test_order = LimitOrder::new(1, 100, 100, Side::Bid);
        let operation = Operation::Limit(test_order);
        let execution_result = orderbook.execute(operation);

        let (retrieved_order, _) = orderbook.order_store.get(test_order.id).unwrap();
        let expected_max_bid = orderbook.get_max_bid();
        let expected_min_ask = orderbook.get_min_ask();
        let expected_stats = orderbook.stats();

        match execution_result {
            ExecutionResult::Executed(FillResult::Created(created_order)) => {
                let assert_order_flow = || {
                    assert_eq!(created_order, test_order);
                    assert_eq!(*retrieved_order, test_order);
                    assert_eq!(expected_max_bid, Some(100));
                    assert_eq!(expected_min_ask, None);
                    assert_eq!(expected_stats, vec![(100, 100, Side::Bid)]);
                };
                assert_order_flow();
            },
            _ => panic!("expected ExecutionResult::Executed with FillResult::Created"),
        }
    }
}