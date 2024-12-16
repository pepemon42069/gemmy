use prost::Message;
use tokio::sync::mpsc::Receiver;
use tokio::task::JoinHandle;
use tracing::info;
use crate::core::models::{ExecutionResult, Operation, ProtoBuf, ProtoBufResult};
use crate::core::orderbook::OrderBook;

pub fn executor(rx: Receiver<Operation>, mut orderbook: OrderBook) -> JoinHandle<()> {
    let mut rx = rx;
    tokio::spawn(async move {
        while let Some(order) = rx.recv().await {
            let result = orderbook.execute(order);
            tokio::spawn(send_to_kafka(result));
        }
    })
}

async fn send_to_kafka(execution_result: ExecutionResult) {
    let protobuf = execution_result.to_protobuf();
    info!("protobuf: {:#?}", protobuf);
    let encoded_data = encode_protobuf(protobuf);
    info!("encoded_data: {:?}", encoded_data);
}

fn encode_protobuf(protobuf: ProtoBufResult) -> Vec<u8> {
    let (mut encoded_data, status) = match protobuf {
        ProtoBufResult::Create(create_order) => (create_order.encode_to_vec(), 0),
        ProtoBufResult::Fill(fill_order) => (fill_order.encode_to_vec(), 1),
        ProtoBufResult::PartialFill(partial_fill_order) => (partial_fill_order.encode_to_vec(), 2),
        ProtoBufResult::CancelModify(cancel_modify_order) => (cancel_modify_order.encode_to_vec(), 3),
        ProtoBufResult::Failed(generic_message) => (generic_message.encode_to_vec(), 4),
    };
    encoded_data.push(status);
    encoded_data
}