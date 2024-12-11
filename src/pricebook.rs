use std::collections::VecDeque;
use qp_trie::Trie;
use uuid::Uuid;

use crate::models::Order;
use crate::utils::bytes_to_price;

#[derive(Debug)]
pub(crate) struct PriceBook {
    pub price_map: Trie<Vec<u8>, VecDeque<Order>>
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

#[cfg(test)]
pub(crate) mod tests {
    use std::collections::VecDeque;
    use uuid::Uuid;
    use crate::models::Order;
    use crate::pricebook::PriceBook;
    use crate::utils::price_to_bytes;

    pub fn create_test_price_book(p0: u64, p1: u64) -> ((Uuid, Uuid, Uuid), PriceBook) {
        let mut book = PriceBook::new();
        let id1 = Uuid::new_v4();
        let id2 = Uuid::new_v4();
        let id3 = Uuid::new_v4();
        book.price_map.insert(price_to_bytes(p0), VecDeque::from(vec![
            Order { id: id1, quantity: 100 },
            Order { id: id2, quantity: 150 },
            Order { id: Uuid::new_v4(), quantity: 50 }
        ]));
        book.price_map.insert(price_to_bytes(p1), VecDeque::from(vec![
            Order { id: id3, quantity: 200 },
            Order { id: Uuid::new_v4(), quantity: 100 }
        ]));
        ((id1, id2, id3), book)
    }

    #[test]
    fn it_gets_total_quantity_at_price() {
        let (_, book) = create_test_price_book(100, 110);
        let result = book.get_total_quantity_at_price(&price_to_bytes(100));
        assert_eq!(300, result);
    }

    #[test]
    fn it_inserts_order_at_price_when_queue_does_not_exist() {
        let (_, mut book) = create_test_price_book(100, 110);
        let order = Order { id: Uuid::new_v4(), quantity: 500 };
        let price = price_to_bytes(200);
        book.insert(price.clone(), order);
        assert_eq!(book.get_total_quantity_at_price(&price), 500u64);
    }

    #[test]
    fn it_inserts_order_at_price_when_queue_exists() {
        let (_, mut book) = create_test_price_book(100, 110);
        let price = price_to_bytes(100);
        let order = Order { id: Uuid::new_v4(), quantity: 200 };
        book.insert(price.clone(), order);
        assert_eq!(book.get_total_quantity_at_price(&price), 500u64);
    }

    #[test]
    fn it_removes_order_from_price_book_when_it_exists() {
        let ((o100i1, ..), mut book) = create_test_price_book(100, 110);
        let price = price_to_bytes(100);
        book.remove(&o100i1, &price);
        assert_eq!(book.get_total_quantity_at_price(&price), 200u64);
    }

    #[test]
    fn it_does_nothing_in_price_book_when_order_does_not_exist() {
        let (_, mut book) = create_test_price_book(100, 110);
        let new_order_id = Uuid::new_v4();
        let price = price_to_bytes(100);
        book.remove(&new_order_id, &price);
        assert_eq!(book.get_total_quantity_at_price(&price), 300u64);
    }

    #[test]
    fn it_does_nothing_in_price_book_when_price_does_not_exist() {
        let ((o100i1, ..), mut book) = create_test_price_book(100, 110);
        let price = price_to_bytes(500);
        book.remove(&o100i1, &price);
        assert_eq!(book.get_total_quantity_at_price(&price), 0u64);
    }
}