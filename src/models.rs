use serde::{Deserialize, Serialize};
use uuid::Uuid;
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
    Filled(Vec<(Uuid, u64, u64)>),
    PartiallyFilled(Vec<(Uuid, u64, u64)>, (Uuid, u64, u64)),
    Created((Uuid, u64, u64))
}

#[derive(Debug, Serialize, Deserialize)]
pub enum ExecutionResult {
    Executed(FillResult),
    Modified(Option<FillResult>),
    Cancelled(Uuid)
}

#[derive(Debug)]
pub(crate) enum ModifyResult {
    CreateNewOrder,
    ModifiedOrder,
    Unchanged
}

#[derive(Debug)]
pub(crate) struct Order {
    pub id: Uuid,
    pub quantity: u64
}