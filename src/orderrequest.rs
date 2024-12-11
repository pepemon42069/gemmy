use serde::{Deserialize, Serialize};
use uuid::Uuid;
use crate::models::{Order, OrderType, Side};
use crate::utils::get_timestamp_now_micros;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrderRequest {
    pub id: Uuid,
    pub price: u64,
    pub quantity: u64,
    pub side: Side,
    pub order_type: OrderType,
    pub timestamp: u128
}

impl OrderRequest {
    pub fn new(price: u64, quantity: u64, side: Side, order_type: OrderType) -> OrderRequest {
        OrderRequest {
            id: Uuid::new_v4(),
            price,
            quantity,
            side,
            order_type,
            timestamp: get_timestamp_now_micros()
        }
    }

    pub(crate) fn to_order(&self) -> Order {
        Order {
            id: self.id,
            quantity: self.quantity
        }
    }
}