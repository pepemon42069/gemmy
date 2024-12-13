use std::collections::HashMap;
use std::ops::{Index, IndexMut};
use crate::models::{LimitOrder, Side};

#[derive(Debug)]
pub struct Store {
    orders: Vec<LimitOrder>,
    free_indexes: Vec<usize>,
    order_id_index_map: HashMap<u128, usize>,
}

impl Store {
    pub fn new(capacity: usize) -> Self {
        let mut store = Self {
            orders: Vec::with_capacity(capacity),
            free_indexes: Vec::with_capacity(capacity),
            order_id_index_map: HashMap::with_capacity(capacity)
        };
        for index in 0..capacity {
            let dummy = LimitOrder::new(0, 0, 0, Side::Bid);
            store.orders.push(dummy);
            store.free_indexes.push(index);
        }
        store
    }

    pub fn get(&self, id: u128) -> Option<(&LimitOrder, usize)> {
        self.order_id_index_map.get(&id)
            .map(|index| (&self.orders[*index], *index))
    }
    
    pub fn get_mut(&mut self, id: u128) -> Option<(&mut LimitOrder, usize)> {
        self.order_id_index_map.get_mut(&id)
            .map(|index| (&mut self.orders[*index], *index))
    }

    pub fn insert(&mut self, order: LimitOrder) -> usize {
        match self.free_indexes.pop() {
            None => {
                self.orders.push(order);
                let index = self.orders.len() - 1;
                self.order_id_index_map.insert(order.id, index);
                index
            }
            Some(index) => {
                let existing = &mut self.orders[index];
                existing.id = order.id;
                existing.quantity = order.quantity;
                existing.price = order.price;
                existing.side = order.side;
                self.order_id_index_map.insert(order.id, index);
                index
            }
        }
    }

    pub fn delete(&mut self, id: &u128) -> bool {
        if let Some(index) = self.order_id_index_map.remove(id) {
            if let Some(order) = self.orders.get_mut(index) {
                self.free_indexes.push(index);
                order.quantity = 0;
                return true;
            }
        }
        false
    }
}

impl Index<usize> for Store {
    type Output = LimitOrder;

    #[inline]
    fn index(&self, index: usize) -> &LimitOrder {
        &self.orders[index]
    }
}

impl IndexMut<usize> for Store {
    #[inline]
    fn index_mut(&mut self, index: usize) -> &mut LimitOrder {
        &mut self.orders[index]
    }
}