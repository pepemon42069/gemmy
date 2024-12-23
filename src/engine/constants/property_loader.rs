use std::error::Error;
use std::net::SocketAddr;
use std::time::Duration;
use dotenv::dotenv;

pub struct ServerProperties {
    pub socket_address: SocketAddr,
    pub rfq_max_count: usize,
    pub rfq_buffer_size: usize,
    pub order_exec_batch_size: usize,
    pub order_exec_batch_timeout: Duration,
    pub orderbook_ticker: String,
    pub orderbook_queue_capacity: usize,
    pub orderbook_store_capacity: usize,
    pub orderbook_snapshot_interval: Duration,
}

pub struct KafkaAdminProperties {
    pub kafka_broker_address: String,
}

pub struct KafkaProducerProperties {
    pub message_timeout: String,
    pub acks: String,
    pub batch_size : String,
    pub linger_ms : String,
    pub compression_type : String,
    pub retries : String,
    pub retry_backoff : String,
    pub delivery_timeout : String,
    pub enable_idempotence : String,
}

pub struct LogProperties {
    pub enable_file_log: bool,
}

pub struct EnvironmentProperties {
    pub server_properties: ServerProperties,
    pub kafka_admin_properties: KafkaAdminProperties,
    pub kafka_producer_properties: KafkaProducerProperties,
    pub log_properties: LogProperties,
}

impl EnvironmentProperties {
    pub fn load() -> Result<Self, Box<dyn Error>> {
        dotenv().ok();
        let properties = Self {
            server_properties: ServerProperties {
                socket_address: std::env::var("GRPC_SOCKET_ADDRESS")?.parse()?,
                rfq_max_count: std::env::var("RFQ_MAX_COUNT")?.parse()?,
                rfq_buffer_size: std::env::var("RFQ_BUFFER_SIZE")?.parse()?,
                order_exec_batch_size: std::env::var("ORDER_EXEC_BATCH_SIZE")?.parse()?,
                order_exec_batch_timeout: Duration::from_millis(std::env::var("ORDER_EXEC_BATCH_TIMEOUT")?.parse()?),
                orderbook_ticker: std::env::var("TICKER")?.parse()?,
                orderbook_queue_capacity: std::env::var("ORDERBOOK_QUEUE_CAPACITY")?.parse()?,
                orderbook_store_capacity: std::env::var("ORDERBOOK_STORE_CAPACITY")?.parse()?,
                orderbook_snapshot_interval: Duration::from_millis(std::env::var("ORDERBOOK_SNAPSHOT_INTERVAL_MILLIS")?.parse()?),
            },
            kafka_admin_properties: KafkaAdminProperties {
                kafka_broker_address: std::env::var("KAFKA_BROKER_ADDRESS")?.parse()?
            },
            kafka_producer_properties: KafkaProducerProperties {
                message_timeout: std::env::var("KAFKA_PRODUCER_MESSAGE_TIMEOUT_MILLIS")?.parse()?,
                acks: std::env::var("KAFKA_ACKS")?.parse()?,
                batch_size : std::env::var("KAFKA_BATCH_SIZE")?.parse()?,
                linger_ms : std::env::var("KAFKA_LINGER_MILLIS")?.parse()?,
                compression_type : std::env::var("KAFKA_COMPRESSION_TYPE")?.parse()?,
                retries : std::env::var("KAFKA_RETRIES")?.parse()?,
                retry_backoff : std::env::var("KAFKA_RETRY_BACKOFF_MILLIS")?.parse()?,
                delivery_timeout : std::env::var("KAFKA_DELIVERY_TIMEOUT_MILLIS")?.parse()?,
                enable_idempotence : std::env::var("KAFKA_ENABLE_IDEMPOTENCE")?.parse()?,
            },
            log_properties: LogProperties {
                enable_file_log: std::env::var("ENABLE_FILE_LOG")?.parse()?
            }
        };
        Ok(properties)
    }
}