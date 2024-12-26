use prost::Message;
use schema_registry_converter::async_impl::proto_raw::ProtoRawEncoder;
use schema_registry_converter::schema_registry_common::SubjectNameStrategy;
use crate::core::models::{ExecutionResult, FillMetaData, FillResult, LimitOrder, ModifyResult, OrderbookAggregated, RfqStatus};
use crate::protobuf::models::{CancelModifyOrder, CreateOrder, FillOrder, FillOrderData, GenericMessage, Level, OrderbookData, PartialFillOrder, RfqResult};

pub async fn exec_to_proto_encoded<'a>(
    execution_result: ExecutionResult,
    symbol: String,
    timestamp: u128,
    encoder: &ProtoRawEncoder<'a>
) -> Vec<u8> {
    let (encoded_data, schema_name) = match execution_result {
        ExecutionResult::Executed(fill_result) =>
            fill_result_to_proto(fill_result, symbol, timestamp),
        ExecutionResult::Modified(modify_result) =>
            modify_result_to_proto(modify_result, symbol, timestamp),
        ExecutionResult::Cancelled(id) =>
            (CancelModifyOrder {
                status: 4,
                order_id: id.to_be_bytes().to_vec(),
                symbol,
                timestamp: timestamp.to_be_bytes().to_vec(),
            }.encode_to_vec(), "CancelModifyOrder"),
        ExecutionResult::Failed(message) =>
            (GenericMessage {
                message: message.clone(),
                symbol,
                timestamp: timestamp.to_be_bytes().to_vec(),
            }.encode_to_vec(), "GenericMessage"),
    };
    encode_proto(encoded_data, schema_name, encoder).await
}

async fn encode_proto<'a>(
    encoded_data: Vec<u8> ,
    schema_name: &str,
    proto_raw_encoder: &ProtoRawEncoder<'a>
) -> Vec<u8> {
    proto_raw_encoder.encode(
        &encoded_data,
        format!("models.{}", schema_name).as_str(),
        SubjectNameStrategy::RecordNameStrategy("models".to_string())
    ).await.unwrap()
}

pub fn rfq_to_proto(rfq_status: RfqStatus) -> RfqResult {
    match rfq_status {
        RfqStatus::CompleteFill(price) => RfqResult {
            status: 0,
            price,
            quantity: 0,
        },
        RfqStatus::PartialFillAndLimitPlaced(price, quantity) => RfqResult {
            status: 1,
            price,
            quantity,
        },
        RfqStatus::ConvertToLimit(price, quantity) => RfqResult {
            status: 2,
            price,
            quantity,
        },
        RfqStatus::NotPossible => RfqResult {
            status: 3,
            price: 0,
            quantity: 0,
        },
    }
}

pub fn orderbook_data_to_proto(
    last_trade_price: u64,
    max_bid: u64,
    min_ask: u64,
    orderbook_data: OrderbookAggregated
) -> OrderbookData {
    OrderbookData {
        last_trade_price,
        max_bid,
        min_ask,
        bids: orderbook_data.bids.iter()
            .map(|(p, q)| Level { price: *p, quantity: *q })
            .collect(),
        asks: orderbook_data.asks.iter()
            .map(|(p, q)| Level { price: *p, quantity: *q })
            .collect(),
    }
}

fn fill_result_to_proto<'a>(
    fill_result: FillResult, 
    symbol: String, 
    timestamp:  u128
) -> (Vec<u8>, &'a str) {
    match fill_result {
        FillResult::Created(order) => 
            (limit_to_proto(order, symbol, timestamp).encode_to_vec(), "CreateOrder"),
        FillResult::Filled(order_fills) => 
            (FillOrder {
                status: 1,
                filled_orders: order_fills
                    .iter()
                    .map(|fill_data| fill_meta_data_to_proto(*fill_data))
                    .collect(),
                symbol,
                timestamp: timestamp.to_be_bytes().to_vec(),
            }.encode_to_vec(), "FillOrder"),
        FillResult::PartiallyFilled(order, order_fills) => 
            (PartialFillOrder {
                status: 2,
                partial_create: Some(limit_to_proto(order, symbol.clone(), timestamp)),
                partial_fills: Some(FillOrder {
                    status: 2,
                    filled_orders: order_fills
                        .iter()
                        .map(|fill_data| fill_meta_data_to_proto(*fill_data))
                        .collect(),
                    symbol: symbol.clone(),
                    timestamp: timestamp.to_be_bytes().to_vec(),
                }),
                symbol,
                timestamp: timestamp.to_be_bytes().to_vec(),
            }.encode_to_vec(), "PartialFillOrder"),
        FillResult::Failed => 
            (GenericMessage {
                message: "failed to place order".to_string(),
                symbol,
                timestamp: timestamp.to_be_bytes().to_vec(),
            }.encode_to_vec(), "GenericMessage"),
    }
}

fn modify_result_to_proto<'a>(
    modify_result: ModifyResult, 
    symbol: String, 
    timestamp: u128
) -> (Vec<u8>, &'a str) {
    match modify_result {
        ModifyResult::Created(fill_result) =>
            fill_result_to_proto(fill_result, symbol, timestamp),
        ModifyResult::Modified(id) =>
            (CancelModifyOrder {
                status: 3,
                order_id: id.to_be_bytes().to_vec(),
                symbol,
                timestamp: timestamp.to_be_bytes().to_vec(),
            }.encode_to_vec(), "CancelModifyOrder"),
        ModifyResult::Failed => 
            (GenericMessage {
                message: "failed to modify order".to_string(),
                symbol,
                timestamp: timestamp.to_be_bytes().to_vec()
            }.encode_to_vec(), "GenericMessage"),
    }
}

fn limit_to_proto(
    limit_order: LimitOrder, 
    symbol: String, 
    timestamp:  u128
) -> CreateOrder {
    CreateOrder {
        status: 0,
        order_id: limit_order.id.to_be_bytes().to_vec(),
        price: limit_order.price,
        quantity: limit_order.quantity,
        side: limit_order.side as i32,
        symbol,
        timestamp: timestamp.to_be_bytes().to_vec(),
    }
}

fn fill_meta_data_to_proto(fill_meta_data: FillMetaData) -> FillOrderData {
    FillOrderData {
        order_id: fill_meta_data.order_id.to_be_bytes().to_vec(),
        matched_order_id: fill_meta_data.matched_order_id.to_be_bytes().to_vec(),
        taker_side: fill_meta_data.taker_side as i32,
        price: fill_meta_data.price,
        amount: fill_meta_data.quantity,
    }
}