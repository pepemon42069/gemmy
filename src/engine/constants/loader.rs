use std::error::Error;
use std::net::SocketAddr;
use dotenv::dotenv;

pub struct ServerProperties {
    pub socket_address: SocketAddr,
}

pub struct KafkaAdminProperties {
    pub kafka_broker_address: String,
}

pub struct KafkaProducerProperties {
    pub message_timeout: String,
    pub acks: String,
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
            },
            kafka_admin_properties: KafkaAdminProperties {
                kafka_broker_address: std::env::var("KAFKA_BROKER_ADDRESS")?.parse()?
            },
            kafka_producer_properties: KafkaProducerProperties {
                message_timeout: std::env::var("KAFKA_PRODUCER_MESSAGE_TIMEOUT_MILLIS")?.parse()?,
                acks: std::env::var("KAFKA_ACKS")?.parse()?,
            },
            log_properties: LogProperties {
                enable_file_log: std::env::var("ENABLE_FILE_LOG")?.parse()?
            }
        };
        Ok(properties)
    }
}