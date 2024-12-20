use std::error::Error;
use std::sync::Arc;
use crate::engine::configuration::kafka::KafkaConfiguration;
use crate::engine::configuration::logs::LogConfiguration;
use crate::engine::configuration::server::ServerConfiguration;
use crate::engine::constants::loader::EnvironmentProperties;

pub struct ConfigurationLoader {
    pub server_configuration: Arc<ServerConfiguration>,
    pub log_configuration: Arc<LogConfiguration>,
    pub kafka_configuration: Arc<KafkaConfiguration>
}

impl ConfigurationLoader {
    
    pub fn load() -> Result<Self, Box<dyn Error>> {
        // load environment variables
        let EnvironmentProperties {
            server_properties,
            kafka_admin_properties,
            kafka_producer_properties,
            log_properties
        } = EnvironmentProperties::load()?;
        
        // server configuration
        let server_configuration = Arc::new(ServerConfiguration::load(server_properties));

        // log configuration
        let log_configuration = Arc::new(LogConfiguration::load(log_properties));

        // kafka configuration & producer
        let kafka_configuration = Arc::new(KafkaConfiguration {
            kafka_admin_properties,
            kafka_producer_properties
        });
        
        Ok(ConfigurationLoader {
            server_configuration,
            log_configuration,
            kafka_configuration,
        })
    }
}