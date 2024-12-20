use rdkafka::ClientConfig;
use rdkafka::error::KafkaError;
use rdkafka::producer::FutureProducer;
use crate::engine::constants::loader::{KafkaAdminProperties, KafkaProducerProperties};

pub struct KafkaConfiguration {
    pub kafka_admin_properties: KafkaAdminProperties,
    pub kafka_producer_properties: KafkaProducerProperties
}
impl KafkaConfiguration {
    pub fn producer(&self) -> Result<FutureProducer, KafkaError> {
        ClientConfig::new()
            .set("bootstrap.servers", &self.kafka_admin_properties.kafka_broker_address)
            .set("message.timeout.ms", &self.kafka_producer_properties.message_timeout)
            .set("acks", &self.kafka_producer_properties.acks)
            .create()
    }
}