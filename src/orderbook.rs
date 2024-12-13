use std::collections::{BTreeMap, VecDeque};
use crate::models::{
    ExecutionResult, FillResult, ModifyResult, Order, OrderOperation, OrderType, Side
};

#[derive(Debug)]
pub struct OrderBook {
    max_bid: Option<u64>,
    min_ask: Option<u64>,
    bid_side_book: BTreeMap<u64, VecDeque<Order>>,
    ask_side_book: BTreeMap<u64, VecDeque<Order>>
}

impl Default for OrderBook {
    fn default() -> Self {
        Self::new()
    }
}

impl OrderBook {
    pub fn new() -> Self {
        OrderBook {
            max_bid: None,
            min_ask: None,
            bid_side_book: BTreeMap::new(),
            ask_side_book: BTreeMap::new(),
        }
    }
    
    pub fn get_max_bid(&self) -> Option<u64> {
        self.max_bid
    }
    
    pub fn get_min_ask(&self) -> Option<u64> {
        self.min_ask
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
                    Side::Bid => self.cancel_bid_order(order.id, order.price),
                    Side::Ask => self.cancel_ask_order(order.id, order.price)
                }
            }
        }
    }

    pub fn stats(&self) -> Vec<(u64, u64, Side)> {
        let mut orders = vec![];
        for (price, _) in self.bid_side_book.iter() {
            let quantity = Self::get_total_quantity_at_price(&self.bid_side_book, price);
            orders.push((*price, quantity, Side::Bid));
        }
        for (price, _) in self.ask_side_book.iter() {
            let quantity = Self::get_total_quantity_at_price(&self.ask_side_book, price);
            orders.push((*price, quantity, Side::Ask));
        }
        orders
    }
    
    fn cancel_bid_order(&mut self, id: u128, price: u64) -> ExecutionResult {
        match Self::remove_order(&mut self.bid_side_book, &id, &price) {
            true => {
                self.update_max_bid();
                ExecutionResult::Cancelled(id)
            }
            false => ExecutionResult::NoExecution
        }
    }

    fn cancel_ask_order(&mut self, id: u128, price: u64) -> ExecutionResult {
        match Self::remove_order(&mut self.ask_side_book, &id, &price) {
            true => {
                self.update_min_ask();
                ExecutionResult::Cancelled(id)
            }
            false => ExecutionResult::NoExecution
        }
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
        if let Some((price, _)) = self.bid_side_book.iter()
            .filter(|(_, order_queue)| !order_queue.is_empty()).last(){
            self.max_bid = Some(*price);
        }
    }

    fn update_min_ask(&mut self) {
        if let Some((price, _)) = self.ask_side_book.iter().rev()
            .filter(|(_, order_queue)| !order_queue.is_empty()).last() {
            self.min_ask = Some(*price);
        }
    }

    fn limit_bid_order(&mut self, price: u64, order: Order) -> FillResult {
        let mut order_fills = Vec::new();
        let mut remaining_quantity = order.quantity;
        let mut update_min_ask = false;
        for (ask_price, queue) in self.ask_side_book.iter_mut() {
            if update_min_ask {
                self.min_ask = Some(*ask_price);
                update_min_ask = false;
            }
            if queue.is_empty() {
                continue;
            }
            if price < *ask_price {
                break;
            }
            Self::process_queue(&mut order_fills, &mut remaining_quantity, ask_price, queue);
            if remaining_quantity > 0 {
                update_min_ask = true
            }
        }
        self.process_bid_fills(order, order_fills, remaining_quantity, price)
    }

    fn limit_ask_order(&mut self, price: u64, order: Order) -> FillResult {
        let mut order_fills = Vec::new();
        let mut remaining_quantity = order.quantity;
        let mut update_max_bid = false;
        for (bid_price, queue) in self.bid_side_book.iter_mut().rev() {
            if update_max_bid {
                self.max_bid = Some(*bid_price);
                update_max_bid = false;
            }
            if queue.is_empty() {
                continue;
            }
            if price > *bid_price {
                break;
            }
            Self::process_queue(&mut order_fills, &mut remaining_quantity, bid_price, queue);
            if remaining_quantity > 0 {
                update_max_bid = true
            }
        }
        self.process_ask_fills(order, order_fills, remaining_quantity, price)
    }

    fn market_bid_order(&mut self, order: Order) -> FillResult {
        let mut order_fills = Vec::new();
        let mut remaining_quantity = order.quantity;
        let mut update_min_ask = false;
        for (ask_price, queue) in self.ask_side_book.iter_mut() {
            if update_min_ask {
                self.min_ask = Some(*ask_price);
                update_min_ask = false;
            }
            if queue.is_empty() {
                continue;
            }
            Self::process_queue(&mut order_fills, &mut remaining_quantity, ask_price, queue);
            if remaining_quantity > 0 {
                update_min_ask = true
            }
        }
        let price = self.min_ask.unwrap_or(u64::MAX);
        self.process_bid_fills(order, order_fills, remaining_quantity, price)
    }

    fn process_bid_fills(&mut self, order: Order, order_fills: Vec<(u128, u64, u64)>,
                         remaining_quantity: u64, price: u64) -> FillResult {
        if remaining_quantity == order.quantity {
            let id = order.id;
            if price > self.max_bid.unwrap_or(u64::MIN) {
                self.max_bid = Some(price)
            }
            Self::enqueue_order(&mut self.bid_side_book, price, order);
            FillResult::Created((id, price, remaining_quantity))
        } else if remaining_quantity > 0 {
            self.max_bid = Some(price);
            Self::enqueue_order(&mut self.bid_side_book, price,
                                Order { id: order.id, quantity: remaining_quantity });
            FillResult::PartiallyFilled(order_fills, (order.id, price, remaining_quantity))
        } else {
            FillResult::Filled(order_fills)
        }
    }

    fn market_ask_order(&mut self, order: Order) -> FillResult {
        let mut order_fills = Vec::new();
        let mut remaining_quantity = order.quantity;
        let mut update_max_bid = false;
        for (bid_price, queue) in self.bid_side_book.iter_mut().rev() {
            if update_max_bid {
                self.max_bid = Some(*bid_price);
                update_max_bid = false;
            }
            if queue.is_empty() {
                continue;
            }
            Self::process_queue(&mut order_fills, &mut remaining_quantity, bid_price, queue);
            if remaining_quantity > 0 {
                update_max_bid = true
            }
        }
        let price = self.max_bid.unwrap_or(u64::MIN);
        self.process_ask_fills(order, order_fills, remaining_quantity, price)
    }

    fn process_ask_fills(&mut self, order: Order, order_fills: Vec<(u128, u64, u64)>, 
        remaining_quantity: u64, price: u64) -> FillResult {
        if remaining_quantity == order.quantity {
            let id = order.id;
            if price < self.min_ask.unwrap_or(u64::MAX) {
                self.min_ask = Some(price)
            }
            Self::enqueue_order(&mut self.ask_side_book, price, order);
            FillResult::Created((id, price, remaining_quantity))
        } else if remaining_quantity > 0 {
            self.min_ask = Some(price);
            Self::enqueue_order(&mut self.ask_side_book, price,
                                Order { id: order.id, quantity: remaining_quantity });
            FillResult::PartiallyFilled(order_fills, (order.id, price, remaining_quantity))
        } else {
            FillResult::Filled(order_fills)
        }
    }

    fn process_queue(order_fills: &mut Vec<(u128, u64, u64)>, remaining_quantity: &mut u64,
                     ask_price: &u64, queue: &mut VecDeque<Order>) {
        while let Some(front_order) = queue.front_mut() {
            if *remaining_quantity == 0 {
                break;
            }
            if front_order.quantity > *remaining_quantity {
                front_order.quantity -= *remaining_quantity;
                order_fills.push((front_order.id, *ask_price, *remaining_quantity));
                *remaining_quantity = 0;
            } else {
                *remaining_quantity -= front_order.quantity;
                order_fills.push((front_order.id, *ask_price, front_order.quantity));
                queue.pop_front();
            }
        }
    }

    fn process_order_modification(book: &mut BTreeMap<u64, VecDeque<Order>>, id: u128, price: u64,
                                  new_price: u64, new_quantity: u64) -> ModifyResult {
        if let Some(order_queue) = book.get_mut(&price) {
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

    fn get_total_quantity_at_price(book: &BTreeMap<u64, VecDeque<Order>>, price: &u64) -> u64 {
        match book.get(price) {
            Some(orders) => {
                orders.iter().map(|o| o.quantity).sum()
            }
            None => 0
        }
    }

    fn enqueue_order(book: &mut BTreeMap<u64, VecDeque<Order>>, price: u64, order: Order) {
        book.entry(price).or_insert_with(|| VecDeque::with_capacity(10)).push_back(order);
    }

    fn remove_order(book: &mut BTreeMap<u64, VecDeque<Order>>, id: &u128, price: &u64) -> bool {
        match book.get_mut(price) {
            Some(order_queue) => {
                order_queue.retain(|order| order.id != *id);
                true
            },
            None => false
        }
    }
}

#[cfg(test)]
pub(crate) mod tests {
    use crate::models::{ExecutionResult, Order, OrderOperation, OrderRequest, OrderType};
    use crate::orderbook::{ FillResult, OrderBook, Side};

    fn create_orderbook() -> OrderBook {
        let mut book = OrderBook::new();
        let orders = vec![
            OrderRequest::new(1, 100, 100, Side::Bid, OrderType::Limit),
            OrderRequest::new(2, 100, 150, Side::Bid, OrderType::Limit),
            OrderRequest::new(3, 100, 50, Side::Bid, OrderType::Limit),
            OrderRequest::new(4, 110, 200, Side::Bid, OrderType::Limit),
            OrderRequest::new(5, 110, 100, Side::Bid, OrderType::Limit),
            OrderRequest::new(6, 120, 100, Side::Ask, OrderType::Limit),
            OrderRequest::new(7, 120, 150, Side::Ask, OrderType::Limit),
            OrderRequest::new(8, 120, 50, Side::Ask, OrderType::Limit),
            OrderRequest::new(9, 130, 200, Side::Ask, OrderType::Limit),
            OrderRequest::new(10, 130, 100, Side::Ask, OrderType::Limit),
        ];
        for order in orders {
            book.execute(OrderOperation::Place(order));
        }
        book
    }

    fn fills_to_ids(fills: Vec<(u128, u64, u64)>) -> Vec<u128> {
        fills.iter().map(|f| f.0).collect()
    }


    #[test]
    fn it_gets_total_quantity_at_price() {
        let book = create_orderbook();
        let result =  OrderBook::get_total_quantity_at_price(&book.bid_side_book, &100);
        assert_eq!(300, result);
    }

    #[test]
    fn it_inserts_order_at_price_when_queue_does_not_exist() {
        let mut book = create_orderbook();
        let order = Order { id: 11, quantity: 500 };
        OrderBook::enqueue_order(&mut book.bid_side_book, 200, order);
        assert_eq!(OrderBook::get_total_quantity_at_price(&book.bid_side_book, &200), 500);
    }

    #[test]
    fn it_inserts_order_at_price_when_queue_exists() {
        let mut book = create_orderbook();
        let order = Order { id: 11, quantity: 200 };
        OrderBook::enqueue_order(&mut book.bid_side_book, 100, order);
        assert_eq!(OrderBook::get_total_quantity_at_price(&book.bid_side_book, &100), 500);
    }

    #[test]
    fn it_removes_order_from_price_book_when_it_exists() {
        let mut book = create_orderbook();
        OrderBook::remove_order(&mut book.bid_side_book, &1, &100);
        assert_eq!(OrderBook::get_total_quantity_at_price(&book.bid_side_book, &100), 200);
    }

    #[test]
    fn it_does_nothing_in_price_book_when_order_does_not_exist() {
        let mut book = create_orderbook();
        OrderBook::remove_order(&mut book.bid_side_book, &11, &100);
        assert_eq!(OrderBook::get_total_quantity_at_price(&book.bid_side_book, &100), 300);
    }

    #[test]
    fn it_does_nothing_in_price_book_when_price_does_not_exist() {
        let mut book = create_orderbook();
        OrderBook::remove_order(&mut book.bid_side_book, &1, &200);
        assert_eq!(OrderBook::get_total_quantity_at_price(&book.bid_side_book, &200), 0);
    }
    
    #[test]
    fn it_cancels_order_when_it_exists() {
        let mut book = create_orderbook();
        book.execute(OrderOperation::Place(
            OrderRequest::new(11, 115, 100, Side::Bid, OrderType::Limit)));
        match book.cancel_bid_order(11, 115) {
            ExecutionResult::Cancelled(id) => {
                assert!(id == 11 && book.get_max_bid() == Some(110))
            },
            _ => panic!("test failed")
        }
    }

    #[test]
    fn it_cancels_nothing_when_order_does_not_exist() {
        let mut book = create_orderbook();
        match book.cancel_bid_order(11, 115) {
            ExecutionResult::NoExecution => {
                assert_eq!(book.get_max_bid(), Some(110))
            },
            _ => panic!("test failed")
        }
    }

    #[test]
    fn it_executes_a_limit_bid_that_is_created() {
        let mut book = create_orderbook();
        let order = Order { id: 11, quantity: 500 };
        let result = book.limit_bid_order(100, order);
        match result {
            FillResult::Created((order_id, ..)) => assert_eq!(order_id, 11),
            _ => panic!("test failed"),
        }
    }

    #[test]
    fn it_executes_a_limit_bid_that_is_filled() {
        let mut book = create_orderbook();
        let order = Order { id: 11, quantity: 400 };
        match book.limit_bid_order(130, order) {
            FillResult::Filled(order_fills) => {
                let quantity = OrderBook::get_total_quantity_at_price(
                    &book.ask_side_book,&130);
                assert!(fills_to_ids(order_fills) == vec![6, 7, 8, 9] && quantity == 200); 
            },
            _ => panic!("test failed"),
        }
    }

    #[test]
    fn it_executes_a_limit_bid_that_is_partially_filled() {
        let mut book = create_orderbook();
        let order = Order { id: 11, quantity: 700 };
        match book.limit_bid_order(150, order) {
            FillResult::PartiallyFilled(order_fills, order_placed) => {
                assert!(fills_to_ids(order_fills) == vec![6, 7, 8, 9, 10] 
                    && order_placed == (11, 150, 100));
            },
            _ => panic!("invalid case for test"),
        }
    }

    #[test]
    fn it_executes_a_limit_ask_that_is_created() {
        let mut book = create_orderbook();
        let order = Order { id: 11, quantity: 250 };
        match book.limit_ask_order(120, order) {
            FillResult::Created((order_id, ..)) => assert_eq!(order_id, 11),
            _ => panic!("test failed"),
        }
    }

    #[test]
    fn it_executes_a_limit_ask_that_is_filled() {
        let mut book = create_orderbook();
        let order = Order { id: 11, quantity: 400 };
        match book.limit_ask_order(100, order) {
            FillResult::Filled(order_fills) => {
                let quantity = OrderBook::get_total_quantity_at_price(
                    &book.bid_side_book, &100);
                assert!(fills_to_ids(order_fills) == vec![4, 5, 1] && quantity == 200);
            },
            _ => panic!("test failed"),
        }
    }

    #[test]
    fn it_executes_a_limit_ask_that_is_partially_filled() {
        let mut book = create_orderbook();
        let order = Order { id: 11, quantity: 700 };
        match book.limit_ask_order(90, order) {
            FillResult::PartiallyFilled(order_fills, order_placed) => {
                assert!(fills_to_ids(order_fills) == vec![4, 5, 1, 2, 3] 
                    && order_placed == (11, 90, 100));
            },
            _ => panic!("test failed"),
        }
    }

    #[test]
    fn it_modifies_limit_bid_order_quantity() {
        let mut book = create_orderbook();
        book.modify_limit_buy_order(1, 100, 150, 100);
        assert_eq!(OrderBook::get_total_quantity_at_price(&book.bid_side_book, &100), 350);
    }

    #[test]
    fn it_modifies_limit_ask_order_quantity() {
        let mut book = create_orderbook();
        book.modify_limit_ask_order(6, 120, 150, 120);
        assert_eq!(OrderBook::get_total_quantity_at_price(&book.ask_side_book, &120), 350);
    }

    #[test]
    fn it_modifies_limit_bid_order_price() {
        let mut book = create_orderbook();
        book.modify_limit_buy_order(1, 100, 400, 120);
        let quantity_at_100 = OrderBook::get_total_quantity_at_price(
            &book.bid_side_book, &100);
        let quantity_at_120 = OrderBook::get_total_quantity_at_price(
            &book.bid_side_book, &120);
        assert!(quantity_at_100 == 200 && quantity_at_120 == 100);
    }

    #[test]
    fn it_modifies_limit_ask_order_price() {
        let mut book  = create_orderbook();
        book.modify_limit_ask_order(6, 120, 400, 110);
        let quantity_at_120 = OrderBook::get_total_quantity_at_price(
            &book.ask_side_book, &120);
        let quantity_at_110 = OrderBook::get_total_quantity_at_price(
            &book.ask_side_book, &110);
        assert!(quantity_at_120 == 200 && quantity_at_110 == 100);
    }

    #[test]
    fn it_modifies_nothing_when_price_and_quantity_are_same() {
        let mut book = create_orderbook();
        book.modify_limit_buy_order(1, 100, 100, 100);
        assert_eq!(OrderBook::get_total_quantity_at_price(&book.bid_side_book, &100), 300);
    }

    #[test]
    fn it_executes_a_market_bid_filled() {
        let mut book = create_orderbook();
        let order = Order { id: 11, quantity: 500 };
        match book.market_bid_order(order) {
            FillResult::Filled(order_fills) => {
                let quantity = OrderBook::get_total_quantity_at_price(
                    &book.ask_side_book,&130);
                assert!(fills_to_ids(order_fills) == vec![6, 7, 8, 9] && quantity == 100);
            },
            _ => panic!("test failed"),
        }
    }

    #[test]
    fn it_executes_a_market_ask_filled() {
        let mut book = create_orderbook();
        let order = Order { id: 5, quantity: 500 };
        match book.market_ask_order(order) {
            FillResult::Filled(order_fills) => {
                let quantity = OrderBook::get_total_quantity_at_price(
                    &book.bid_side_book, &100);
                assert!(fills_to_ids(order_fills) == vec![4, 5, 1, 2] && quantity == 100);
            },
            _ => panic!("test failed"),
        }
    }

    #[test]
    fn it_executes_a_market_bid_partially_filled() {
        let mut book = create_orderbook();
        let order = Order { id: 11, quantity: 700 };
        let result = book.market_bid_order(order);
        println!("{:#?}", result);
        match result {
            FillResult::PartiallyFilled(order_fills, order_placed) => {
                assert!(fills_to_ids(order_fills) == vec![6, 7, 8, 9, 10] 
                    && order_placed == (11, 130, 100));
            },
            _ => panic!("test failed"),
        }
    }

    #[test]
    fn it_executes_a_market_ask_partially_filled() {
        let mut book = create_orderbook();
        let order = Order { id: 11, quantity: 700 };
        match book.market_ask_order(order) {
            FillResult::PartiallyFilled(order_fills, order_placed) => {
                assert!(fills_to_ids(order_fills) == vec![4, 5, 1, 2, 3] 
                    && order_placed == (11, 100, 100));
            },
            _ => panic!("test failed"),
        }
    }

    #[test]
    fn it_updates_top_price_when_bid_is_created() {
        let mut book = create_orderbook();
        let order = Order { id: 11, quantity: 500 };
        book.limit_bid_order(115, order);
        match book.max_bid {
            Some(price) => assert_eq!(price, 115),
            None => panic!("test failed"),
        }
    }
    
    #[test]
    fn it_updates_top_price_when_ask_is_created() {
        let mut book = create_orderbook();
        let order = Order { id: 11, quantity: 500 };
        book.limit_ask_order(115, order);
        match book.min_ask {
            Some(price) => assert_eq!(price, 115),
            None => panic!("test failed"),
        }
    }

    #[test]
    fn it_updates_top_price_when_bid_is_filled() {
        let mut book = create_orderbook();
        let order = Order { id: 5, quantity: 500 };
        book.limit_bid_order(130, order);
        match book.min_ask {
            Some(price) => assert_eq!(price, 130),
            None => panic!("test failed"),
        }
    }

    #[test]
    fn it_updates_top_price_when_ask_is_filled() {
        let mut book = create_orderbook();
        let order = Order { id: 5, quantity: 500 };
        book.limit_ask_order(100, order);
        match book.max_bid {
            Some(price) => assert_eq!(price, 100),
            None => panic!("test failed"),
        }
    }

    #[test]
    fn it_updates_top_price_when_bid_is_partially_filled() {
        let mut book = create_orderbook();
        let order = Order { id: 5, quantity: 700 };
        book.limit_bid_order(130, order);
        assert!(book.min_ask == Some(130) 
            && book.max_bid == Some(130))
    }

    #[test]
    fn it_updates_top_price_when_ask_is_partially_filled() {
        let mut book = create_orderbook();
        let order = Order { id: 5, quantity: 700 };
        book.limit_ask_order(100, order);
        assert!(book.max_bid == Some(100)
            && book.min_ask == Some(100))
    }

    #[test]
    fn it_shows_stats() {
        let book = create_orderbook();
        let stats = book.stats();
        assert!(stats.len() == 4 && stats
            .iter().any(|(p, q, s)| *p == 100 && *q == 300 && *s == Side::Bid));
    }

    #[test]
    fn it_gets_max_bid() {
        let book = create_orderbook();
        let max_bid = book.get_max_bid();
        assert_eq!(max_bid, Some(110));
    }

    #[test]
    fn it_gets_min_ask() {
        let book = create_orderbook();
        let min_ask = book.get_min_ask();
        assert_eq!(min_ask, Some(120));
    }

    #[test]
    fn it_returns_none_for_empty_get_max_bid() {
        let book = OrderBook::new();
        let max_bid = book.get_max_bid();
        assert_eq!(max_bid, None);
    }

    #[test]
    fn it_returns_none_for_empty_get_min_ask() {
        let book = OrderBook::new();
        let min_ask = book.get_min_ask();
        assert_eq!(min_ask, None);
    }
}