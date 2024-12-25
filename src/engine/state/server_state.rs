use std::error::Error;
use std::fs;
use std::sync::Arc;
use rdkafka::producer::FutureProducer;
use schema_registry_converter::async_impl::schema_registry::{post_schema};
use schema_registry_converter::schema_registry_common::{SchemaType, SuppliedSchema};

use tokio::sync::Notify;
use crate::engine::configuration::kafka_configuration::KafkaConfiguration;
use crate::engine::configuration::server_configuration::ServerConfiguration;
use crate::engine::services::orderbook_manager_service::OrderbookManager;

pub struct ServerState {
    pub shutdown_notification: Arc<Notify>,
    pub orderbook_manager: Arc<OrderbookManager>,
    pub kafka_producer: Arc<FutureProducer>,
}

impl ServerState {
    pub async fn init(
        server_configuration: Arc<ServerConfiguration>,
        kafka_configuration: Arc<KafkaConfiguration>
    ) -> Result<ServerState, Box<dyn Error>> {


        let proto = fs::read_to_string("resources/protobuf/models.proto")?;
        let schema = SuppliedSchema {
            name: Some("models.proto".to_string()),
            schema_type: SchemaType::Protobuf,
            schema: proto.to_string(),
            references: vec![],
        };
        post_schema(
            &kafka_configuration.kafka_admin_properties.sr_settings, 
            "models".to_string(), schema
        ).await?;

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