use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Side, as the name indicates is used to represent a side of the orderbook.
/// The traits Serialize, Deserialize are implemented to broaden its utility.
#[derive(Debug, Copy, Clone, PartialEq, Serialize, Deserialize)]
pub enum Side {
    /// Bid represents the buy side of the orderbook.
    Bid,
    /// Ask represents the sell side of the orderbook.
    Ask,
}

/// This represents the available operations that can be performed by the orderbook.
#[derive(Debug, Copy, Clone)]
pub enum Operation {
    /// Limit allows the user to place a limit order through a [`LimitOrder`] struct.
    Limit(LimitOrder),
    /// Market allows the user to place a market order through a [`MarketOrder`] struct.
    Market(MarketOrder),
    /// Modify allows the user to change the price and quantity of an existing limit order.
    /// This too takes a [`LimitOrder`] struct that must contain the original id of the order.
    /// The values can for price and quantity can be same or different.
    Modify(LimitOrder),
    /// Cancel allows the user to cancel an existing limit order.
    /// This only takes the existing order id.
    Cancel(u128),
}

/// This represents the result when an order is placed in the orderbook.
/// The successful cases contain metadata about which makers got matched and the order that gets created.
#[derive(Debug)]
pub enum FillResult {
    /// This means that the limit order was fully filled and contains a vector of [`FillMetaData`] struct.
    /// This metadata describes the matched orders.
    Filled(Vec<FillMetaData>),
    /// This means that the limit order was partially filled and contains the [`LimitOrder`] that was created,
    /// as well as a vector of [`FillMetaData`] struct containing any matched orders.
    PartiallyFilled(LimitOrder, Vec<FillMetaData>),
    /// This means that the limit order was created and wasn't matched against any other bids.
    /// This contains a [`LimitOrder`] struct.
    Created(LimitOrder),
    /// This is used to represent any failure scenario in order matching.
    Failed,
}

/// This represents the result of an operation execution.
/// Depending on the flow of the operation, it can amount to one of four possible values.
#[derive(Debug)]
pub enum ExecutionResult {
    /// This is returned every time an order is matched within the execution flow that generates a [`FillResult`].
    Executed(FillResult),
    /// This is returned when the execution modifies an existing limit order and generates a [`ModifyResult`] enum.
    Modified(ModifyResult),
    /// This is returned when the execution cancels an existing order with the passed id.
    Cancelled(u128),
    /// This is used to represent any failure scenario in operation execution.
    Failed(String),
}

/// This represents the result of a modify operation for an existing limit order.
#[derive(Debug)]
pub enum ModifyResult {
    /// This means that post order modification, a new limit order was created.
    /// [`FillResult`] will contain any matched orders or the created limit order.
    Created(FillResult),
    /// This means that the order was modified in place i.e. it's quantity was updated.
    Modified,
    ///  This is used to represent any failure scenario while modifying the limit order.
    Failed,
}

/// This structure represents a limit order.
#[derive(Debug, Copy, Clone, PartialEq)]
pub struct LimitOrder {
    /// This represents unique 128-bit id can is capable of storing uuid v4.
    /// The uniqueness of this id is not enforced within the book as of now.
    pub id: u128,
    /// This represents the price of the asset.
    pub price: u64,
    /// This represents the quantity of the asset.
    pub quantity: u64,
    /// This is the side of the orderbook in which the order will get placed.
    pub side: Side,
}

impl LimitOrder {
    /// This is a constructor like method.
    ///
    /// # Arguments
    ///
    /// * `id` - A unique order id.
    /// * `price` - The price at which the order will get placed.
    /// * `quantity` - The quantity of the opposite side to be matched.
    /// * `side` - The side of the orderbook where this order gets placed.
    ///
    /// # Returns
    ///
    /// * A [`LimitOrder`] with the specified arguments.
    pub fn new(id: u128, price: u64, quantity: u64, side: Side) -> Self {
        Self {
            id,
            price,
            quantity,
            side,
        }
    }

    /// This is the same as new, except it auto generates id. (uuid v4)
    ///
    /// # Arguments
    ///
    /// * `price` - The price at which the order will get placed.
    /// * `quantity` - The quantity of the opposite side to be matched.
    /// * `side` - The side of the orderbook where this order gets placed.
    ///
    /// # Returns
    ///
    /// * A [`LimitOrder`] with the specified arguments and an auto generated 128-bit id.
    pub fn new_uuid_v4(price: u64, quantity: u64, side: Side) -> Self {
        Self {
            id: Uuid::new_v4().as_u128(),
            price,
            quantity,
            side,
        }
    }

    /// This is a helper method to change the quantity of the limit order in place.
    ///
    /// # Arguments
    ///
    /// * `quantity` - The new quantity for the order.
    ///
    /// # Returns
    ///
    /// * `()` This function does not return any value.
    #[inline(always)]
    pub fn update_order_quantity(&mut self, quantity: u64) {
        self.quantity = quantity;
    }
}

/// This represents a market order.
/// It's essentially same as the [`LimitOrder`] struct but does not contain an asset price.
#[derive(Debug, Copy, Clone, PartialEq)]
pub struct MarketOrder {
    /// This represents unique 128-bit id can is capable of storing uuid v4.
    /// The uniqueness of this id is not enforced within the book as of now.
    pub id: u128,
    /// This represents the price of the asset.
    pub quantity: u64,
    /// This is the side of the orderbook in which the order will get placed.
    pub side: Side,
}

impl MarketOrder {
    /// This is a constructor like method.
    ///
    /// # Arguments
    ///
    /// * `id` - A unique order id.
    /// * `quantity` - The quantity of the opposite side to be matched.
    /// * `side` - The side of the orderbook where this order gets placed.
    ///
    /// # Returns
    ///
    /// * A [`MarketOrder`] with the specified arguments.
    pub fn new(id: u128, quantity: u64, side: Side) -> Self {
        Self { id, quantity, side }
    }

    /// This is the same as new, except it auto generates id. (uuid v4)
    ///
    /// # Arguments
    ///
    /// * `quantity` - The quantity of the opposite side to be matched.
    /// * `side` - The side of the orderbook where this order gets placed.
    ///
    /// # Returns
    ///
    /// * A [`MarketOrder`] with the specified arguments and an auto generated 128-bit id.
    pub fn new_uuid_v4(quantity: u64, side: Side) -> Self {
        Self {
            id: Uuid::new_v4().as_u128(),
            quantity,
            side,
        }
    }

    /// This is a helper method that transforms a [`MarketOrder`] into a [`LimitOrder`] with the passed price.
    /// # Arguments
    ///
    /// * `price` - The price at which the order will get placed.
    ///
    /// # Returns
    ///
    /// * A [`LimitOrder`] with the specified price and same details as the market order that calls the method.
    #[inline(always)]
    pub fn to_limit(&self, price: u64) -> LimitOrder {
        LimitOrder {
            id: self.id,
            price,
            quantity: self.quantity,
            side: self.side,
        }
    }
}

/// This struct represents the data generated whenever an order is matched against one on the opposite side.
#[derive(Debug, Copy, Clone, PartialEq)]
pub struct FillMetaData {
    /// This is the id of the taker's order.
    pub order_id: u128,
    /// This is the id of the matched maker's order.
    pub matched_order_id: u128,
    /// This is the side of the taker.
    pub taker_side: Side,
    /// This is the price at which the matching takes place.
    pub price: u64,
    /// this is the quantity filled in this match.
    pub quantity: u64,
}

/// This represents a struct used to return bids and asks in the orderbook at a specific depth.
/// For example, a level 2 depth will give us top two bids and bottom two asks with aggregated quantities.
#[derive(Debug, Clone, PartialEq)]
pub struct Depth {
    /// The number of price levels to be returned on either side from center of the orderbook.
    pub levels: usize,
    /// A vector of bids aggregated by quantity of the same length as levels.
    pub bids: Vec<Level>,
    /// A vector of asks aggregated by quantity of the same length as levels.
    pub asks: Vec<Level>,
}

/// This is a helper struct used in construction of depth.
#[derive(Debug, Copy, Clone, PartialEq)]
pub struct Level {
    /// A price point in the orderbook.
    pub price: u64,
    /// Aggregated quantity of all orders at the aforementioned price point.
    pub quantity: u64,
}
