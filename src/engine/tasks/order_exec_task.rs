use std::sync::Arc;
use std::time::Duration;
use prost::Message;
use rdkafka::producer::{FutureProducer, FutureRecord};
use rdkafka::util::Timeout;
use schema_registry_converter::async_impl::proto_raw::ProtoRawEncoder;
use schema_registry_converter::async_impl::schema_registry::SrSettings;
use schema_registry_converter::schema_registry_common::SubjectNameStrategy;
use tokio::sync::mpsc::Receiver;
use tokio::sync::Notify;
use tracing::{error, info};
use crate::core::models::{Operation, ProtoBufResult};
use crate::core::utils::generate_u128_timestamp;
use crate::engine::configuration::kafka_configuration::KafkaConfiguration;
use crate::engine::configuration::server_configuration::ServerConfiguration;
use crate::engine::services::orderbook_manager_service::OrderbookManager;
use crate::engine::state::server_state::ServerState;

pub struct Executor {
    pub batch_size: usize,
    pub batch_timeout: Duration,
    pub shutdown_notification: Arc<Notify>,
    pub orderbook_manager: Arc<OrderbookManager>,
    pub kafka_topic: String,
    pub kafka_producer: Arc<FutureProducer>,
    pub sr_settings : Arc<SrSettings>,
    pub rx: Receiver<Operation>

}

impl Executor {
    pub fn new(
        server_configuration: Arc<ServerConfiguration>,
        kafka_configuration: Arc<KafkaConfiguration>,
        state: Arc<ServerState>,
        rx: Receiver<Operation>
    ) -> Executor {
        Self {
            batch_size: server_configuration.server_properties.order_exec_batch_size,
            batch_timeout: server_configuration.server_properties.order_exec_batch_timeout,
            shutdown_notification: Arc::clone(&state.shutdown_notification),
            orderbook_manager: Arc::clone(&state.orderbook_manager),
            kafka_topic: kafka_configuration.kafka_admin_properties.kafka_topic.clone(),
            kafka_producer: Arc::clone(&state.kafka_producer),
            sr_settings: Arc::clone(&kafka_configuration.kafka_admin_properties.sr_settings),
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
                        self.process_batch(&batch).await;
                        batch.clear();
                    }
                }
                _ = batch_timer.tick() => {
                    if !batch.is_empty() {
                        self.process_batch(&batch).await;
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

    async fn process_batch(&self, batch: &[Operation]) {
        let primary = self.orderbook_manager.get_primary();
        let id = unsafe { (*primary).get_id()};
        let mut results = vec![];
        for order in batch {
            results.push((unsafe { (*primary).execute(*order) }, generate_u128_timestamp()));
        }
        let kafka_producer = self.kafka_producer.clone();
        let kafka_topic = self.kafka_topic.clone();
        let proto_raw_encoder = ProtoRawEncoder::new(
            self.sr_settings.as_ref().clone());
        tokio::spawn(async move {
            for (result, timestamp) in results {
                let protobuf = result.to_protobuf(id.clone(), timestamp);
                let encoded_data = encode_protobuf(protobuf, &proto_raw_encoder).await;
                let delivery_result = kafka_producer
                    .send(
                        FutureRecord::<(), Vec<u8>>::to(kafka_topic.as_str())
                            .payload(&encoded_data),
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
        });
    }
}

async fn encode_protobuf<'a>(
    protobuf: ProtoBufResult , 
    proto_raw_encoder: &ProtoRawEncoder<'a> ) -> Vec<u8> {
    let (encoded_data, schema_name) = match protobuf {
        ProtoBufResult::Create(create_order) => {
            (create_order.encode_to_vec(), "CreateOrder")
        },
        ProtoBufResult::Fill(fill_order) => {
            (fill_order.encode_to_vec(), "FillOrder")
        },
        ProtoBufResult::PartialFill(partial_fill_order) => {
            (partial_fill_order.encode_to_vec(), "PartialFillOrder")
        },
        ProtoBufResult::CancelModify(cancel_modify_order) => {
            (cancel_modify_order.encode_to_vec(), "CancelModifyOrder")
        },
        ProtoBufResult::Failed(generic_message) => {
            (generic_message.encode_to_vec(), "GenericMessage")
        },
    };
    proto_raw_encoder.encode(
        &encoded_data,
        format!("models.{}", &schema_name).as_str(),
        SubjectNameStrategy::RecordNameStrategy("models".to_string())
    ).await.unwrap()
}