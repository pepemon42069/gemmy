use crate::engine::constants::property_loader::ServerProperties;

pub struct ServerConfiguration {
    pub server_properties: ServerProperties
}

impl ServerConfiguration {
    pub fn load(server_properties: ServerProperties) -> ServerConfiguration {
        ServerConfiguration {
            server_properties
        }
    }
}