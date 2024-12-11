use std::collections::VecDeque;
use crate::models::{ExecutionResult, FillResult, ModifyResult, Order, OrderOperation, OrderType, Side};
use crate::pricebook::PriceBook;

#[derive(Debug)]
pub struct OrderBook {
    pub id: u128,
    pub max_bid: u64,
    pub min_ask: u64,
    bid_side_book: PriceBook,
    ask_side_book: PriceBook
}

impl Default for OrderBook {
    fn default() -> Self {
        Self::new(0)
    }
}

impl OrderBook {
    pub fn new(id: u128) -> Self {
        OrderBook {
            id,
            max_bid: u64::MAX,
            min_ask: u64::MIN,
            bid_side_book: PriceBook::new(),
            ask_side_book: PriceBook::new(),
        }
    }
    
    pub fn execute(&mut self, operation: OrderOperation) -> ExecutionResult {
        match operation {
            OrderOperation::Place(order) => {
                let book_order = order.to_order();
                match order.side {
                    Side::Bid => {
                        match order.order_type {
                            OrderType::Limit => {
                                ExecutionResult::Executed(
                                    self.limit_bid_order(order.price, book_order))
                            }
                            OrderType::Market => {
                                ExecutionResult::Executed(self.market_bid_order(book_order))
                            }
                        }
                    }
                    Side::Ask => {
                        match order.order_type {
                            OrderType::Limit => {
                                ExecutionResult::Executed(
                                    self.limit_ask_order(order.price, book_order))
                            }
                            OrderType::Market => {
                                ExecutionResult::Executed(self.market_ask_order(book_order))
                            }
                        }
                    }
                }
            }
            OrderOperation::Modify(order, new_price, new_quantity) => {
                match order.side {
                    Side::Bid => {
                        ExecutionResult::Modified(self.modify_limit_buy_order(
                            order.id, order.price, new_price, new_quantity))
                        
                    }
                    Side::Ask => {
                        ExecutionResult::Modified(self.modify_limit_ask_order(
                            order.id, order.price, new_price, new_quantity))
                    }
                }
            }
            OrderOperation::Cancel(order) => {
                match order.side {
                    Side::Bid => {
                        self.cancel_bid_order(order.id, order.price);
                        ExecutionResult::Cancelled(order.id)
                    }
                    Side::Ask => {
                        self.cancel_ask_order(order.id, order.price);
                        ExecutionResult::Cancelled(order.id)
                    }
                }
            }
        }
    }

    pub fn stats(&self) -> Vec<(u64, u64, Side)> {
        let mut orders = vec![];
        let bid_side_map = &self.bid_side_book.price_map;
        for (price, _) in bid_side_map.iter() {
            let quantity = self.bid_side_book.get_total_quantity_at_price(price);
            orders.push((*price, quantity, Side::Bid));
        }
        let ask_side_map = &self.ask_side_book.price_map;
        for (price, _) in ask_side_map.iter() {
            let quantity = self.ask_side_book.get_total_quantity_at_price(price);
            orders.push((*price, quantity, Side::Ask));
        }
        orders
    }
    
    fn cancel_bid_order(&mut self, id: u128, price: u64) {
        self.bid_side_book.remove(&id, &price);
        self.update_max_bid();
    }

    fn cancel_ask_order(&mut self, id: u128, price: u64) {
        self.ask_side_book.remove(&id, &price);
        self.update_min_ask();
    }

    fn modify_limit_buy_order(
        &mut self, id: u128, price: u64, new_quantity: u64, new_price: u64) -> Option<FillResult> {
        let result = Self::process_order_modification(
            &mut self.bid_side_book, id, price, new_quantity, new_price);
        match result {
            ModifyResult::CreateNewOrder => {
                Some(self.limit_bid_order(new_price, Order {id, quantity: new_quantity }))
            }
            ModifyResult::ModifiedOrder => {
                self.update_max_bid();
                None
            }
            _ => None
        }
    }

    fn modify_limit_ask_order(
        &mut self, id: u128, price: u64, new_quantity: u64, new_price: u64) -> Option<FillResult> {
        let result = Self::process_order_modification(
            &mut self.ask_side_book, id, price, new_quantity, new_price);
        match result {
            ModifyResult::CreateNewOrder => {
                Some(self.limit_ask_order(new_price, Order {id, quantity: new_quantity }))
            }
            ModifyResult::ModifiedOrder => {
                self.update_min_ask();
                None
            }
            _ => None
        }
    }

    fn update_max_bid(&mut self) {
        if let Some((price, _)) = self.bid_side_book.price_map.iter()
            .filter(|(_, order_queue)| !order_queue.is_empty()).last(){
            self.max_bid = *price;
        }
    }

    fn update_min_ask(&mut self) {
        if let Some((price, _)) = self.ask_side_book.price_map.iter().rev()
            .filter(|(_, order_queue)| !order_queue.is_empty()).last() {
            self.min_ask = *price;
        }
    }

    fn limit_bid_order(&mut self, price: u64, order: Order) -> FillResult {
        let mut fills = Vec::new();
        let mut remaining_quantity = order.quantity;
        let asks = self.ask_side_book.price_map.iter_mut();
        for (ask, order_queue) in asks {
            if order_queue.is_empty() {
                continue;
            }
            if price < *ask { break; }
            Self::process_order_queue(&mut fills, &mut remaining_quantity, *ask, order_queue);
        }
        let fill_result = Self::process_fills(
            remaining_quantity, fills, price, order, &mut self.bid_side_book);
        // TODO: 1 bottleneck found
        // self.update_max_bid();
        fill_result
    }

    fn limit_ask_order(&mut self, price: u64, order: Order) -> FillResult {
        let mut fills = Vec::new();
        let mut remaining_quantity = order.quantity;
        let bids = self.bid_side_book.price_map.iter_mut().rev();
        for (bid, order_queue) in bids {
            if order_queue.is_empty() {
                continue;
            }
            if price > *bid { break; }
            Self::process_order_queue(&mut fills, &mut remaining_quantity, *bid, order_queue);
        }
        let fill_result = Self::process_fills(
            remaining_quantity, fills, price, order, &mut self.ask_side_book);
        self.update_min_ask();
        fill_result
    }

    fn market_bid_order(&mut self, order: Order) -> FillResult {
        let mut fills = Vec::new();
        let mut remaining_quantity = order.quantity;
        let asks = self.ask_side_book.price_map.iter_mut();
        for (ask, order_queue) in asks {
            if order_queue.is_empty() {
                continue;
            }
            if remaining_quantity == 0 { break; }
            Self::process_order_queue(&mut fills, &mut remaining_quantity, *ask, order_queue);
        }
        let market_price = fills.iter()
            .map(|(_, fill_price, _)| *fill_price).max().unwrap_or(self.max_bid);
        let fill_result = Self::process_fills(
            remaining_quantity, fills, market_price, order, &mut self.bid_side_book);
        self.update_max_bid();
        fill_result
    }

    fn market_ask_order(&mut self, order: Order) -> FillResult {
        let mut fills = Vec::new();
        let mut remaining_quantity = order.quantity;
        let bids = self.bid_side_book.price_map.iter_mut().rev();
        for (bid, order_queue) in bids {
            if order_queue.is_empty() {
                continue;
            }
            if remaining_quantity == 0 { break; }
            Self::process_order_queue(&mut fills, &mut remaining_quantity, *bid, order_queue);
        }
        let market_price = fills.iter()
            .map(|(_, fill_price, _)| *fill_price).min().unwrap_or(self.min_ask);
        let fill_result = Self::process_fills(
            remaining_quantity, fills, market_price, order, &mut self.ask_side_book);
        self.update_min_ask();
        fill_result
    }

    fn process_order_queue(fills: &mut Vec<(u128, u64, u64)>, remaining_quantity: &mut u64, 
                           book_price: u64, order_queue: &mut VecDeque<Order>) {
        while !order_queue.is_empty() && *remaining_quantity != 0 {
            let book_order = order_queue.front_mut().unwrap();
            if book_order.quantity <= *remaining_quantity {
                fills.push((book_order.id, book_price, book_order.quantity));
                *remaining_quantity -= book_order.quantity;
                order_queue.pop_front();
            } else {
                fills.push((book_order.id, book_price, *remaining_quantity));
                book_order.quantity -= *remaining_quantity;
                *remaining_quantity = 0;
            }
        }
    }
    
    fn process_fills(remaining_quantity: u64, fills: Vec<(u128, u64, u64)>, price: u64, 
                     order: Order, book: &mut PriceBook ) -> FillResult {
        let fill_result;
        if remaining_quantity == order.quantity {
            fill_result = FillResult::Created((order.id, price, remaining_quantity));
            book.insert(price, order);
        } else if remaining_quantity > 0 {
            fill_result = FillResult::PartiallyFilled(fills, (order.id, price, remaining_quantity));
            book.insert(price, Order { id: order.id, quantity: remaining_quantity });
        } else {
            fill_result = FillResult::Filled(fills);
        }
        fill_result
    }

    fn process_order_modification(book: &mut PriceBook, id: u128, price: u64,
                                  new_price: u64, new_quantity: u64) -> ModifyResult {
        if let Some(order_queue) = book.price_map.get_mut(&price) {
            if price == new_price {
                if let Some(order) = order_queue.iter_mut()
                    .find(|o| o.id == id && o.quantity != new_quantity) {
                    order.quantity = new_quantity;
                    return ModifyResult::ModifiedOrder;
                }
            } else if let Some(index) = order_queue.iter().position(|o| o.id == id) {
                order_queue.remove(index);
                return ModifyResult::CreateNewOrder;
            }
        }
        ModifyResult::Unchanged
    }
}

#[cfg(test)]
pub(crate) mod tests {
    use crate::models::Order;
    use crate::orderbook::{ExecutionResult, FillResult, OrderBook, OrderOperation, OrderType, Side};
    use crate::orderrequest::OrderRequest;
    use crate::pricebook::tests::create_test_price_book;

    pub fn create_test_order_book() -> ((u128, u128, u128), (u128, u128, u128), OrderBook) {
        let mut book = OrderBook::default();
        let (ids_bid, bid_side_book) = create_test_price_book(100, 110);
        let (ids_ask, ask_side_book) = create_test_price_book(120, 130);
        book.bid_side_book = bid_side_book;
        book.ask_side_book = ask_side_book;
        (ids_bid, ids_ask, book)
    }

    #[test]
    fn it_cancels_order_when_it_exists() {
        let ((o100i1, ..), _, mut book) = create_test_order_book();
        book.cancel_bid_order(o100i1, 100);
        assert_eq!(book.bid_side_book.get_total_quantity_at_price(&100), 200u64);
    }

    #[test]
    fn it_cancels_nothing_when_order_does_not_exist() {
        let ((o100i1, ..), _, mut book) = create_test_order_book();
        book.cancel_ask_order(o100i1, 130);
        assert_eq!(book.ask_side_book.get_total_quantity_at_price(&130), 300u64);
    }

    #[test]
    fn it_executes_a_limit_bid_that_is_created() {
        let (.., mut book) = create_test_order_book();
        let id = 5;
        let order = Order { id, quantity: 500 };
        let result = book.limit_bid_order(100, order);
        println!("{:#?}", result);
        match result {
            FillResult::Created((order_id, ..)) => assert_eq!(order_id, id),
            _ => panic!("invalid case for test"),
        }
    }

    #[test]
    fn it_executes_a_limit_bid_that_is_filled() {
        let (.., mut book) = create_test_order_book();
        let id = 5;
        let order = Order { id, quantity: 400 };
        let result = book.limit_bid_order(130, order);
        println!("{:#?}", result);
        match result {
            FillResult::Filled(_) => {
                let quantity = book.ask_side_book
                    .get_total_quantity_at_price(&130);
                assert_eq!(quantity, 200); 
            },
            _ => panic!("invalid case for test"),
        }
    }

    #[test]
    fn it_executes_a_limit_bid_that_is_partially_filled() {
        let (.., mut book) = create_test_order_book();
        let id = 5;
        let order = Order { id, quantity: 700 };
        let result = book.limit_bid_order(150, order);
        println!("{:#?}", result);
        match result {
            FillResult::PartiallyFilled(..) => {
                let quantity = book.bid_side_book
                    .get_total_quantity_at_price(&150);
                assert_eq!(quantity, 100);
            },
            _ => panic!("invalid case for test"),
        }
    }

    #[test]
    fn it_executes_a_limit_ask_that_is_created() {
        let (.., mut book) = create_test_order_book();
        let id = 5;
        let order = Order { id, quantity: 250 };
        let result = book.limit_ask_order(120, order);
        println!("{:#?}", result);
        match result {
            FillResult::Created((order_id, ..)) => assert_eq!(order_id, id),
            _ => panic!("invalid case for test")
        }
    }

    #[test]
    fn it_executes_a_limit_ask_that_is_filled() {
        let (.., mut book) = create_test_order_book();
        let id = 5;
        let order = Order { id, quantity: 400 };
        let result = book.limit_ask_order(100, order);
        println!("{:#?}", result);
        match result {
            FillResult::Filled(_) => {
                let quantity = book.bid_side_book
                    .get_total_quantity_at_price(&100);
                assert_eq!(quantity, 200);
            },
            _ => panic!("invalid case for test"),
        }
    }

    #[test]
    fn it_executes_a_limit_ask_that_is_partially_filled() {
        let (.., mut book) = create_test_order_book();
        let id = 5;
        let order = Order { id, quantity: 700 };
        let result = book.limit_ask_order(90, order);
        println!("{:#?}", result);
        match result {
            FillResult::PartiallyFilled(..) => {
                let quantity = book.ask_side_book
                    .get_total_quantity_at_price(&90);
                assert_eq!(quantity, 100);
            },
            _ => panic!("invalid case for test"),
        }
    }

    #[test]
    fn it_modifies_limit_bid_order_quantity() {
        let ((id, ..), _, mut book) = create_test_order_book();
        book.modify_limit_buy_order(id, 100, 150, 100);
        assert_eq!(book.bid_side_book.get_total_quantity_at_price(&100), 350);
    }

    #[test]
    fn it_modifies_limit_ask_order_quantity() {
        let (_, (id, ..), mut book) = create_test_order_book();
        book.modify_limit_ask_order(id, 120, 150, 120);
        assert_eq!(book.ask_side_book.get_total_quantity_at_price(&120), 350);
    }

    #[test]
    fn it_modifies_limit_bid_order_price() {
        let ((id, ..), _, mut book) = create_test_order_book();
        book.modify_limit_buy_order(id, 100, 400, 120);
        let quantity_at_100 = book.bid_side_book.get_total_quantity_at_price(&100);
        let quantity_at_120 = book.bid_side_book.get_total_quantity_at_price(&120);
        assert!(quantity_at_100 == 200 && quantity_at_120 == 100);
    }

    #[test]
    fn it_modifies_limit_ask_order_price() {
        let (_, (id, ..), mut book)  = create_test_order_book();
        book.modify_limit_ask_order(id, 120, 400, 110);
        let quantity_at_120 = book.ask_side_book.get_total_quantity_at_price(&120);
        let quantity_at_110 = book.ask_side_book.get_total_quantity_at_price(&110);
        assert!(quantity_at_120 == 200 && quantity_at_110 == 100);
    }

    #[test]
    fn it_modifies_nothing_when_price_and_quantity_are_same() {
        let ((id, ..), _, mut book) = create_test_order_book();
        book.modify_limit_buy_order(id, 100, 100, 100);
        assert_eq!(book.bid_side_book.get_total_quantity_at_price(&100), 300);
    }

    #[test]
    fn it_executes_a_market_bid_filled() {
        let (.., mut book) = create_test_order_book();
        let order = Order { id: 5, quantity: 500 };
        let result = book.market_bid_order(order);
        println!("{:#?}", result);
        match result {
            FillResult::Filled(..) => {
                let price = 130;
                assert_eq!(book.ask_side_book.get_total_quantity_at_price(&price), 100);
            }
            _ => panic!("invalid case for test"),
        }
    }

    #[test]
    fn it_executes_a_market_ask_filled() {
        let (.., mut book) = create_test_order_book();
        let order = Order { id: 5, quantity: 500 };
        let result = book.market_ask_order(order);
        println!("{:#?}", result);
        match result {
            FillResult::Filled(..) => {
                let price = 100;
                assert_eq!(book.bid_side_book.get_total_quantity_at_price(&price), 100);
            }
            _ => panic!("invalid case for test"),
        }
    }

    #[test]
    fn it_executes_a_market_bid_partially_filled() {
        let (.., mut book) = create_test_order_book();
        let order = Order { id: 5, quantity: 700 };
        let result = book.market_bid_order(order);
        println!("{:#?}", result);
        match result {
            FillResult::PartiallyFilled(..) => {
                let price = 130;
                assert!(book.bid_side_book.get_total_quantity_at_price(&price) == 100
                    && book.ask_side_book.get_total_quantity_at_price(&price) == 0);
            }
            _ => panic!("invalid case for test"),
        }
    }

    #[test]
    fn it_executes_a_market_ask_partially_filled() {
        let (.., mut book) = create_test_order_book();
        let order = Order { id: 5, quantity: 700 };
        let result = book.market_ask_order(order);
        println!("{:#?}", result);
        match result {
            FillResult::PartiallyFilled(..) => {
                let price = 100;
                assert!(book.ask_side_book.get_total_quantity_at_price(&price) == 100
                    && book.bid_side_book.get_total_quantity_at_price(&price) == 0);
            }
            _ => panic!("invalid case for test"),
        }
    }
    
    
    //TODO: complete tests for larger executions
    #[test]
    fn it_executes_a_series_of_orders() {
        let (_, _, mut book) = create_test_order_book();
        let order_request = OrderRequest::new(5,110, 100, Side::Bid, OrderType::Limit);
        let operations = vec![
            OrderOperation::Place(order_request.clone()),
            OrderOperation::Place(OrderRequest::new(6, 110, 200, Side::Ask, OrderType::Market)),
            OrderOperation::Modify(order_request.clone(), 110, 200),
            OrderOperation::Cancel(order_request.clone())
        ];
        for operation in operations {
            match book.execute(operation) {
                ExecutionResult::Executed(result) => {
                    println!("executed: {:#?}", result);
                }
                ExecutionResult::Modified(result) => {
                    match result {
                        Some(fills) => {
                            println!("modified with fills: {:#?}", fills);
                        }
                        None => {
                            println!("modified");
                        }
                    }
                }
                ExecutionResult::Cancelled(result) => {
                    println!("cancelled id: {:#?}", result);
                }
            }
        }
    }

    #[test]
    fn it_shows_stats() {
        let (.., book) = create_test_order_book();
        let stats = book.stats();
        assert!(stats.len() == 4 && stats
            .iter().any(|(p, q, s)| *p == 100 && *q == 300 && *s == Side::Bid));
    }
}