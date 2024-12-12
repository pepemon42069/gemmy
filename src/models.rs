use serde::{Deserialize, Serialize};
use crate::orderrequest::OrderRequest;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum Side {
    Bid,
    Ask
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum OrderType {
    Limit,
    Market
}

#[derive(Debug, Serialize, Deserialize)]
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
    Cancelled(u128)
}

#[derive(Debug)]
pub(crate) enum ModifyResult {
    CreateNewOrder,
    ModifiedOrder,
    Unchanged
}

#[derive(Debug)]
pub struct Order {
    pub id: u128,
    pub quantity: u64
}