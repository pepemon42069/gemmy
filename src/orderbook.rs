use uuid::Uuid;
use crate::models::{ExecutionResult, FillResult, ModifyResult, Order, OrderOperation, OrderType, Side};
use crate::pricebook::PriceBook;
use crate::utils::{bytes_to_price, price_to_bytes};


#[derive(Debug)]
pub struct OrderBook {
    pub id: Uuid,
    pub max_bid: u64,
    pub min_ask: u64,
    bid_side_book: PriceBook,
    ask_side_book: PriceBook
}

impl Default for OrderBook {
    fn default() -> Self {
        Self::new()
    }
}

impl OrderBook {
    pub fn new() -> Self {
        OrderBook {
            id: Uuid::new_v4(),
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
            orders.push((bytes_to_price(price.clone()), quantity, Side::Bid));
        }
        let ask_side_map = &self.ask_side_book.price_map;
        for (price, _) in ask_side_map.iter() {
            let quantity = self.ask_side_book.get_total_quantity_at_price(price);
            orders.push((bytes_to_price(price.clone()), quantity, Side::Ask));
        }
        orders
    }
    
    fn cancel_bid_order(&mut self, id: Uuid, price: u64) {
        self.bid_side_book.remove(&id, &price_to_bytes(price));
        self.update_max_bid();
    }

    fn cancel_ask_order(&mut self, id: Uuid, price: u64) {
        self.ask_side_book.remove(&id, &price_to_bytes(price));
        self.update_min_ask();
    }

    fn modify_limit_buy_order(
        &mut self, id: Uuid, price: u64, new_quantity: u64, new_price: u64) -> Option<FillResult> {
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
        &mut self, id: Uuid, price: u64, new_quantity: u64, new_price: u64) -> Option<FillResult> {
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
        let bid_prices = self.bid_side_book.get_ne_desc_prices_u64();
        if let Some(max_bid) = bid_prices.first() {
            self.max_bid = *max_bid;
        }
    }

    fn update_min_ask(&mut self) {
        let ask_prices = self.ask_side_book.get_ne_asc_prices_u64();
        if let Some(min_ask) = ask_prices.first() {
            self.min_ask = *min_ask;
        }
    }

    fn limit_bid_order(&mut self, price: u64, order: Order) -> FillResult {
        let mut fills = Vec::new();
        let mut remaining_quantity = order.quantity;
        let ask_prices = self.ask_side_book.get_ne_asc_prices_u64();
        for ask in ask_prices {
            if price < ask { break; }
            Self::process_order_queue(
                &mut fills, &mut remaining_quantity, ask, &mut self.ask_side_book);
        }
        let fill_result = Self::process_fills(
            remaining_quantity, fills, price, order, &mut self.bid_side_book);
        self.update_max_bid();
        fill_result
    }

    fn limit_ask_order(&mut self, price: u64, order: Order) -> FillResult {
        let mut fills = Vec::new();
        let mut remaining_quantity = order.quantity;
        let bid_prices = self.bid_side_book.get_ne_desc_prices_u64();
        for bid in bid_prices {
            if price > bid { break; }
            Self::process_order_queue(
                &mut fills, &mut remaining_quantity, bid, &mut self.bid_side_book);
        }
        let fill_result = Self::process_fills(
            remaining_quantity, fills, price, order, &mut self.ask_side_book);
        self.update_min_ask();
        fill_result
    }

    fn market_bid_order(&mut self, order: Order) -> FillResult {
        let mut fills = Vec::new();
        let mut remaining_quantity = order.quantity;
        let ask_prices = self.ask_side_book.get_ne_asc_prices_u64();
        for ask in ask_prices {
            if remaining_quantity == 0 { break; }
            Self::process_order_queue(
                &mut fills, &mut remaining_quantity, ask, &mut self.ask_side_book);
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
        let bid_prices = self.bid_side_book.get_ne_desc_prices_u64();
        for bid in bid_prices {
            if remaining_quantity == 0 { break; }
            Self::process_order_queue(
                &mut fills, &mut remaining_quantity, bid, &mut self.bid_side_book);
        }
        let market_price =  fills.iter()
            .map(|(_, fill_price, _)| *fill_price).min().unwrap_or(self.max_bid);
        let fill_result = Self::process_fills(
            remaining_quantity, fills, market_price, order, &mut self.ask_side_book);
        self.update_min_ask();
        fill_result
    }

    fn process_order_queue(fills: &mut Vec<(Uuid, u64, u64)>, remaining_quantity: &mut u64, 
                           book_price: u64, book: &mut PriceBook) {
        let key = price_to_bytes(book_price);
        if let Some(order_queue) = book.price_map.get_mut(&key) {
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
    }
    
    fn process_fills(remaining_quantity: u64, fills: Vec<(Uuid, u64, u64)>, price: u64, 
                     order: Order, book: &mut PriceBook ) -> FillResult {
        let fill_result;
        if remaining_quantity == 0 {
            fill_result = FillResult::Filled(fills);
        } else if remaining_quantity == order.quantity {
            fill_result = FillResult::Created((order.id, price, remaining_quantity));
            book.insert(price_to_bytes(price), order);
        } else {
            fill_result = FillResult::PartiallyFilled(fills, (order.id, price, remaining_quantity));
            book.insert(price_to_bytes(price), Order { 
                id: order.id, quantity: remaining_quantity });
        }
        fill_result
    }

    fn process_order_modification(book: &mut PriceBook, id: Uuid, price: u64,
                                  new_price: u64, new_quantity: u64) -> ModifyResult {
        let key = price_to_bytes(price);
        if let Some(order_queue) = book.price_map.get_mut(&key) {
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
    use uuid::Uuid;
    use crate::models::Order;
    use crate::orderbook::{ExecutionResult, FillResult, OrderBook, OrderOperation, OrderType, Side};
    use crate::orderrequest::OrderRequest;
    use crate::pricebook::tests::create_test_price_book;
    use crate::utils::price_to_bytes;

    pub fn create_test_order_book() -> ((Uuid, Uuid, Uuid), (Uuid, Uuid, Uuid), OrderBook) {
        let mut book = OrderBook::new();
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
        assert_eq!(book.bid_side_book.get_total_quantity_at_price(&price_to_bytes(100)), 200u64);
    }

    #[test]
    fn it_cancels_nothing_when_order_does_not_exist() {
        let ((o100i1, ..), _, mut book) = create_test_order_book();
        book.cancel_ask_order(o100i1, 130);
        assert_eq!(book.ask_side_book.get_total_quantity_at_price(&price_to_bytes(130)), 300u64);
    }

    #[test]
    fn it_executes_a_limit_bid_that_is_created() {
        let (.., mut book) = create_test_order_book();
        let id = Uuid::new_v4();
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
        let id = Uuid::new_v4();
        let order = Order { id, quantity: 400 };
        let result = book.limit_bid_order(130, order);
        println!("{:#?}", result);
        match result {
            FillResult::Filled(_) => {
                let quantity = book.ask_side_book
                    .get_total_quantity_at_price(&price_to_bytes(130));
                assert_eq!(quantity, 200); 
            },
            _ => panic!("invalid case for test"),
        }
    }

    #[test]
    fn it_executes_a_limit_bid_that_is_partially_filled() {
        let (.., mut book) = create_test_order_book();
        let id = Uuid::new_v4();
        let order = Order { id, quantity: 700 };
        let result = book.limit_bid_order(150, order);
        println!("{:#?}", result);
        match result {
            FillResult::PartiallyFilled(..) => {
                let quantity = book.bid_side_book
                    .get_total_quantity_at_price(&price_to_bytes(150));
                assert_eq!(quantity, 100);
            },
            _ => panic!("invalid case for test"),
        }
    }

    #[test]
    fn it_executes_a_limit_ask_that_is_created() {
        let (.., mut book) = create_test_order_book();
        let id = Uuid::new_v4();
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
        let id = Uuid::new_v4();
        let order = Order { id, quantity: 400 };
        let result = book.limit_ask_order(100, order);
        println!("{:#?}", result);
        match result {
            FillResult::Filled(_) => {
                let quantity = book.bid_side_book
                    .get_total_quantity_at_price(&price_to_bytes(100));
                assert_eq!(quantity, 200);
            },
            _ => panic!("invalid case for test"),
        }
    }

    #[test]
    fn it_executes_a_limit_ask_that_is_partially_filled() {
        let (.., mut book) = create_test_order_book();
        let id = Uuid::new_v4();
        let order = Order { id, quantity: 700 };
        let result = book.limit_ask_order(90, order);
        println!("{:#?}", result);
        match result {
            FillResult::PartiallyFilled(..) => {
                let quantity = book.ask_side_book
                    .get_total_quantity_at_price(&price_to_bytes(90));
                assert_eq!(quantity, 100);
            },
            _ => panic!("invalid case for test"),
        }
    }

    #[test]
    fn it_modifies_limit_bid_order_quantity() {
        let ((id, ..), _, mut book) = create_test_order_book();
        book.modify_limit_buy_order(id, 100, 150, 100);
        assert_eq!(book.bid_side_book.get_total_quantity_at_price(&price_to_bytes(100)), 350);
    }

    #[test]
    fn it_modifies_limit_ask_order_quantity() {
        let (_, (id, ..), mut book) = create_test_order_book();
        book.modify_limit_ask_order(id, 120, 150, 120);
        assert_eq!(book.ask_side_book.get_total_quantity_at_price(&price_to_bytes(120)), 350);
    }

    #[test]
    fn it_modifies_limit_bid_order_price() {
        let ((id, ..), _, mut book) = create_test_order_book();
        book.modify_limit_buy_order(id, 100, 400, 120);
        let quantity_at_100 = book.bid_side_book.get_total_quantity_at_price(
            &price_to_bytes(100));
        let quantity_at_120 = book.bid_side_book.get_total_quantity_at_price(
            &price_to_bytes(120));
        assert!(quantity_at_100 == 200 && quantity_at_120 == 100);
    }

    #[test]
    fn it_modifies_limit_ask_order_price() {
        let (_, (id, ..), mut book)  = create_test_order_book();
        book.modify_limit_ask_order(id, 120, 400, 110);
        let quantity_at_120 = book.ask_side_book.get_total_quantity_at_price(
            &price_to_bytes(120));
        let quantity_at_110 = book.ask_side_book.get_total_quantity_at_price(
            &price_to_bytes(110));
        assert!(quantity_at_120 == 200 && quantity_at_110 == 100);
    }

    #[test]
    fn it_modifies_nothing_when_price_and_quantity_are_same() {
        let ((id, ..), _, mut book) = create_test_order_book();
        book.modify_limit_buy_order(id, 100, 100, 100);
        assert_eq!(book.bid_side_book.get_total_quantity_at_price(&price_to_bytes(100)), 300);
    }

    #[test]
    fn it_executes_a_market_bid_filled() {
        let (.., mut book) = create_test_order_book();
        let order = Order { id: Uuid::new_v4(), quantity: 500 };
        let result = book.market_bid_order(order);
        println!("{:#?}", result);
        match result {
            FillResult::Filled(..) => {
                let price = price_to_bytes(130);
                assert_eq!(book.ask_side_book.get_total_quantity_at_price(&price), 100);
            }
            _ => panic!("invalid case for test"),
        }
    }

    #[test]
    fn it_executes_a_market_ask_filled() {
        let (.., mut book) = create_test_order_book();
        let order = Order { id: Uuid::new_v4(), quantity: 500 };
        let result = book.market_ask_order(order);
        println!("{:#?}", result);
        match result {
            FillResult::Filled(..) => {
                let price = price_to_bytes(100);
                assert_eq!(book.bid_side_book.get_total_quantity_at_price(&price), 100);
            }
            _ => panic!("invalid case for test"),
        }
    }

    #[test]
    fn it_executes_a_market_bid_partially_filled() {
        let (.., mut book) = create_test_order_book();
        let order = Order { id: Uuid::new_v4(), quantity: 700 };
        let result = book.market_bid_order(order);
        println!("{:#?}", result);
        match result {
            FillResult::PartiallyFilled(..) => {
                let price = price_to_bytes(130);
                assert!(book.bid_side_book.get_total_quantity_at_price(&price) == 100
                    && book.ask_side_book.get_total_quantity_at_price(&price) == 0);
            }
            _ => panic!("invalid case for test"),
        }
    }

    #[test]
    fn it_executes_a_market_ask_partially_filled() {
        let (.., mut book) = create_test_order_book();
        let order = Order { id: Uuid::new_v4(), quantity: 700 };
        let result = book.market_ask_order(order);
        println!("{:#?}", result);
        match result {
            FillResult::PartiallyFilled(..) => {
                let price = price_to_bytes(100);
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
        let order_request = OrderRequest::new(110, 100, Side::Bid, OrderType::Limit);
        let operations = vec![
            OrderOperation::Place(order_request.clone()),
            OrderOperation::Place(OrderRequest::new(110, 200, Side::Ask, OrderType::Market)),
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