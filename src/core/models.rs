use crate::protobuf::models::{
    CancelModifyOrder, CreateOrder, FillOrder, FillOrderData, GenericMessage, PartialFillOrder,
    RfqResult,
};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Side, as the name indicates is used to represent a side of the orderbook.
/// The traits Serialize, Deserialize are implemented to broaden its utility.
#[derive(Debug, Copy, Clone, PartialEq, Serialize, Deserialize)]
pub enum Side {
    /// Bid represents the buy side of the orderbook.
    Bid = 0,
    /// Ask represents the sell side of the orderbook.
    Ask = 1,
}

impl From<i32> for Side {
    fn from(value: i32) -> Self {
        match value {
            0 => Side::Bid,
            1 => Side::Ask,
            _ => panic!("invalid side"),
        }
    }
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
    Executed(FillResult, String, u128),
    /// This is returned when the execution modifies an existing limit order and generates a [`ModifyResult`] enum.
    Modified(ModifyResult, String, u128),
    /// This is returned when the execution cancels an existing order with the passed id.
    Cancelled(u128, String, u128),
    /// This is used to represent any failure scenario in operation execution.
    Failed(String, String, u128),
}

#[derive(Debug)]
pub enum RfqStatus {
    CompleteFill(u64),
    PartialFillAndLimitPlaced(u64, u64),
    ConvertToLimit(u64, u64),
    NotPossible,
}

impl RfqStatus {
    pub fn to_protobuf(&self) -> RfqResult {
        match self {
            RfqStatus::CompleteFill(price) => RfqResult {
                status: 0,
                price: *price,
                quantity: 0,
            },
            RfqStatus::PartialFillAndLimitPlaced(price, remaining_quantity) => RfqResult {
                status: 1,
                price: *price,
                quantity: *remaining_quantity,
            },
            RfqStatus::ConvertToLimit(price, quantity) => RfqResult {
                status: 2,
                price: *price,
                quantity: *quantity,
            },
            RfqStatus::NotPossible => RfqResult {
                status: 3,
                price: 0,
                quantity: 0,
            },
        }
    }
}

/// This represents the result of a modify operation for an existing limit order.
#[derive(Debug)]
pub enum ModifyResult {
    /// This means that post order modification, a new limit order was created.
    /// [`FillResult`] will contain any matched orders or the created limit order.
    Created(FillResult),
    /// This means that the order was modified in place i.e. it's quantity was updated.
    Modified(u128),
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

    fn to_create_order_proto(self, ticker: String, timestamp: u128) -> CreateOrder {
        CreateOrder {
            status: 0,
            order_id: self.id.to_be_bytes().to_vec(),
            price: self.price,
            quantity: self.quantity,
            side: self.side as i32,
            symbol: ticker,
            timestamp: timestamp.to_be_bytes().to_vec(),
        }
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

impl FillMetaData {
    fn to_fill_order_data_proto(self) -> FillOrderData {
        FillOrderData {
            order_id: self.order_id.to_be_bytes().to_vec(),
            matched_order_id: self.matched_order_id.to_be_bytes().to_vec(),
            taker_side: self.taker_side as i32,
            price: self.price,
            amount: self.quantity,
        }
    }
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

#[derive(Debug, Clone)]
pub enum ProtoBufResult {
    Create(CreateOrder),
    Fill(FillOrder),
    PartialFill(PartialFillOrder),
    CancelModify(CancelModifyOrder),
    Failed(GenericMessage),
}

pub trait ProtoBuf {
    fn to_protobuf(&self, ticker: String, timestamp: u128) -> ProtoBufResult;
}

impl ProtoBuf for FillResult {
    fn to_protobuf(&self, ticker: String, timestamp: u128) -> ProtoBufResult {
        match self {
            FillResult::Created(order) => ProtoBufResult::Create(order.to_create_order_proto(ticker, timestamp)),
            FillResult::Filled(order_fills) => ProtoBufResult::Fill(FillOrder {
                status: 1,
                filled_orders: order_fills
                    .iter()
                    .map(|fill_data| fill_data.to_fill_order_data_proto())
                    .collect(),
                symbol: ticker,
                timestamp: timestamp.to_be_bytes().to_vec(),
            }),
            FillResult::PartiallyFilled(order, order_fills) => {
                ProtoBufResult::PartialFill(PartialFillOrder {
                    status: 2,
                    partial_create: Some(order.to_create_order_proto(ticker.clone(), timestamp)),
                    partial_fills: Some(FillOrder {
                        status: 2,
                        filled_orders: order_fills
                            .iter()
                            .map(|fill_data| fill_data.to_fill_order_data_proto())
                            .collect(),
                        symbol: ticker.clone(),
                        timestamp: timestamp.to_be_bytes().to_vec(),
                    }),
                    symbol: ticker,
                    timestamp: timestamp.to_be_bytes().to_vec(),
                })
            }
            FillResult::Failed => ProtoBufResult::Failed(GenericMessage {
                message: "failed to place order".to_string(),
                symbol: ticker,
                timestamp: timestamp.to_be_bytes().to_vec(),
            }),
        }
    }
}

impl ProtoBuf for ModifyResult {
    fn to_protobuf(&self, ticker: String, timestamp: u128) -> ProtoBufResult {
        match self {
            ModifyResult::Created(fill_result) => fill_result.to_protobuf(ticker, timestamp),
            ModifyResult::Modified(id) => ProtoBufResult::CancelModify(CancelModifyOrder {
                status: 3,
                order_id: id.to_be_bytes().to_vec(),
                symbol: ticker,
                timestamp: timestamp.to_be_bytes().to_vec(),
            }),
            ModifyResult::Failed => ProtoBufResult::Failed(GenericMessage {
                message: "failed to modify order".to_string(),
                symbol: ticker,
                timestamp: timestamp.to_be_bytes().to_vec(),
            }),
        }
    }
}

impl ExecutionResult {
    pub fn to_protobuf(&self) -> ProtoBufResult {
        match self {
            ExecutionResult::Executed(fill_result, ticker, timestamp) => 
                fill_result.to_protobuf(ticker.clone(), *timestamp),
            ExecutionResult::Modified(modify_result, ticker, timestamp) => 
                modify_result.to_protobuf(ticker.clone(), *timestamp),
            ExecutionResult::Cancelled(id, ticker, timestamp) => 
                ProtoBufResult::CancelModify(CancelModifyOrder {
                    status: 4,
                    order_id: id.to_be_bytes().to_vec(),
                    symbol: ticker.clone(),
                    timestamp: timestamp.to_be_bytes().to_vec(),
                }),
            ExecutionResult::Failed(message, ticker, timestamp) => 
                ProtoBufResult::Failed(GenericMessage {
                    message: message.to_string(),
                    symbol: ticker.clone(),
                    timestamp: timestamp.to_be_bytes().to_vec(),
                }),
        }
    }
}
