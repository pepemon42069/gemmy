use std::sync::Arc;
use rdkafka::error::KafkaError;
use rdkafka::producer::FutureProducer;
use tokio::sync::Notify;
use crate::engine::configuration::kafka_configuration::KafkaConfiguration;
use crate::engine::configuration::server_configuration::ServerConfiguration;
use crate::engine::services::orderbook_manager_service::OrderbookManager;

pub struct ServerState {
    pub shutdown_notification: Arc<Notify>,
    pub orderbook_manager: Arc<OrderbookManager>,
    pub kafka_producer: Arc<FutureProducer>
}

impl ServerState {
    pub fn init(
        server_configuration: Arc<ServerConfiguration>, 
        kafka_configuration: Arc<KafkaConfiguration>
    ) -> Result<ServerState, KafkaError> {
        let shutdown_notification = Arc::new(Notify::new());
        let orderbook_manager = Arc::new(OrderbookManager::new(
            server_configuration.server_properties.orderbook_ticker.clone(),
            server_configuration.server_properties.orderbook_queue_capacity,
            server_configuration.server_properties.orderbook_store_capacity
        ));
        let kafka_producer = Arc::new(kafka_configuration.producer()?);
        Ok(ServerState { shutdown_notification, orderbook_manager, kafka_producer })
    }
}