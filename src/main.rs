use gemmy::engine::services::{
    manager::Manager, order_dispatcher::OrderDispatchService, order_executor::executor,
    stat_streamer::StatStreamer,
};
use std::time::Duration;
use std::{error::Error, sync::Arc};
use tokio::time::sleep;
use tokio::{
    signal,
    sync::{mpsc, Notify},
};
use tonic::transport::Server;
use tracing::{error, info};
use gemmy::engine::configuration::{
    kafka::KafkaConfiguration,
    logs::LogConfiguration
};
use gemmy::engine::constants::property_loader::EnvironmentProperties;

#[tokio::main]
pub async fn main() -> Result<(), Box<dyn Error>> {
    // load environment variables
    let EnvironmentProperties { 
        server_properties, 
        kafka_admin_properties, 
        kafka_producer_properties, 
        log_properties
    } = EnvironmentProperties::load()?;

    // log configuration
    LogConfiguration::load(log_properties);
    
    // kafka configuration & producer
    let kafka_configuration = KafkaConfiguration { 
        kafka_admin_properties, 
        kafka_producer_properties 
    };
    let kafka_producer = Arc::new(kafka_configuration.producer()?);

    // mpsc setup
    let (tx, rx) = mpsc::channel(10000);
    let shutdown_notify = Arc::new(Notify::new());

    let manager = Arc::new(Manager::new());
    let writer = Arc::clone(&manager);
    let reader = Arc::clone(&manager);

    let order_executor = executor(rx, writer, kafka_producer, Arc::clone(&shutdown_notify));

    let snapshot_task = {
        let manager = Arc::clone(&manager);
        let shutdown_notify_clone = Arc::clone(&shutdown_notify);
        tokio::spawn(async move {
            loop {
                tokio::select! {
                    _ = shutdown_notify_clone.notified() => {
                        info!("shutting down snapshot task");
                        break;
                    },
                    _ = sleep(Duration::from_millis(250)) => {
                        manager.snapshot();
                    }
                }
            }
        })
    };

    // graceful shutdown task
    let shutdown_task = {
        let shutdown_notify_task_ref = Arc::clone(&shutdown_notify);
        tokio::spawn(async move {
            signal::ctrl_c()
                .await
                .expect("failed to listen for shutdown signal");
            info!("shutdown signal received");
            shutdown_notify_task_ref.notify_waiters();
        })
    };

    // service configuration
    let server = Server::builder()
        .add_service(OrderDispatchService::create_no_interceptor(tx))
        .add_service(StatStreamer::create(10, 10, reader))
        .serve_with_shutdown(server_properties.socket_address, async {
            shutdown_task.await.expect("failed to shut down");
            info!("shutdown complete");
        });

    // starting the server and handling shutdown ops
    tokio::select! {
        result = server => {
            if let Err(e) = result {
                error!("error while starting server: {}", e);
            }
            info!("started gRPC server at: {}", server_properties.socket_address);
        },
        _ = shutdown_notify.notified() => {
            info!("initiating server shutdown");
        },
    }

    if let Err(e) = order_executor.await {
        error!("error while shutting down order_executor: {}", e);
    }

    if let Err(e) = snapshot_task.await {
        error!("error while shutting down snapshot_task: {}", e);
    }

    info!("gRPC server stopped gracefully");

    Ok(())
}
