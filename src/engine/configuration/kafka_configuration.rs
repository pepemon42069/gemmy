use crate::engine::constants::property_loader::{KafkaAdminProperties, KafkaProducerProperties};
use rdkafka::error::KafkaError;
use rdkafka::producer::FutureProducer;
use rdkafka::ClientConfig;

pub struct KafkaConfiguration {
    pub kafka_admin_properties: KafkaAdminProperties,
    pub kafka_producer_properties: KafkaProducerProperties,
}
impl KafkaConfiguration {
    pub fn producer(&self) -> Result<FutureProducer, KafkaError> {
        ClientConfig::new()
            .set(
                "bootstrap.servers",
                &self.kafka_admin_properties.kafka_broker_address,
            )
            .set(
                "message.timeout.ms",
                &self.kafka_producer_properties.message_timeout,
            )
            .set("acks", &self.kafka_producer_properties.acks)
            .set("batch.size", &self.kafka_producer_properties.batch_size)
            .set("linger.ms", &self.kafka_producer_properties.linger_ms)
            .set(
                "compression.type",
                &self.kafka_producer_properties.compression_type,
            )
            .set("retries", &self.kafka_producer_properties.retries)
            .set(
                "retry.backoff.ms",
                &self.kafka_producer_properties.retry_backoff,
            )
            .set(
                "delivery.timeout.ms",
                &self.kafka_producer_properties.delivery_timeout,
            )
            .create()
    }
}
