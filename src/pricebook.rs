use std::collections::{BTreeMap, VecDeque};

use crate::models::Order;

#[derive(Debug)]
pub(crate) struct PriceBook {
    pub top_price: Option<u64>,
    pub price_map: BTreeMap<u64, VecDeque<Order>>
}

impl PriceBook {
    pub fn new() -> Self {
        PriceBook {top_price: None, price_map: BTreeMap::new() }
    }

    pub fn insert(&mut self, price: u64, order: Order) {
        self.price_map
            .entry(price)
            .or_insert_with(|| VecDeque::with_capacity(10))
            .push_back(order);
    }

    pub fn remove(&mut self, id: &u128, price: &u64) {
        if let Some(order_queue) = self.price_map.get_mut(price) {
            order_queue.retain(|order| order.id != *id)
        }
    }

    pub fn get_total_quantity_at_price(&self, price: &u64) -> u64 {
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
    use crate::models::{Order, Side};
    use crate::pricebook::PriceBook;

    pub fn create_test_price_book(p0: u64, p1: u64, side: Side) -> ((u128, u128, u128), PriceBook) {
        let mut book = PriceBook::new();
        match side {
            Side::Bid => book.top_price = Some(p1),
            Side::Ask => book.top_price = Some(p0)
        }
        let id1 = 1;
        let id2 = 2;
        let id3 = 3;
        book.price_map.insert(p0, VecDeque::from(vec![
            Order { id: id1, quantity: 100 },
            Order { id: id2, quantity: 150 },
            Order { id: 4, quantity: 50 }
        ]));
        book.price_map.insert(p1, VecDeque::from(vec![
            Order { id: id3, quantity: 200 },
            Order { id: 5, quantity: 100 }
        ]));
        ((id1, id2, id3), book)
    }

    #[test]
    fn it_gets_total_quantity_at_price() {
        let (_, book) = create_test_price_book(100, 110, Side::Bid);
        let result = book.get_total_quantity_at_price(&100);
        assert_eq!(300, result);
    }

    #[test]
    fn it_inserts_order_at_price_when_queue_does_not_exist() {
        let (_, mut book) = create_test_price_book(100, 110, Side::Bid);
        let order = Order { id: 1, quantity: 500 };
        let price = 200;
        book.insert(price, order);
        assert_eq!(book.get_total_quantity_at_price(&price), 500u64);
    }

    #[test]
    fn it_inserts_order_at_price_when_queue_exists() {
        let (_, mut book) = create_test_price_book(100, 110, Side::Bid);
        let price = 100;
        let order = Order { id: 1, quantity: 200 };
        book.insert(price, order);
        assert_eq!(book.get_total_quantity_at_price(&price), 500u64);
    }

    #[test]
    fn it_removes_order_from_price_book_when_it_exists() {
        let ((o100i1, ..), mut book) =
            create_test_price_book(100, 110, Side::Bid);
        let price = 100;
        book.remove(&o100i1, &price);
        assert_eq!(book.get_total_quantity_at_price(&price), 200u64);
    }

    #[test]
    fn it_does_nothing_in_price_book_when_order_does_not_exist() {
        let (_, mut book) = create_test_price_book(100, 110, Side::Bid);
        let new_order_id = 5;
        let price = 100;
        book.remove(&new_order_id, &price);
        assert_eq!(book.get_total_quantity_at_price(&price), 300u64);
    }

    #[test]
    fn it_does_nothing_in_price_book_when_price_does_not_exist() {
        let ((o100i1, ..), mut book) =
            create_test_price_book(100, 110, Side::Bid);
        let price = 500;
        book.remove(&o100i1, &price);
        assert_eq!(book.get_total_quantity_at_price(&price), 0u64);
    }
}