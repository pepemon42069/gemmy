use std::collections::{VecDeque};
use qp_trie::Trie;
use uuid::Uuid;
use crate::utils::{bytes_to_price, price_to_bytes};

#[derive(Debug)]
pub enum Side {
    Bid,
    Ask
}

#[derive(Debug)]
pub enum OrderType {
    Limit,
    Market
}

#[derive(Debug)]
pub enum FillResult {
    Filled(Vec<(Uuid, u64, u64)>),
    PartiallyFilled(Vec<(Uuid, u64, u64)>, (Uuid, u64, u64)),
    Created((Uuid, u64, u64))
}

#[derive(Debug)]
pub enum ModifyResult {
    CreateNewOrder,
    ModifiedOrder,
    Unchanged
}

#[derive(Debug)]
pub struct Order {
    id: Uuid,
    quantity: u64
}

#[derive(Debug)]
pub struct PriceBook {
    price_map: Trie<Vec<u8>, VecDeque<Order>>
}

#[derive(Debug)]
pub struct OrderBook {
    pub max_bid: u64,
    pub min_ask: u64,
    pub bid_side_book: PriceBook,
    pub ask_side_book: PriceBook
}

impl PriceBook {
    pub fn new() -> Self {
        PriceBook { price_map: Trie::new() }
    }

    pub fn insert(&mut self, price:Vec<u8>, order: Order) {
        const MAX_ORDERS_AT_PRICE: usize = 50000;

        match self.price_map.get_mut(&price) {
            Some(order_queue) => {
                order_queue.push_back(order);
            }
            None => {
                let mut queue = VecDeque::with_capacity(MAX_ORDERS_AT_PRICE);
                queue.push_back(order);
                self.price_map.insert(price, queue);
            }
        }
    }

    pub fn remove(&mut self, id: &Uuid, price: &Vec<u8>) {
        if let Some(order_queue) = self.price_map.get_mut(price) {
            order_queue.retain(|order| order.id != *id)
        }
    }

    pub fn get_ne_prices_u64(&self) -> Vec<u64> {
        self.price_map.iter()
            .filter(|(_, v)| !v.is_empty())
            .map(|(k, _)| bytes_to_price(k.clone())).collect()
    }

    pub fn get_ne_asc_prices_u64(&self) -> Vec<u64> {
        let mut prices: Vec<u64> = self.get_ne_prices_u64();
        prices.sort();
        prices
    }

    pub fn get_ne_desc_prices_u64(&self) -> Vec<u64> {
        let mut prices: Vec<u64> = self.get_ne_prices_u64();
        prices.sort_by(|a, b| b.cmp(a));
        prices
    }

    pub fn get_total_quantity_at_price(&self, price: &Vec<u8>) -> u64 {
        match self.price_map.get(price) {
            Some(orders) => {
                orders.iter().map(|o| o.quantity).sum()
            }
            None => 0
        }
    }
}

impl OrderBook {
    pub fn new() -> Self {
        OrderBook {
            max_bid: u64::MAX,
            min_ask: u64::MIN,
            bid_side_book: PriceBook::new(),
            ask_side_book: PriceBook::new(),
        }
    }
    
    pub fn cancel_bid_order(&mut self, id: &Uuid, price: u64) {
        self.bid_side_book.remove(id, &price_to_bytes(price));
    }

    pub fn cancel_ask_order(&mut self, id: &Uuid, price: u64) {
        self.ask_side_book.remove(id, &price_to_bytes(price));
    }

    pub fn modify_limit_buy_order(
        &mut self, id: Uuid, price: u64, new_quantity: u64, new_price: u64) {
        let result = Self::process_order_modification(
            &mut self.bid_side_book, id, price, new_quantity, new_price);
        match result {
            ModifyResult::CreateNewOrder => {
                self.limit_bid_order(new_price, Order {id, quantity: new_quantity });
            }
            ModifyResult::ModifiedOrder => {
                self.update_max_bid()
            }
            _ => ()
        }
    }

    pub fn modify_limit_ask_order(
        &mut self, id: Uuid, price: u64, new_quantity: u64, new_price: u64) {
        let result = Self::process_order_modification(
            &mut self.ask_side_book, id, price, new_quantity, new_price);
        match result {
            ModifyResult::CreateNewOrder => {
                self.limit_ask_order(new_price, Order {id, quantity: new_quantity });
            }
            ModifyResult::ModifiedOrder => {
                self.update_min_ask()
            }
            _ => ()
        }
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

    pub fn update_max_bid(&mut self) {
        let bid_prices = self.bid_side_book.get_ne_desc_prices_u64();
        if let Some(max_bid) = bid_prices.first() {
            self.max_bid = *max_bid;
        }
    }

    pub fn update_min_ask(&mut self) {
        let ask_prices = self.ask_side_book.get_ne_asc_prices_u64();
        if let Some(min_ask) = ask_prices.first() {
            self.min_ask = *min_ask;
        }
    }

    pub fn limit_bid_order(&mut self, price: u64, order: Order) -> FillResult {
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

    pub fn limit_ask_order(&mut self, price: u64, order: Order) -> FillResult {
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

    pub fn market_bid_order(&mut self, order: Order) -> FillResult {
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

    pub fn market_ask_order(&mut self, order: Order) -> FillResult {
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
}

#[cfg(test)]
mod tests {
    use std::collections::{VecDeque};
    use uuid::Uuid;
    use crate::orderbook::{FillResult, Order, OrderBook, PriceBook};

    use crate::utils::price_to_bytes;

    fn create_bid_side_book() -> ((Uuid, Uuid, Uuid), PriceBook) {
        let mut book = PriceBook::new();

        let o100i1 = Uuid::new_v4();
        let o100i2 = Uuid::new_v4();
        let o110i3 = Uuid::new_v4();

        let mut orders_100 = VecDeque::with_capacity(50000);
        orders_100.push_back(Order { id: o100i1, quantity: 100 });
        orders_100.push_back(Order { id: o100i2, quantity: 150 });
        orders_100.push_back(Order { id: Uuid::new_v4(), quantity: 50 });

        let mut orders_110 = VecDeque::with_capacity(50000);
        orders_110.push_back(Order { id: o110i3, quantity: 200 });
        orders_110.push_back(Order { id: Uuid::new_v4(), quantity: 100 });

        book.price_map.insert(price_to_bytes(100), orders_100);
        book.price_map.insert(price_to_bytes(110), orders_110);

        ((o100i1, o100i2, o110i3) , book)
    }

    fn create_ask_side_book() -> ((Uuid, Uuid, Uuid), PriceBook) {
        let mut book = PriceBook::new();

        let o120i1 = Uuid::new_v4();
        let o120i2 = Uuid::new_v4();
        let o130i3 = Uuid::new_v4();

        let mut orders_120 = VecDeque::with_capacity(50000);
        orders_120.push_back(Order { id: o120i1, quantity: 100 });
        orders_120.push_back(Order { id: o120i2, quantity: 150 });
        orders_120.push_back(Order { id: Uuid::new_v4(), quantity: 50 });

        let mut orders_130 = VecDeque::with_capacity(50000);
        orders_130.push_back(Order { id: o130i3, quantity: 200 });
        orders_130.push_back(Order { id: Uuid::new_v4(), quantity: 100 });

        book.price_map.insert(price_to_bytes(120), orders_120);
        book.price_map.insert(price_to_bytes(130), orders_130);

        ((o120i1, o120i2, o130i3) , book)
    }

    fn create_test_order_book() -> ((Uuid, Uuid, Uuid), (Uuid, Uuid, Uuid), OrderBook) {
        let mut book = OrderBook::new();
        let (ids_bid, bid_side_book) = create_bid_side_book();
        let (ids_ask, ask_side_book) = create_ask_side_book();
        book.bid_side_book = bid_side_book;
        book.ask_side_book = ask_side_book;
        (ids_bid, ids_ask, book)
    }

    #[test]
    fn it_gets_total_quantity_at_price() {
        let (_, book) = create_bid_side_book();
        let result = book.get_total_quantity_at_price(&price_to_bytes(100));
        assert_eq!(300, result);
    }

    #[test]
    fn it_inserts_order_at_price_when_queue_does_not_exist() {
        let (_, mut book) = create_bid_side_book();
        let order = Order { id: Uuid::new_v4(), quantity: 500 };
        let price = price_to_bytes(200);
        book.insert(price.clone(), order);
        assert_eq!(book.get_total_quantity_at_price(&price), 500u64);
    }

    #[test]
    fn it_inserts_order_at_price_when_queue_exists() {
        let (_, mut book) = create_bid_side_book();
        let price = price_to_bytes(100);
        let order = Order { id: Uuid::new_v4(), quantity: 200 };
        book.insert(price.clone(), order);
        assert_eq!(book.get_total_quantity_at_price(&price), 500u64);
    }

    #[test]
    fn it_removes_order_from_price_book_when_it_exists() {
        let ((o100i1, ..), mut book) = create_bid_side_book();
        let price = price_to_bytes(100);
        book.remove(&o100i1, &price);
        assert_eq!(book.get_total_quantity_at_price(&price), 200u64);
    }

    #[test]
    fn it_does_nothing_in_price_book_when_order_does_not_exist() {
        let (_, mut book) = create_bid_side_book();
        let new_order_id = Uuid::new_v4();
        let price = price_to_bytes(100);
        book.remove(&new_order_id, &price);
        assert_eq!(book.get_total_quantity_at_price(&price), 300u64);
    }

    #[test]
    fn it_does_nothing_in_price_book_when_price_does_not_exist() {
        let ((o100i1, ..), mut book) = create_bid_side_book();
        let price = price_to_bytes(500);
        book.remove(&o100i1, &price);
        assert_eq!(book.get_total_quantity_at_price(&price), 0u64);
    }

    #[test]
    fn it_cancels_order_when_it_exists() {
        let ((o100i1, ..), _, mut book) = create_test_order_book();
        book.cancel_bid_order(&o100i1, 100);
        assert_eq!(book.bid_side_book.get_total_quantity_at_price(&price_to_bytes(100)), 200u64);
    }

    #[test]
    fn it_cancels_nothing_when_order_does_not_exist() {
        let ((o100i1, ..), _, mut book) = create_test_order_book();
        book.cancel_ask_order(&o100i1, 130);
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
}