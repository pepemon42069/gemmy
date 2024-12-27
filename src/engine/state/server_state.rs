use rdkafka::producer::FutureProducer;
use schema_registry_converter::async_impl::schema_registry::post_schema;
use schema_registry_converter::schema_registry_common::{SchemaType, SuppliedSchema};
use std::error::Error;
use std::fs;
use std::sync::Arc;
use rdkafka::admin::{AdminClient, AdminOptions, NewTopic, TopicReplication};
use rdkafka::client::DefaultClientContext;
use rdkafka::error::KafkaError;
use crate::engine::configuration::kafka_configuration::KafkaConfiguration;
use crate::engine::configuration::server_configuration::ServerConfiguration;
use crate::engine::services::orderbook_manager_service::OrderbookManager;
use tokio::sync::Notify;
use tracing::info;

pub struct ServerState {
    pub shutdown_notification: Arc<Notify>,
    pub orderbook_manager: Arc<OrderbookManager>,
    pub kafka_producer: Arc<FutureProducer>,
    pub kafka_admin_client: Arc<AdminClient<DefaultClientContext>>
}

impl ServerState {
    pub async fn init(
        server_configuration: Arc<ServerConfiguration>,
        kafka_configuration: Arc<KafkaConfiguration>,
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
            "models".to_string(),
            schema,
        )
        .await?;
        info!("successfully registered schemas");

        let shutdown_notification = Arc::new(Notify::new());
        let orderbook_manager = Arc::new(OrderbookManager::new(
            server_configuration
                .server_properties
                .orderbook_ticker
                .clone(),
            server_configuration
                .server_properties
                .orderbook_queue_capacity,
            server_configuration
                .server_properties
                .orderbook_store_capacity,
        ));

        let kafka_producer = Arc::new(kafka_configuration.producer()?);
        let kafka_admin_client = Arc::new(kafka_configuration.admin_client()?);

        check_and_create_topics(
            Arc::clone(&kafka_admin_client),
            kafka_configuration.kafka_admin_properties.kafka_topic.as_str(),
        ).await?;

        Ok(ServerState {
            shutdown_notification,
            orderbook_manager,
            kafka_producer,
            kafka_admin_client,
        })
    }
}


async fn check_and_create_topics(
    admin_client: Arc<AdminClient<DefaultClientContext>>, 
    topic: &str
) -> Result<(), KafkaError> {
    let topics = vec![
        NewTopic::new(topic, 1, TopicReplication::Fixed(1))
    ];
    match admin_client.create_topics(&topics, &AdminOptions::default()).await {
        Ok(topic_results) => {
            topic_results.iter().for_each(|res| {
                info!("kafka topic status: {:?}", res);
            });
            Ok(())
        }
        Err(e) => Err(e)
    }
}