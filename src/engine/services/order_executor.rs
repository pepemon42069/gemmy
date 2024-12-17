use std::sync::Arc;
use std::time::Duration;
use prost::Message;
use rdkafka::producer::{FutureProducer, FutureRecord};
use rdkafka::util::Timeout;
use tokio::sync::mpsc::Receiver;
use tokio::sync::RwLock;
use tokio::task::JoinHandle;
use crate::core::models::{ExecutionResult, Operation, ProtoBuf, ProtoBufResult};
use crate::core::orderbook::OrderBook;

pub fn executor(rx: Receiver<Operation>, mut orderbook: Arc<RwLock<OrderBook>>, kafka_producer: Arc<FutureProducer>) -> JoinHandle<()> {
    let mut rx = rx;
    tokio::spawn(async move {
        while let Some(order) = rx.recv().await {
            let result = orderbook.write().await.execute(order);
            tokio::spawn(send_to_kafka(result, Arc::clone(&kafka_producer)));
        }
    })
}

async fn send_to_kafka(execution_result: ExecutionResult, kafka_producer: Arc<FutureProducer>) {
    let protobuf = execution_result.to_protobuf();
    let encoded_data = encode_protobuf(protobuf);
    let delivery_result = kafka_producer
        .send(FutureRecord::<(), Vec<u8>>::to("order-topic")
                  .payload(&encoded_data), Timeout::After(Duration::new(5, 0))).await;
    match delivery_result {
        Ok(_) => println!("Successfully sent message"),
        Err((e, _)) => {
            println!("Error sending message: {:?}", e);
        }
    }
}

fn encode_protobuf(protobuf: ProtoBufResult) -> Vec<u8> {
    let (mut encoded_data, status) = match protobuf {
        ProtoBufResult::Create(create_order) => (create_order.encode_to_vec(), 0),
        ProtoBufResult::Fill(fill_order) => (fill_order.encode_to_vec(), 1),
        ProtoBufResult::PartialFill(partial_fill_order) => (partial_fill_order.encode_to_vec(), 2),
        ProtoBufResult::CancelModify(cancel_modify_order) => (cancel_modify_order.encode_to_vec(), 3),
        ProtoBufResult::Failed(generic_message) => (generic_message.encode_to_vec(), 4)
    };
    encoded_data.push(status);
    encoded_data
}