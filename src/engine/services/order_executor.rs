use crate::core::models::{ExecutionResult, Operation, ProtoBuf, ProtoBufResult};
use crate::engine::services::manager::Manager;
use prost::Message;
use rdkafka::{util::Timeout, producer::{FutureProducer, FutureRecord}};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::mpsc::Receiver;
use tokio::sync::Notify;
use tokio::task::JoinHandle;
use tracing::{info, error};

const BATCH_SIZE: usize = 10000;
const BATCH_TIMEOUT: Duration = Duration::from_millis(250);

pub fn executor(
    rx: Receiver<Operation>,
    manager: Arc<Manager>,
    kafka_producer: Arc<FutureProducer>,
    shutdown_notify: Arc<Notify>,
) -> JoinHandle<()> {
    let mut rx = rx;
    tokio::spawn(async move {
        let mut batch = Vec::with_capacity(BATCH_SIZE);
        let mut batch_timer = tokio::time::interval(BATCH_TIMEOUT);
        loop {
            tokio::select! {
                Some(order) = rx.recv() => {
                    batch.push(order);
                    if batch.len() >= BATCH_SIZE {
                        process_batch(&batch, &manager, &kafka_producer).await;
                        batch.clear();
                    }
                }
                _ = batch_timer.tick() => {
                    if !batch.is_empty() {
                        process_batch(&batch, &manager, &kafka_producer).await;
                        batch.clear();
                    }
                }
                _ = shutdown_notify.notified() => {
                    info!("shutting down executor");
                    break;
                }
            }
        }
    })
}

async fn process_batch(
    batch: &[Operation],
    manager: &Arc<Manager>,
    kafka_producer: &Arc<FutureProducer>,
) {
    let primary = manager.get_primary();
    if primary.is_null() {
        error!("Error: primary order book pointer is null");
        return;
    }
    for order in batch {
        let result = unsafe { (*primary).execute(order.clone()) };
        tokio::spawn(send_to_kafka(result, Arc::clone(&kafka_producer)));
    }
}

async fn send_to_kafka(execution_result: ExecutionResult, kafka_producer: Arc<FutureProducer>) {
    let protobuf = execution_result.to_protobuf();
    let encoded_data = encode_protobuf(protobuf);
    let delivery_result = kafka_producer
        .send(
            FutureRecord::<(), Vec<u8>>::to("order-topic").payload(&encoded_data),
            Timeout::After(Duration::new(5, 0)),
        )
        .await;
    match delivery_result {
        Ok(_) => info!("Successfully sent message"),
        Err((e, _)) => {
            error!("Error sending message: {:?}", e);
        }
    }
}

fn encode_protobuf(protobuf: ProtoBufResult) -> Vec<u8> {
    let (mut encoded_data, status) = match protobuf {
        ProtoBufResult::Create(create_order) => (create_order.encode_to_vec(), 0),
        ProtoBufResult::Fill(fill_order) => (fill_order.encode_to_vec(), 1),
        ProtoBufResult::PartialFill(partial_fill_order) => (partial_fill_order.encode_to_vec(), 2),
        ProtoBufResult::CancelModify(cancel_modify_order) => {
            (cancel_modify_order.encode_to_vec(), 3)
        }
        ProtoBufResult::Failed(generic_message) => (generic_message.encode_to_vec(), 4),
    };
    encoded_data.push(status);
    encoded_data
}
