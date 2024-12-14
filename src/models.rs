use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Copy, Clone, PartialEq, Serialize, Deserialize)]
pub enum Side {
    Bid,
    Ask,
}

#[derive(Debug, Copy, Clone)]
pub enum Operation {
    Limit(LimitOrder),
    Market(MarketOrder),
    Modify(LimitOrder),
    Cancel(u128),
}

#[derive(Debug)]
pub enum FillResult {
    Filled(Vec<FillMetaData>),
    PartiallyFilled(LimitOrder, Vec<FillMetaData>),
    Created(LimitOrder),
    Failed,
}

#[derive(Debug)]
pub enum ExecutionResult {
    Executed(FillResult),
    Modified(ModifyResult),
    Cancelled(u128),
    Failed(String),
}

#[derive(Debug)]
pub enum ModifyResult {
    Created(FillResult),
    Modified,
    Failed,
}

#[derive(Debug, Copy, Clone, PartialEq)]
pub struct LimitOrder {
    pub id: u128,
    pub price: u64,
    pub quantity: u64,
    pub side: Side,
}

impl LimitOrder {
    pub fn new(id: u128, price: u64, quantity: u64, side: Side) -> Self {
        Self {
            id,
            price,
            quantity,
            side,
        }
    }

    pub fn new_uuid_v4(price: u64, quantity: u64, side: Side) -> Self {
        Self {
            id: Uuid::new_v4().as_u128(),
            price,
            quantity,
            side,
        }
    }

    #[inline(always)]
    pub fn update_order_quantity(&mut self, quantity: u64) {
        self.quantity = quantity;
    }
}

#[derive(Debug, Copy, Clone, PartialEq)]
pub struct MarketOrder {
    pub id: u128,
    pub quantity: u64,
    pub side: Side,
}

impl MarketOrder {
    pub fn new(id: u128, quantity: u64, side: Side) -> Self {
        Self { id, quantity, side }
    }

    pub fn new_uuid_v4(quantity: u64, side: Side) -> Self {
        Self {
            id: Uuid::new_v4().as_u128(),
            quantity,
            side,
        }
    }

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

#[derive(Debug, Copy, Clone, PartialEq)]
pub struct FillMetaData {
    pub order_id: u128,
    pub matched_order_id: u128,
    pub taker_side: Side,
    pub price: u64,
    pub quantity: u64,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Depth {
    pub levels: usize,
    pub bids: Vec<Level>,
    pub asks: Vec<Level>,
}

#[derive(Debug, Copy, Clone, PartialEq)]
pub struct Level {
    pub price: u64,
    pub quantity: u64,
}
