use crate::models::{LimitOrder, Side};
use std::collections::HashMap;
use std::ops::{Index, IndexMut};

#[derive(Debug)]
/// This struct represents a store for our order data.
/// This is done primarily to easily retrieve the order data via a hash map.
/// We also pre-allocate the entire memory needed to store the order data to save reallocation calls.
pub struct Store {
    /// This vector stores all our limit orders.
    orders: Vec<LimitOrder>,
    /// This vector represents the indices of the above vector that are free to use.
    free_indexes: Vec<usize>,
    /// THis map creates a relation between the index on our BTreeMap in the orderbook and the orders vector here.
    order_id_index_map: HashMap<u128, usize>,
}

impl Store {
    /// This is a constructor like method.
    /// Apart from allocate memory, it also pre-populates the data.
    ///
    /// # Arguments
    ///
    /// * `capacity` - Capacity determines the pre-allocated size of the order store.
    ///
    /// # Returns
    ///
    /// * A [`Store`] with the specified capacity.
    pub fn new(capacity: usize) -> Self {
        let mut store = Self {
            orders: Vec::with_capacity(capacity),
            free_indexes: Vec::with_capacity(capacity),
            order_id_index_map: HashMap::with_capacity(capacity),
        };
        for index in 0..capacity {
            let dummy = LimitOrder::new(0, 0, 0, Side::Bid);
            store.orders.push(dummy);
            store.free_indexes.push(index);
        }
        store
    }

    /// This method uses an id to retrieve an immutable reference of limit order along with its index within our store.
    ///
    /// # Arguments
    ///
    /// * `id` - This is the id of the limit order.
    ///
    /// # Returns
    ///
    /// * An optional tuple [`Option<(&LimitOrder, usize)>`], containing a reference to the limit order and its index.
    pub fn get(&self, id: u128) -> Option<(&LimitOrder, usize)> {
        self.order_id_index_map
            .get(&id)
            .map(|index| (&self.orders[*index], *index))
    }

    /// This method uses an id to retrieve a mutable reference of limit order along with its index within our store.
    ///
    /// # Arguments
    ///
    /// * `id` - This is the id of the limit order.
    ///
    /// # Returns
    ///
    /// * An optional tuple [`Option<(&mut LimitOrder, usize)>`], containing a mutable reference to the limit order and its index.
    pub fn get_mut(&mut self, id: u128) -> Option<(&mut LimitOrder, usize)> {
        self.order_id_index_map
            .get_mut(&id)
            .map(|index| (&mut self.orders[*index], *index))
    }

    /// This method inserts a [`LimitOrder`] in our store.
    /// This is done by checking a free index and pushing a new order or modifying an existing order in place to save reallocation calls.
    ///
    /// # Arguments
    ///
    /// * `order` - This is the limit order to be saved in the store.
    ///
    /// # Returns
    ///
    /// * The index of the stored limit order.
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

    /// This method deletes a [`LimitOrder`] in our store by id.
    /// This is done by marking the order quantity 0 and marking its index free.
    ///
    /// # Arguments
    ///
    /// * `id` - This is the id of the limit order to be deleted in our store.
    ///
    /// # Returns
    ///
    /// * A boolean depicting whether the operation successfully deleted an entry
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

/// [`Index`] trait is implemented to get an immutable reference to the [`LimitOrder`] in the orders vector.
impl Index<usize> for Store {
    type Output = LimitOrder;

    /// This method helps us index the store and access the orders vector.
    ///
    /// # Arguments
    ///
    /// * `index` - This is the index of the limit order in the orders vector.
    ///
    /// # Returns
    ///
    /// * An immutable reference `&` to the [`LimitOrder`] in the orders vector.
    #[inline]
    fn index(&self, index: usize) -> &LimitOrder {
        &self.orders[index]
    }
}

/// [`IndexMut`] trait is implemented to get a mutable reference to the [`LimitOrder`] in the orders vector.
impl IndexMut<usize> for Store {
    /// This method helps us mutably index the store and access the orders vector.
    ///
    /// # Arguments
    ///
    /// * `index` - This is the index of the limit order in the orders vector.
    ///
    /// # Returns
    ///
    /// * A mutable reference `&mut` to the [`LimitOrder`] in the orders vector.
    #[inline]
    fn index_mut(&mut self, index: usize) -> &mut LimitOrder {
        &mut self.orders[index]
    }
}
