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

        let expected_max_bid = orderbook.get_max_bid();
        let expected_min_ask = orderbook.get_min_ask();
        let expected_depth = orderbook.depth(1);

        match execution_result {
            ExecutionResult::Executed(FillResult::Created(created_order)) => {
                let assert_order_flow = || {
                    assert_eq!(created_order, test_order);
                    assert_eq!(expected_max_bid, Some(100));
                    assert_eq!(expected_min_ask, None);
                    assert_eq!(expected_depth.bids.len(), 1);
                };
                assert_order_flow();
            },
            _ => panic!("expected ExecutionResult::Executed with FillResult::Created"),
        }
    }

    #[test]
    fn orderbook_flow_place_2_limit_ask_orders() {
        let mut orderbook = OrderBook::default();

        let test_order_1 = LimitOrder::new(1, 100, 100, Side::Bid);
        let operation_1 = Operation::Limit(test_order_1);

        let test_order_2 = LimitOrder::new(2, 110, 200, Side::Ask);
        let operation_2 = Operation::Limit(test_order_2);
        
        let execution_result_1 = orderbook.execute(operation_1);
        let execution_result_2 = orderbook.execute(operation_2);
        
        let expected_max_bid = orderbook.get_max_bid();
        let expected_min_ask = orderbook.get_min_ask();
        let expected_depth = orderbook.depth(2);


        match (execution_result_1, execution_result_2) {
            (
                ExecutionResult::Executed(FillResult::Created(created_order_1)),
                ExecutionResult::Executed(FillResult::Created(created_order_2)),
            ) => {
                let assert_order_flow = || {
                    assert_eq!(created_order_1, test_order_1);
                    assert_eq!(created_order_2, test_order_2);
                    assert_eq!(expected_max_bid, Some(100));
                    assert_eq!(expected_min_ask, Some(110));
                    assert_eq!(expected_depth.bids.len(), 1);
                    assert_eq!(expected_depth.asks.len(), 1);
                };
                assert_order_flow();
            },
            _ => panic!("expected ExecutionResult::Executed with FillResult::Created for both orders"),
        }
    }
}