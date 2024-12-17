use super::{
    models::{
        ProtoBuf, Depth, ExecutionResult, FillMetaData, FillResult, Level, LimitOrder, MarketOrder,
        ModifyResult, Operation, ProtoBufResult, Side
    },
    store::Store
};
use std::collections::{BTreeMap, VecDeque};
use std::ops::{Index, IndexMut};
use uuid::Uuid;
use crate::core::models::RfqStatus;

/// This is the core structure that is used to create an orderbook.
/// It stores all limit order data in the form of a two BTreeMaps, each representing either side of the orderbook.
/// The keys are prices and leaves of the tree are vector dequeues containing indices to the limit orders in store.
/// This struct also contains the store itself, along with some metadata such as queue capacity, etc.
#[derive(Debug, Clone)]
pub struct OrderBook {
    /// A unique id assigned to the orderbook on creation. (uniqueness is not enforced in code)
    id: u128,
    /// Maximum bid at any given time in the orderbook.
    /// This is `None`, upon creation and is populated as soon as the first order enters the book.
    /// Unwrapping in codebase should default to `u64::MIN`
    max_bid: Option<u64>,
    /// Minimum ask at any given time in the orderbook.
    /// This is `None`, upon creation and is populated as soon as the first order enters the book.
    /// Unwrapping in codebase should defaults to `u64::MAX`
    min_ask: Option<u64>,
    /// This represents the bid side order book.
    bid_side_book: BTreeMap<u64, VecDeque<usize>>,
    /// This represents the ask side order book.
    ask_side_book: BTreeMap<u64, VecDeque<usize>>,
    /// A minimum allocation capacity for vector dequeues
    queue_capacity: usize,
    /// The store for all orders.
    order_store: Store
}

/// This assigns the default values for vector dequeue capacity as well as the store capacity when constructing the orderbook.
impl Default for OrderBook {
    /// A constructor like method that allocates default values to the orderbook.
    ///
    /// # Returns
    ///
    /// * An [`OrderBook`] with `DEFAULT_QUEUE_CAPACITY` and `DEFAULT_STORE_CAPACITY`.
    fn default() -> Self {
        const DEFAULT_QUEUE_CAPACITY: usize = 10;
        const DEFAULT_STORE_CAPACITY: usize = 10000;

        Self::new(DEFAULT_QUEUE_CAPACITY, DEFAULT_STORE_CAPACITY)
    }
}

impl OrderBook {
    /// This is a constructor like method.
    ///
    /// # Arguments
    ///
    /// * `queue_capacity` - This is the pre-allocated size of vector dequeues containing indices of orders in the BTreeMap leaves.
    /// * `store_capacity` - This is the pre-allocated size of the order store.
    ///
    /// # Returns
    ///
    /// * An [`OrderBook`] with the specified capacities, and a `Uuid::new_v4()` based id.
    pub fn new(queue_capacity: usize, store_capacity: usize) -> Self {
        OrderBook {
            id: Uuid::new_v4().as_u128(),
            max_bid: None,
            min_ask: None,
            bid_side_book: BTreeMap::new(),
            ask_side_book: BTreeMap::new(),
            order_store: Store::new(store_capacity),
            queue_capacity,
        }
    }

    /// This helps us get the orderbook id
    ///
    /// # Returns
    ///
    /// * A `u128` orderbook id.
    pub fn get_id(&self) -> u128 {
        self.id
    }

    /// This helps us get the maximum value of the bid side orderbook.
    ///
    /// # Returns
    ///
    /// * An `Option<u64>` with the maximum value of the bid side orderbook.
    pub fn get_max_bid(&self) -> Option<u64> {
        self.max_bid
    }

    /// This helps us get the minimum value of the ask side orderbook.
    ///
    /// # Returns
    ///
    /// * An `Option<u64>` with the minimum value of ask bid side orderbook.
    pub fn get_min_ask(&self) -> Option<u64> {
        self.min_ask
    }

    /// This method is used to execute an [`Operation`] on the orderbook.
    /// The flow of this method is dictated by the operation provided, leading to an [`ExecutionResult`].
    ///
    /// *Rules of flow:*
    /// - A limit/market operation leads to `Executed(Filled/PartiallyFilled/Created)` states on success and to `Failed` otherwise.
    /// - A modification operation leads to `Executed(Modified/Created)` states on success and to `Failed` otherwise.
    /// - A cancel operation leads to `Cancelled(u128)` state on success and to `Failed` otherwise.
    ///
    /// Check out the individual enums [`FillResult`], [`FillMetaData`] and [`ModifyResult`] for more details.
    ///
    /// # Arguments
    ///
    /// * `operation` - This can be one of four different types, [`Operation::Limit`], [`Operation::Market`], [`Operation::Modify`], [`Operation::Cancel`].
    ///
    /// # Returns
    ///
    /// * [`ExecutionResult`] that depicts the status of execution of the operation.
    pub fn execute(&mut self, operation: Operation) -> ExecutionResult {
        match operation {
            Operation::Limit(order) => match order.side {
                Side::Bid => ExecutionResult::Executed(self.limit_bid_order(order)),
                Side::Ask => ExecutionResult::Executed(self.limit_ask_order(order)),
            },
            Operation::Market(order) => match order.side {
                Side::Bid => {
                    let result = self.market_bid_order(order);
                    match result {
                        FillResult::Failed => {
                            ExecutionResult::Failed("placed market order on empty book".to_string())
                        }
                        _ => ExecutionResult::Executed(result),
                    }
                }
                Side::Ask => {
                    let result = self.market_ask_order(order);
                    match result {
                        FillResult::Failed => {
                            ExecutionResult::Failed("placed market order on empty book".to_string())
                        }
                        _ => ExecutionResult::Executed(result),
                    }
                }
            },
            Operation::Modify(order) => match order.side {
                Side::Bid => match self.modify_limit_buy_order(order) {
                    ModifyResult::Failed => {
                        ExecutionResult::Failed("no modification occurred".to_string())
                    }
                    result => ExecutionResult::Modified(result),
                },
                Side::Ask => match self.modify_limit_ask_order(order) {
                    ModifyResult::Failed => {
                        ExecutionResult::Failed("no modification occurred".to_string())
                    }
                    result => ExecutionResult::Modified(result),
                },
            },
            Operation::Cancel(id) => match self.cancel_order(id) {
                None => ExecutionResult::Failed("order not found".to_string()),
                Some(id) => ExecutionResult::Cancelled(id),
            },
        }
    }
    
    pub fn execute_proto(&mut self, operation: Operation) -> ProtoBufResult {
        self.execute(operation).to_protobuf()
    }

    /// This method returns the depth of the orderbook upto specified levels.
    ///
    /// # Arguments
    ///
    /// * `levels` - This represents the levels of depth the orderbook data needs to be aggregated and provided.
    ///     For example. level = 2 will give top two prices and aggregated quantities on both sides of the orderbook.
    ///
    /// # Returns
    ///
    /// * A [`Depth`] with both bid/ask side price and quantity aggregations for specified `levels`.
    pub fn depth(&self, levels: usize) -> Depth {
        Depth {
            levels,
            bids: Self::get_order_levels(levels, &self.bid_side_book, &self.order_store),
            asks: Self::get_order_levels(levels, &self.ask_side_book, &self.order_store),
        }
    }

    /// This is an internal method used to cancel an existing order.
    ///
    /// # Arguments
    ///
    /// * `id` - This represents the id of the limit order to be cancelled.
    ///
    /// # Returns
    ///
    /// * The same id as an optional value. None is returned if it didn't exist.
    fn cancel_order(&mut self, id: u128) -> Option<u128> {
        match self.order_store.get(id) {
            Some((order, index)) => {
                match order.side {
                    Side::Bid => {
                        let mut bids = self.bid_side_book.iter().rev();
                        let first = bids.next();
                        if let Some((price, queue)) = first {
                            if order.price == *price && queue.len() == 1 {
                                if let Some((price, _)) = bids.next() {
                                    self.max_bid = Some(*price);
                                }
                            }
                        }
                        if let Some(order_queue) = self.bid_side_book.get_mut(&order.price) {
                            order_queue.retain(|i| index != *i);
                        }
                    }
                    Side::Ask => {
                        let mut asks = self.ask_side_book.iter();
                        let first = asks.next();
                        if let Some((price, queue)) = first {
                            if order.price == *price && queue.len() == 1 {
                                if let Some((price, _)) = asks.next() {
                                    self.min_ask = Some(*price);
                                }
                            }
                        }
                        if let Some(order_queue) = self.ask_side_book.get_mut(&order.price) {
                            order_queue.retain(|i| index != *i);
                        }
                    }
                }
                self.order_store.delete(&id);
                Some(id)
            }
            None => None,
        }
    }

    /// This is an internal method used to modify an existing bid order.
    ///
    /// # Arguments
    ///
    /// * `order` - This represents the [`LimitOrder`] to be cancelled.
    ///
    /// # Returns
    ///
    /// * A [`ModifyResult`] depicting whether an order was modified in place, created anew or the operation failed.
    fn modify_limit_buy_order(&mut self, order: LimitOrder) -> ModifyResult {
        if let Some((existing_order, index)) = self.order_store.get_mut(order.id) {
            if let Some(order_queue) = self.bid_side_book.get_mut(&existing_order.price) {
                if let Some(position) = order_queue.iter().position(|i| index == *i) {
                    if existing_order.price != order.price {
                        order_queue.remove(position);
                        self.order_store.delete(&order.id);
                        return ModifyResult::Created(self.limit_bid_order(order));
                    }
                    if existing_order.quantity != order.quantity {
                        existing_order.quantity = order.quantity;
                        return ModifyResult::Modified(order.id);
                    }
                }
            }
        }
        ModifyResult::Failed
    }

    /// This is an internal method used to modify an existing ask order.
    ///
    /// # Arguments
    ///
    /// * `order` - This represents the [`LimitOrder`] to be cancelled.
    ///
    /// # Returns
    ///
    /// * A [`ModifyResult`] depicting whether an order was modified in place, created anew or the operation failed.
    fn modify_limit_ask_order(&mut self, order: LimitOrder) -> ModifyResult {
        if let Some((existing_order, index)) = self.order_store.get_mut(order.id) {
            if let Some(order_queue) = self.ask_side_book.get_mut(&existing_order.price) {
                if let Some(position) = order_queue.iter().position(|i| index == *i) {
                    if existing_order.price != order.price {
                        order_queue.remove(position);
                        self.order_store.delete(&order.id);
                        return ModifyResult::Created(self.limit_ask_order(order));
                    }
                    if existing_order.quantity != order.quantity {
                        existing_order.quantity = order.quantity;
                        return ModifyResult::Modified(order.id);
                    }
                }
            }
        }
        ModifyResult::Failed
    }

    /// This is an internal method used to place a limit bid order.
    ///
    /// *Algorithm:*
    /// - start matching from the top of the book till the limit price exceeds top of the book or the quantity is extinguished.
    /// - skip empty levels
    /// - update min_ask if a partial fill takes place on a specific level.
    /// - fill price queues as per its algorithm
    /// - process resultant fills as per its algorithm
    /// # Arguments
    ///
    /// * `order` - This represents the [`LimitOrder`] to be placed.
    ///
    /// # Returns
    ///
    /// * A [`FillResult`] depicting whether an order was:
    ///     - Fully filled with a resultant vector containing this [`FillMetaData`] generated in order matching.
    ///     - Partially filled with a [`LimitOrder`] being placed with *remaining* quantity and a vector containing this [`FillMetaData`].
    ///     - Created, returning a [`LimitOrder`] with no fills.
    fn limit_bid_order(&mut self, order: LimitOrder) -> FillResult {
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
            if order.price < *ask_price {
                break;
            }
            Self::process_queue_limit(
                &order.id,
                ask_price,
                order.side,
                &mut remaining_quantity,
                queue,
                &mut self.order_store,
                &mut order_fills,
            );
            if remaining_quantity > 0 {
                update_min_ask = true
            }
        }
        self.process_bid_fills(order, order_fills, remaining_quantity)
    }

    /// This is an internal method used to place a limit ask order.
    ///
    /// *Algorithm:*
    /// - start matching from the top of the book till the limit price exceeds top of the book or the quantity is extinguished.
    /// - skip empty levels
    /// - update max_bid if a partial fill takes place on a specific level.
    /// - fill price queues as per its algorithm
    /// - process resultant fills as per its algorithm
    ///
    /// # Arguments
    ///
    /// * `order` - This represents the [`LimitOrder`] to be placed.
    ///
    /// # Returns
    ///
    /// * A [`FillResult`] depicting whether an order was:
    ///     - Fully filled with a resultant vector containing this [`FillMetaData`] generated in order matching.
    ///     - Partially filled with a [`LimitOrder`] being placed with *remaining* quantity and a vector containing this [`FillMetaData`].
    ///     - Created, returning a [`LimitOrder`] with no fills.
    fn limit_ask_order(&mut self, order: LimitOrder) -> FillResult {
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
            if order.price > *bid_price {
                break;
            }
            Self::process_queue_limit(
                &order.id,
                bid_price,
                order.side,
                &mut remaining_quantity,
                queue,
                &mut self.order_store,
                &mut order_fills,
            );
            if remaining_quantity > 0 {
                update_max_bid = true
            }
        }
        self.process_ask_fills(order, order_fills, remaining_quantity)
    }

    /// This is an internal method used to place a market bid order.
    ///
    /// *Algorithm:*
    /// - start matching from the top of the book till the book extinguishes or the quantity.
    /// - if book is empty, disallow operation
    /// - skip empty levels
    /// - update min_ask if a partial fill takes place on a specific level.
    /// - fill price queues as per its algorithm
    /// - before processing fills, if quantity still remains, convert it to limit order at last min_ask
    /// - process resultant fills as per its algorithm
    ///
    /// # Arguments
    ///
    /// * `order` - This represents the [`MarketOrder`] to be placed.
    ///
    /// # Returns
    ///
    /// * A [`FillResult`] depicting whether an order was:
    ///     - Fully filled with a resultant vector containing this [`FillMetaData`] generated in order matching.
    ///     - Partially filled with a [`LimitOrder`] being placed with *remaining* quantity and a vector containing this [`FillMetaData`].
    fn market_bid_order(&mut self, order: MarketOrder) -> FillResult {
        let mut order_fills = Vec::new();
        let mut remaining_quantity = order.quantity;
        let mut update_min_ask = false;

        if self.min_ask.is_none() {
            return FillResult::Failed;
        }

        for (ask_price, queue) in self.ask_side_book.iter_mut() {
            if update_min_ask {
                self.min_ask = Some(*ask_price);
                update_min_ask = false;
            }
            if queue.is_empty() {
                continue;
            }
            Self::process_queue_limit(
                &order.id,
                ask_price,
                order.side,
                &mut remaining_quantity,
                queue,
                &mut self.order_store,
                &mut order_fills,
            );
            if remaining_quantity > 0 {
                update_min_ask = true
            }
        }
        let order = order.to_limit(self.min_ask.unwrap_or(u64::MAX));
        self.process_bid_fills(order, order_fills, remaining_quantity)
    }

    /// This is an internal method used to process the fills generated by a limit/market bid order.
    ///
    /// *Algorithm:*
    /// - If remaining quantity remains unchanged, insert in queue and store. Return created order.
    /// - If some quantity remains, match as a limit order at highest price. Return both created order and fills.
    /// - If no quantity remains, mark the order filled. Return fills.
    ///
    /// # Arguments
    ///
    /// * `order` - This represents a limit order received or constructed in the caller method.
    /// * `order_fills` - This represents the vector containing data of order matching.
    /// * `remaining_quantity` - This represents the quantity left in the order post order matching.
    ///
    /// # Returns
    ///
    /// * A [`FillResult`] depicting whether an order was:
    ///     - Fully filled with a resultant vector containing this [`FillMetaData`] generated in order matching.
    ///     - Partially filled with a [`LimitOrder`] being placed with *remaining* quantity and a vector containing this [`FillMetaData`].
    fn process_bid_fills(
        &mut self,
        mut order: LimitOrder,
        order_fills: Vec<FillMetaData>,
        remaining_quantity: u64,
    ) -> FillResult {
        if remaining_quantity == order.quantity {
            if order.price > self.max_bid.unwrap_or(u64::MIN) {
                self.max_bid = Some(order.price)
            }
            let index = self.order_store.insert(order);
            self.bid_side_book
                .entry(order.price)
                .or_insert_with(|| VecDeque::with_capacity(self.queue_capacity))
                .push_back(index);
            FillResult::Created(order)
        } else if remaining_quantity > 0 {
            self.max_bid = Some(order.price);
            order.update_order_quantity(remaining_quantity);
            let index = self.order_store.insert(order);
            self.bid_side_book
                .entry(order.price)
                .or_insert_with(|| VecDeque::with_capacity(self.queue_capacity))
                .push_back(index);
            FillResult::PartiallyFilled(order, order_fills)
        } else {
            FillResult::Filled(order_fills)
        }
    }

    /// This is an internal method used to place a market ask order.
    ///
    /// *Algorithm:*
    /// - start matching from the top of the book till the book extinguishes or the quantity.
    /// - if book is empty, disallow operation
    /// - skip empty levels
    /// - update max_bid if a partial fill takes place on a specific level.
    /// - fill price queues as per its algorithm
    /// - before processing fills, if quantity still remains, convert it to limit order at last max_bid
    /// - process resultant fills as per its algorithm
    ///
    /// # Arguments
    ///
    /// * `order` - This represents the [`MarketOrder`] to be placed.
    ///
    /// # Returns
    ///
    /// * A [`FillResult`] depicting whether an order was:
    ///     - Fully filled with a resultant vector containing this [`FillMetaData`] generated in order matching.
    ///     - Partially filled with a [`LimitOrder`] being placed with *remaining* quantity and a vector containing this [`FillMetaData`].
    fn market_ask_order(&mut self, order: MarketOrder) -> FillResult {
        let mut order_fills = Vec::new();
        let mut remaining_quantity = order.quantity;
        let mut update_max_bid = false;

        if self.max_bid.is_none() {
            return FillResult::Failed;
        }

        for (bid_price, queue) in self.bid_side_book.iter_mut().rev() {
            if update_max_bid {
                self.max_bid = Some(*bid_price);
                update_max_bid = false;
            }
            if queue.is_empty() {
                continue;
            }
            Self::process_queue_limit(
                &order.id,
                bid_price,
                order.side,
                &mut remaining_quantity,
                queue,
                &mut self.order_store,
                &mut order_fills,
            );
            if remaining_quantity > 0 {
                update_max_bid = true
            }
        }
        let order = order.to_limit(self.max_bid.unwrap_or(u64::MIN));
        self.process_ask_fills(order, order_fills, remaining_quantity)
    }

    /// This is an internal method used to process the fills generated by a limit/market ask order.
    /// *Algorithm:*
    /// - If remaining quantity remains unchanged, insert in queue and store. Return created order.
    /// - If some quantity remains, match as a limit order at highest price. Return both created order and fills.
    /// - If no quantity remains, mark the order filled. Return fills.
    ///
    /// # Arguments
    ///
    /// * `order` - This represents a limit order received or constructed in the caller method.
    /// * `order_fills` - This represents the vector containing data of order matching.
    /// * `remaining_quantity` - This represents the quantity left in the order post order matching.
    ///
    /// # Returns
    ///
    /// * A [`FillResult`] depicting whether an order was:
    ///     - Fully filled with a resultant vector containing this [`FillMetaData`] generated in order matching.
    ///     - Partially filled with a [`LimitOrder`] being placed with *remaining* quantity and a vector containing this [`FillMetaData`].
    fn process_ask_fills(
        &mut self,
        mut order: LimitOrder,
        order_fills: Vec<FillMetaData>,
        remaining_quantity: u64,
    ) -> FillResult {
        if remaining_quantity == order.quantity {
            if order.price < self.min_ask.unwrap_or(u64::MAX) {
                self.min_ask = Some(order.price)
            }
            let index = self.order_store.insert(order);
            self.ask_side_book
                .entry(order.price)
                .or_insert_with(|| VecDeque::with_capacity(self.queue_capacity))
                .push_back(index);
            FillResult::Created(order)
        } else if remaining_quantity > 0 {
            self.min_ask = Some(order.price);
            order.update_order_quantity(remaining_quantity);
            let index = self.order_store.insert(order);
            self.ask_side_book
                .entry(order.price)
                .or_insert_with(|| VecDeque::with_capacity(self.queue_capacity))
                .push_back(index);
            FillResult::PartiallyFilled(order, order_fills)
        } else {
            FillResult::Filled(order_fills)
        }
    }

    /// This is an internal method used to process the queue of orders at a particular price.
    /// Whenever a limit or a market order starts matching, this method is used to pop orders against the quantity in the order.
    /// *Algorithm:*
    /// - Dequeue each front index at a price.
    /// - Get its order details, from store.
    /// - If it has enough quantity, modify in place. Else, pop and update store.
    /// - Repeat till queue is empty or no quantity remains to be filled.
    ///
    /// # Arguments
    ///
    /// * `id` - Original order id, used fore store operations.
    /// * `price` - The current price being processed from the top of the book.
    /// * `side` - The side of the taker.
    /// * `remaining_quantity` - The quantity left in the original order to be matched.
    /// * `queue` - The current(price) order queue to fill the order that has been placed.
    /// * `store` - The order store.
    /// * `order_fills` - This represents each match that takes place across the entire matching process.
    ///
    /// # Returns
    ///
    /// * A resultant vector containing [`FillMetaData`] generated in order matching.
    fn process_queue_limit(
        id: &u128,
        price: &u64,
        side: Side,
        remaining_quantity: &mut u64,
        queue: &mut VecDeque<usize>,
        store: &mut Store,
        order_fills: &mut Vec<FillMetaData>,
    ) {
        while let Some(front_order_index) = queue.front() {
            if *remaining_quantity == 0 {
                break;
            }
            let front_order_data = store.index_mut(*front_order_index);
            if front_order_data.quantity > *remaining_quantity {
                front_order_data.quantity -= *remaining_quantity;
                order_fills.push(FillMetaData {
                    order_id: *id,
                    matched_order_id: front_order_data.id,
                    taker_side: side,
                    price: *price,
                    quantity: *remaining_quantity,
                });
                *remaining_quantity = 0;
            } else {
                *remaining_quantity -= front_order_data.quantity;
                order_fills.push(FillMetaData {
                    order_id: *id,
                    matched_order_id: front_order_data.id,
                    taker_side: side,
                    price: *price,
                    quantity: front_order_data.quantity,
                });
                let id = front_order_data.id;
                store.delete(&id);
                queue.pop_front();
            }
        }
    }

    /// This is an internal helper method used to aggregate quantity at prices going down the top of the book
    ///
    /// # Arguments
    ///
    /// * `levels` - The levels we go on either direction to aggregate quantity.
    /// * `book` - The bid/ask side orderbook we process.
    /// * `store` - The order store.
    ///
    /// # Returns
    ///
    /// * A vector containing [`Level`], i.e. price and aggregated quantity.
    fn get_order_levels(
        levels: usize,
        book: &BTreeMap<u64, VecDeque<usize>>,
        store: &Store,
    ) -> Vec<Level> {
        let mut orders = Vec::with_capacity(levels);
        book.iter().take(levels).for_each(|(price, queue)| {
            orders.push(Level {
                price: *price,
                quantity: queue.iter().map(|index| store.index(*index).quantity).sum(),
            });
        });
        orders
    }
    
    fn process_price(
        amount_spent: &mut u64,
        remaining_quantity: &mut u64,
        price: &u64,
        orders: &VecDeque<usize>, 
        store: &Store
    ) {
        let total_quantity: u64 = orders.iter()
            .map(|index| store.index(*index).quantity).sum();
        if total_quantity <= *remaining_quantity {
            *amount_spent += *price * total_quantity;
            *remaining_quantity -= total_quantity;
        } else {
            *amount_spent += *price * *remaining_quantity;
            *remaining_quantity = 0;
        }
    }
    
    fn process_remaining_quantity(
        amount_spent: u64,
        remaining_quantity: u64,
        original_quantity: u64,
        top_price: u64
    ) -> RfqStatus {
        if remaining_quantity == original_quantity {
            RfqStatus::ConvertToLimit(top_price, original_quantity)
        } else if remaining_quantity == 0 {
            RfqStatus::CompleteFill(amount_spent / original_quantity)
        } else {
            RfqStatus::PartialFillAndLimitPlaced(amount_spent / (original_quantity - remaining_quantity),remaining_quantity)
        }
    }
    
    pub fn request_for_quote(&self, market_order: MarketOrder) -> RfqStatus {
        let quantity = market_order.quantity;
        println!("details: {:#?} {:?} {:?}", market_order, self.min_ask, self.max_bid);
        if quantity == 0 { return RfqStatus::NotPossible; }
        match market_order.side {
            Side::Bid => {
                let min_ask = match self.min_ask {
                    Some(ask) => ask,
                    None => return RfqStatus::NotPossible
                };
                let book = &self.ask_side_book;
                let mut remaining_quantity = quantity;
                let mut amount_spent = 0;
                for (price, orders) in book.iter() {
                    if remaining_quantity == 0 {
                        break;
                    }
                    Self::process_price(
                        &mut amount_spent,
                        &mut remaining_quantity,
                        price,
                        orders,
                        &self.order_store);
                }
                Self::process_remaining_quantity(
                    amount_spent,
                    remaining_quantity,
                    quantity,
                    min_ask
                )
            }
            Side::Ask => {
                let max_bid = match self.max_bid {
                    Some(bid) => bid,
                    None => return RfqStatus::NotPossible
                };
                let book = &self.bid_side_book;
                let mut remaining_quantity = quantity;
                let mut amount_spent = 0;
                for (price, orders) in book.iter().rev() {
                    if remaining_quantity == 0 {
                        break;
                    }
                    Self::process_price(
                        &mut amount_spent,
                        &mut remaining_quantity,
                        price,
                        orders,
                        &self.order_store
                    );
                }
                Self::process_remaining_quantity(
                    amount_spent,
                    remaining_quantity,
                    quantity,
                    max_bid
                )
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::core::{
        models::{ExecutionResult, FillMetaData, LimitOrder, MarketOrder, Operation, FillResult, Side},
        orderbook::OrderBook,
        store::Store,
    };
    use std::collections::{BTreeMap, VecDeque};
    use std::ops::Index;

    fn create_orderbook() -> OrderBook {
        let mut book = OrderBook::default();
        let orders = vec![
            LimitOrder::new(1, 100, 100, Side::Bid),
            LimitOrder::new(2, 100, 150, Side::Bid),
            LimitOrder::new(3, 100, 50, Side::Bid),
            LimitOrder::new(4, 110, 200, Side::Bid),
            LimitOrder::new(5, 110, 100, Side::Bid),
            LimitOrder::new(6, 120, 100, Side::Ask),
            LimitOrder::new(7, 120, 150, Side::Ask),
            LimitOrder::new(8, 120, 50, Side::Ask),
            LimitOrder::new(9, 130, 200, Side::Ask),
            LimitOrder::new(10, 130, 100, Side::Ask),
        ];
        for order in orders {
            book.execute(Operation::Limit(order));
        }
        book
    }

    fn fills_to_ids(fills: Vec<FillMetaData>) -> Vec<u128> {
        fills.iter().map(|f| f.matched_order_id).collect()
    }

    fn get_total_quantity_at_price(
        price: &u64,
        book: &BTreeMap<u64, VecDeque<usize>>,
        store: &Store,
    ) -> u64 {
        match book.get(price) {
            Some(orders) => orders
                .iter()
                .map(|index| store.index(*index).quantity)
                .sum(),
            None => 0,
        }
    }

    #[test]
    fn it_gets_total_quantity_at_price() {
        let book = create_orderbook();
        let result = get_total_quantity_at_price(&100, &book.bid_side_book, &book.order_store);
        assert_eq!(300, result);
    }

    #[test]
    fn it_cancels_order_when_it_exists() {
        let mut book = create_orderbook();
        let order = LimitOrder::new(11, 115, 100, Side::Bid);
        book.execute(Operation::Limit(order));
        match book.cancel_order(order.id) {
            Some(id) => {
                let store_order = book.order_store.get(id);
                assert!(id == order.id && book.get_max_bid() == Some(110) && store_order.is_none())
            }
            _ => panic!("test failed"),
        }
    }

    #[test]
    fn it_cancels_nothing_when_order_does_not_exist() {
        let mut book = create_orderbook();
        match book.cancel_order(11) {
            None => (),
            _ => panic!("test failed"),
        }
    }

    #[test]
    fn it_executes_a_limit_bid_that_is_created() {
        let mut book = create_orderbook();
        let order = LimitOrder::new(11, 100, 500, Side::Bid);
        match book.limit_bid_order(order) {
            FillResult::Created(created_order) => {
                let (stored_order, _) = book.order_store.get(order.id).unwrap();
                assert!(created_order.id == order.id && order == *stored_order)
            }
            _ => panic!("test failed"),
        }
    }

    #[test]
    fn it_executes_a_limit_bid_that_is_filled() {
        let mut book = create_orderbook();
        let order = LimitOrder::new(11, 130, 400, Side::Bid);
        match book.limit_bid_order(order) {
            FillResult::Filled(order_fills) => {
                let quantity =
                    get_total_quantity_at_price(&130, &book.ask_side_book, &book.order_store);
                assert!(fills_to_ids(order_fills) == vec![6, 7, 8, 9] && quantity == 200);
            }
            _ => panic!("test failed"),
        }
    }

    #[test]
    fn it_executes_a_limit_bid_that_is_partially_filled() {
        let mut book = create_orderbook();
        let order = LimitOrder::new(11, 150, 700, Side::Bid);
        match book.limit_bid_order(order) {
            FillResult::PartiallyFilled(order_placed, order_fills) => {
                let (stored_order, _) = book.order_store.get(order.id).unwrap();
                let created_order = LimitOrder::new(11, 150, 100, Side::Bid);
                assert!(
                    fills_to_ids(order_fills) == vec![6, 7, 8, 9, 10]
                        && order_placed == created_order
                        && created_order == *stored_order
                );
            }
            _ => panic!("invalid case for test"),
        }
    }

    #[test]
    fn it_executes_a_limit_ask_that_is_created() {
        let mut book = create_orderbook();
        let order = LimitOrder::new(11, 120, 250, Side::Ask);
        match book.limit_ask_order(order) {
            FillResult::Created(created_order) => {
                let (stored_order, _) = book.order_store.get(order.id).unwrap();
                assert!(created_order.id == order.id && order == *stored_order)
            }
            _ => panic!("test failed"),
        }
    }

    #[test]
    fn it_executes_a_limit_ask_that_is_filled() {
        let mut book = create_orderbook();
        let order = LimitOrder::new(11, 100, 400, Side::Ask);
        match book.limit_ask_order(order) {
            FillResult::Filled(order_fills) => {
                let quantity = get_total_quantity_at_price(
                    &order.price,
                    &book.bid_side_book,
                    &book.order_store,
                );
                assert!(fills_to_ids(order_fills) == vec![4, 5, 1] && quantity == 200);
            }
            _ => panic!("test failed"),
        }
    }

    #[test]
    fn it_executes_a_limit_ask_that_is_partially_filled() {
        let mut book = create_orderbook();
        let order = LimitOrder::new(11, 90, 700, Side::Ask);
        match book.limit_ask_order(order) {
            FillResult::PartiallyFilled(order_placed, order_fills) => {
                let (stored_order, _) = book.order_store.get(order.id).unwrap();
                let created_order = LimitOrder::new(11, 90, 100, Side::Ask);
                assert!(
                    fills_to_ids(order_fills) == vec![4, 5, 1, 2, 3]
                        && order_placed == created_order
                        && created_order == *stored_order
                );
            }
            _ => panic!("test failed"),
        }
    }

    #[test]
    fn it_modifies_limit_bid_order_quantity() {
        let mut book = create_orderbook();
        let order = LimitOrder::new(1, 100, 150, Side::Bid);
        book.modify_limit_buy_order(order);
        assert_eq!(
            get_total_quantity_at_price(&order.price, &book.bid_side_book, &book.order_store),
            350
        );
    }

    #[test]
    fn it_modifies_limit_ask_order_quantity() {
        let mut book = create_orderbook();
        let order = LimitOrder::new(6, 120, 150, Side::Ask);
        book.modify_limit_ask_order(order);
        assert_eq!(
            get_total_quantity_at_price(&order.price, &book.ask_side_book, &book.order_store),
            350
        );
    }

    #[test]
    fn it_modifies_limit_bid_order_price() {
        let mut book = create_orderbook();
        let order = LimitOrder::new(1, 120, 400, Side::Bid);
        book.modify_limit_buy_order(order);
        let quantity_at_100 =
            get_total_quantity_at_price(&100, &book.bid_side_book, &book.order_store);
        let quantity_at_120 =
            get_total_quantity_at_price(&120, &book.bid_side_book, &book.order_store);
        assert!(quantity_at_100 == 200 && quantity_at_120 == 100);
    }

    #[test]
    fn it_modifies_limit_ask_order_price() {
        let mut book = create_orderbook();
        let order = LimitOrder::new(6, 110, 400, Side::Ask);
        book.modify_limit_ask_order(order);
        let quantity_at_120 =
            get_total_quantity_at_price(&120, &book.ask_side_book, &book.order_store);
        let quantity_at_110 =
            get_total_quantity_at_price(&110, &book.ask_side_book, &book.order_store);
        assert!(quantity_at_120 == 200 && quantity_at_110 == 100);
    }

    #[test]
    fn it_modifies_nothing_when_price_and_quantity_are_same() {
        let mut book = create_orderbook();
        let order = LimitOrder::new(1, 100, 100, Side::Bid);
        book.modify_limit_buy_order(order);
        assert_eq!(
            get_total_quantity_at_price(&100, &book.bid_side_book, &book.order_store),
            300
        );
    }

    #[test]
    fn it_executes_a_market_bid_filled() {
        let mut book = create_orderbook();
        let order = MarketOrder::new(11, 500, Side::Bid);
        match book.market_bid_order(order) {
            FillResult::Filled(order_fills) => {
                let quantity =
                    get_total_quantity_at_price(&130, &book.ask_side_book, &book.order_store);
                assert!(fills_to_ids(order_fills) == vec![6, 7, 8, 9] && quantity == 100);
            }
            _ => panic!("test failed"),
        }
    }

    #[test]
    fn it_executes_a_market_ask_filled() {
        let mut book = create_orderbook();
        let order = MarketOrder::new(11, 500, Side::Ask);
        match book.market_ask_order(order) {
            FillResult::Filled(order_fills) => {
                let quantity =
                    get_total_quantity_at_price(&100, &book.bid_side_book, &book.order_store);
                assert!(fills_to_ids(order_fills) == vec![4, 5, 1, 2] && quantity == 100);
            }
            _ => panic!("test failed"),
        }
    }

    #[test]
    fn it_executes_a_market_bid_partially_filled() {
        let mut book = create_orderbook();
        let order = MarketOrder::new(11, 700, Side::Bid);
        match book.market_bid_order(order) {
            FillResult::PartiallyFilled(order_placed, order_fills) => {
                assert!(
                    fills_to_ids(order_fills) == vec![6, 7, 8, 9, 10]
                        && order_placed == LimitOrder::new(11, 130, 100, Side::Bid)
                );
            }
            _ => panic!("test failed"),
        }
    }

    #[test]
    fn it_executes_a_market_ask_partially_filled() {
        let mut book = create_orderbook();
        let order = MarketOrder::new(11, 700, Side::Ask);
        match book.market_ask_order(order) {
            FillResult::PartiallyFilled(order_placed, order_fills) => {
                assert!(
                    fills_to_ids(order_fills) == vec![4, 5, 1, 2, 3]
                        && order_placed == LimitOrder::new(11, 100, 100, Side::Ask)
                );
            }
            _ => panic!("test failed"),
        }
    }

    #[test]
    fn it_does_not_execute_market_bid_when_max_bid_is_none() {
        let mut book = OrderBook::default();
        let order = MarketOrder::new(1, 100, Side::Bid);
        match book.execute(Operation::Market(order)) {
            ExecutionResult::Failed(message) => {
                assert_eq!(message, "placed market order on empty book")
            }
            _ => panic!("test failed"),
        }
    }

    #[test]
    fn it_does_not_execute_market_ask_when_max_bid_is_none() {
        let mut book = OrderBook::default();
        let order = MarketOrder::new(1, 100, Side::Ask);
        match book.execute(Operation::Market(order)) {
            ExecutionResult::Failed(message) => {
                assert_eq!(message, "placed market order on empty book")
            }
            _ => panic!("test failed"),
        }
    }

    #[test]
    fn it_updates_top_price_when_bid_is_created() {
        let mut book = create_orderbook();
        let order = LimitOrder::new(11, 115, 500, Side::Bid);
        book.limit_bid_order(order);
        match book.max_bid {
            Some(price) => assert_eq!(price, order.price),
            None => panic!("test failed"),
        }
    }

    #[test]
    fn it_updates_top_price_when_ask_is_created() {
        let mut book = create_orderbook();
        let order = LimitOrder::new(11, 115, 500, Side::Ask);
        book.limit_ask_order(order);
        match book.min_ask {
            Some(price) => assert_eq!(price, order.price),
            None => panic!("test failed"),
        }
    }

    #[test]
    fn it_updates_top_price_when_bid_is_filled() {
        let mut book = create_orderbook();
        let order = LimitOrder::new(11, 130, 500, Side::Bid);
        book.limit_bid_order(order);
        match book.min_ask {
            Some(price) => assert_eq!(price, order.price),
            None => panic!("test failed"),
        }
    }

    #[test]
    fn it_updates_top_price_when_ask_is_filled() {
        let mut book = create_orderbook();
        let order = LimitOrder::new(11, 100, 500, Side::Ask);
        book.limit_ask_order(order);
        match book.max_bid {
            Some(price) => assert_eq!(price, order.price),
            None => panic!("test failed"),
        }
    }

    #[test]
    fn it_updates_top_price_when_bid_is_partially_filled() {
        let mut book = create_orderbook();
        let order = LimitOrder::new(11, 130, 700, Side::Bid);
        book.limit_bid_order(order);
        assert!(book.min_ask == Some(order.price) && book.max_bid == Some(order.price))
    }

    #[test]
    fn it_updates_top_price_when_ask_is_partially_filled() {
        let mut book = create_orderbook();
        let order = LimitOrder::new(11, 100, 700, Side::Ask);
        book.limit_ask_order(order);
        assert!(book.max_bid == Some(order.price) && book.min_ask == Some(order.price))
    }

    #[test]
    fn it_tests_orderbook_depth() {
        let book = create_orderbook();
        let depth = book.depth(2);
        assert!(
            depth.levels == 2
                && depth.bids.len() == 2
                && depth.asks.len() == 2
                && depth.bids[0].price == 100
                && depth.bids[1].price == 110
                && depth.bids[0].quantity == 300
                && depth.bids[1].quantity == 300
                && depth.asks[0].price == 120
                && depth.asks[1].price == 130
                && depth.asks[0].quantity == 300
                && depth.asks[1].quantity == 300
        );
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
        let book = OrderBook::default();
        let max_bid = book.get_max_bid();
        assert_eq!(max_bid, None);
    }

    #[test]
    fn it_returns_none_for_empty_get_min_ask() {
        let book = OrderBook::default();
        let min_ask = book.get_min_ask();
        assert_eq!(min_ask, None);
    }
}
