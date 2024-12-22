use gemmy::engine::services::{
    order_dispatch_service::OrderDispatchService,
    stat_stream_service::StatStreamer,
};
use std::{error::Error, sync::Arc};
use tracing::{error, info};
use gemmy::engine::configuration::configuration_loader::ConfigurationLoader;
use gemmy::engine::state::server_state::ServerState;
use gemmy::engine::tasks::task_manager::TaskManager;

#[tokio::main]
pub async fn main() -> Result<(), Box<dyn Error>> {
    
    // load configurations
    let ConfigurationLoader {
        server_configuration,
        kafka_configuration,
        ..
    } = ConfigurationLoader::load()?;
    
    // initialize server state
    let state = ServerState::init(kafka_configuration)?;
    
    // initialize task manager and register tasks
    let mut task_manager = TaskManager::init(
        Arc::clone(&state.shutdown_notification), 
        Arc::clone(&state.orderbook_manager)
    );

    // create services
    let order_dispatcher_service = OrderDispatchService::create(
        server_configuration.server_properties.order_exec_batch_size,
        server_configuration.server_properties.order_exec_batch_timeout,
        Arc::clone(&state.shutdown_notification), 
        Arc::clone(&state.orderbook_manager), 
        Arc::clone(&state.kafka_producer), 
        &mut task_manager
    );
    
    let stat_streamer_service = StatStreamer::create(
        server_configuration.server_properties.rfq_max_count,
        server_configuration.server_properties.rfq_buffer_size, 
        Arc::clone(&state.orderbook_manager)
    );
    
    // start the server thread
    let server = tonic::transport::Server::builder()
        .add_service(order_dispatcher_service)
        .add_service(stat_streamer_service)
        .serve_with_shutdown(server_configuration.server_properties.socket_address, async {
            info!("successfully started gRPC server at: {}", server_configuration.server_properties.socket_address);
            task_manager.deregister("shutdown_task").await.expect("failed to shut down server");
        });

    // handle graceful shutdown
    tokio::select! {
        result = server => {
            if let Err(e) = result {
                error!("error while starting server: {}", e);
            }
        },
        _ = state.shutdown_notification.notified() => {
            info!("initiating server shutdown");
            task_manager.deregister("order_exec_task").await.expect("failed to shut down order executor task");
            task_manager.deregister("snapshot_task").await.expect("failed to shut down snapshot task");
        },
    }

    info!("gRPC server stopped gracefully");

    Ok(())
}
