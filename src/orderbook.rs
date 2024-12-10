use std::collections::{VecDeque};
use qp_trie::Trie;
use uuid::Uuid;

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
    Uninitiated,
    Filled,
    PartiallyFilled,
    Created
}

#[derive(Debug)]
pub struct Order {
    id: Uuid,
    quantity: u64
}

#[derive(Debug)]
pub struct PriceBook {
    book_side: Side,
    price_map: Trie<Vec<u8>, VecDeque<Order>>
}

#[derive(Debug)]
pub struct OrderBook {
    id: Uuid,
    max_bid: Vec<u8>,
    min_ask: Vec<u8>,
    bid_side_book: PriceBook,
    ask_side_book: PriceBook
}

impl PriceBook {
    pub fn new(book_side: Side) -> Self {
        PriceBook {
            book_side,
            price_map: Trie::new()
        }
    }

    pub fn insert(&mut self, price:&Vec<u8>, order: Order) {
        const MAX_ORDERS_AT_PRICE: usize = 50000;

        match self.price_map.get_mut(price) {
            Some(order_queue) => {
                order_queue.push_back(order);
            }
            None => {
                let mut queue = VecDeque::with_capacity(MAX_ORDERS_AT_PRICE);
                queue.push_back(order);
                self.price_map.insert(price.clone(), queue);
            }
        }
    }

    pub fn remove(&mut self, id: &Uuid, price: &Vec<u8>) {
        if let Some(order_queue) = self.price_map.get_mut(price) {
            order_queue.retain(|order| order.id != *id)
        }
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

#[cfg(test)]
mod tests {
    use std::collections::{VecDeque};
    use uuid::Uuid;
    use crate::orderbook::{Order, PriceBook, Side};

    use crate::utils::price_to_bytes;

    fn create_test_price_book() -> ((Uuid, Uuid, Uuid), PriceBook) {
        let mut book = PriceBook::new(Side::Bid);

        let o100i1 = Uuid::new_v4();
        let o100i2 = Uuid::new_v4();
        let o110i3 = Uuid::new_v4();

        let mut orders_100 = VecDeque::with_capacity(50000);
        orders_100.push_back(Order { id: o100i1.clone(), quantity: 100 });
        orders_100.push_back(Order { id: o100i2.clone(), quantity: 150 });
        orders_100.push_back(Order { id: Uuid::new_v4(), quantity: 50 });

        let mut orders_110 = VecDeque::with_capacity(50000);
        orders_110.push_back(Order { id: o110i3.clone(), quantity: 200 });
        orders_110.push_back(Order { id: Uuid::new_v4(), quantity: 100 });

        book.price_map.insert(price_to_bytes(100), orders_100);
        book.price_map.insert(price_to_bytes(110), orders_110);

        ((o100i1, o100i2, o110i3) , book)
    }

    #[test]
    pub fn it_gets_total_quantity_at_price() {
        let (_, book) = create_test_price_book();
        let result = book.get_total_quantity_at_price(&price_to_bytes(100));
        assert_eq!(300, result);
    }

    #[test]
    pub fn it_inserts_order_at_price_when_queue_does_not_exist() {
        let (_, mut book) = create_test_price_book();
        let order = Order { id: Uuid::new_v4(), quantity: 500 };
        let price = price_to_bytes(200);
        book.insert(&price, order);
        assert_eq!(book.get_total_quantity_at_price(&price), 500u64);
    }

    #[test]
    pub fn it_inserts_order_at_price_when_queue_exists() {
        let (_, mut book) = create_test_price_book();
        let price = price_to_bytes(100);
        let order = Order { id: Uuid::new_v4(), quantity: 200 };
        book.insert(&price, order);
        assert_eq!(book.get_total_quantity_at_price(&price), 500u64);
    }

    #[test]
    pub fn it_removes_order_from_price_book_when_it_exists() {
        let ((o100i1,_,_), mut book) = create_test_price_book();
        let price = price_to_bytes(100);
        book.remove(&o100i1, &price);
        assert_eq!(book.get_total_quantity_at_price(&price), 200u64);
    }

    #[test]
    pub fn it_does_nothing_in_price_book_when_order_does_not_exist() {
        let (_, mut book) = create_test_price_book();
        let new_order_id = Uuid::new_v4();
        let price = price_to_bytes(100);
        book.remove(&new_order_id, &price);
        assert_eq!(book.get_total_quantity_at_price(&price), 300u64);
    }

    #[test]
    pub fn it_does_nothing_in_price_book_when_price_does_not_exist() {
        let ((o100i1, _, _), mut book) = create_test_price_book();
        let price = price_to_bytes(500);
        book.remove(&o100i1, &price);
        assert_eq!(book.get_total_quantity_at_price(&price), 0u64);
    }
}