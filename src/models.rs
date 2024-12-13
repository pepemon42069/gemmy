use serde::{Deserialize, Serialize};

#[derive(Debug, Copy, Clone, PartialEq, Serialize, Deserialize)]
pub enum Side {
    Bid,
    Ask
}

#[derive(Debug, Copy, Clone, Serialize, Deserialize)]
pub enum OrderType {
    Limit,
    Market
}

#[derive(Debug, Copy, Clone, Serialize, Deserialize)]
pub enum OrderOperation {
    Place(OrderRequest),
    Modify(OrderRequest, u64, u64),
    Cancel(OrderRequest)
}

#[derive(Debug, Serialize, Deserialize)]
pub enum FillResult {
    InvalidOrder,
    Filled(Vec<(u128, u64, u64)>),
    PartiallyFilled(Vec<(u128, u64, u64)>, (u128, u64, u64)),
    Created((u128, u64, u64))
}

#[derive(Debug, Serialize, Deserialize)]
pub enum ExecutionResult {
    Executed(FillResult),
    Modified(Option<FillResult>),
    Cancelled(u128),
    NoExecution
}

#[derive(Debug)]
pub(crate) enum ModifyResult {
    CreateNewOrder,
    ModifiedOrder,
    Unchanged
}

#[derive(Debug)]
pub(crate) struct Order {
    pub id: u128,
    pub quantity: u64
}

#[derive(Debug, Copy, Clone, Serialize, Deserialize)]
pub struct OrderRequest {
    pub id: u128,
    pub price: u64,
    pub quantity: u64,
    pub side: Side,
    pub order_type: OrderType
}

impl OrderRequest {
    pub fn new(
        id: u128, price: u64, quantity: u64, side: Side, order_type: OrderType) -> OrderRequest {
        OrderRequest {
            id,
            price,
            quantity,
            side,
            order_type
        }
    }

    pub(crate) fn to_order(self) -> Order {
        Order {
            id: self.id,
            quantity: self.quantity
        }
    }
}