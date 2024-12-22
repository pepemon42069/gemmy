use std::sync::Arc;
use std::time::Duration;
use prost::Message;
use rdkafka::producer::{FutureProducer, FutureRecord};
use rdkafka::util::Timeout;
use tokio::sync::mpsc::Receiver;
use tokio::sync::Notify;
use tracing::{error, info};
use crate::core::models::{ExecutionResult, Operation, ProtoBuf, ProtoBufResult};
use crate::engine::services::orderbook_manager_service::OrderbookManager;

pub struct Executor {
    pub batch_size: usize,
    pub batch_timeout: Duration,
    pub shutdown_notification: Arc<Notify>,
    pub orderbook_manager: Arc<OrderbookManager>,
    pub kafka_producer: Arc<FutureProducer>,
    pub rx: Receiver<Operation>
}

unsafe impl Send for Executor {}
unsafe impl Sync for Executor {}

impl Executor {
    pub fn new(
        batch_size: usize,
        batch_timeout: Duration,
        shutdown_notification: Arc<Notify>,
        orderbook_manager: Arc<OrderbookManager>,
        kafka_producer: Arc<FutureProducer>,
        rx: Receiver<Operation>
    ) -> Self {
        Self {
            batch_size,
            batch_timeout,
            shutdown_notification,
            orderbook_manager,
            kafka_producer,
            rx
        }
    }
    
    pub async fn run(&mut self) {
        let mut batch = Vec::with_capacity(self.batch_size);
        let mut batch_timer = tokio::time::interval(self.batch_timeout);
        loop {
            tokio::select! {
                Some(order) = self.rx.recv() => {
                    batch.push(order);
                    if batch.len() >= self.batch_size {
                        process_batch(&batch, &self.orderbook_manager, &self.kafka_producer).await;
                        batch.clear();
                    }
                }
                _ = batch_timer.tick() => {
                    if !batch.is_empty() {
                        process_batch(&batch, &self.orderbook_manager, &self.kafka_producer).await;
                        batch.clear();
                    }
                }
                _ = self.shutdown_notification.notified() => {
                    info!("shutting down order_exec_task");
                    break;
                }
            }
        }
    }
}

async fn process_batch(batch: &[Operation], manager: &Arc<OrderbookManager>, kafka_producer: &Arc<FutureProducer>) {
    let primary = manager.get_primary();
    if primary.is_null() {
        error!("Error: primary order book pointer is null");
        return;
    }
    for order in batch {
        let result = unsafe { (*primary).execute(*order) };
        tokio::spawn(send_to_kafka(result, Arc::clone(kafka_producer)));
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