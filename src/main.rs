use std::{env, error::Error, sync::Arc};
use std::time::Duration;
use dotenv::dotenv;
use rdkafka::ClientConfig;
use rdkafka::producer::FutureProducer;
use tokio::{signal, sync::{Notify, mpsc}};
use tokio::time::sleep;
use tonic::transport::Server;
use tracing::{error, info};
use tracing_appender::{
    non_blocking::WorkerGuard,
    rolling::{RollingFileAppender, Rotation},
};
use gemmy::engine::services::{
    order_dispatcher::OrderDispatchService,
    manager::Manager,
    order_executor::executor,
    stat_streamer::StatStreamer,
};

fn configure_logging(enable_file_log: bool) -> Option<Arc<WorkerGuard>> {
    if enable_file_log {
        let file_appender = RollingFileAppender::new(
            Rotation::DAILY, "log", "gemmy.log");
        let (file_writer, guard) = 
            tracing_appender::non_blocking(file_appender);
        tracing_subscriber::fmt()
            .with_ansi(false)
            .with_max_level(tracing::Level::INFO)
            .with_writer(file_writer)
            .init();
        Some(Arc::new(guard))
    } else {
        tracing_subscriber::fmt()
            .with_ansi(true)
            .with_max_level(tracing::Level::INFO)
            .init();
        None
    }
}

#[tokio::main]
pub async fn main() -> Result<(), Box<dyn Error>> {
    // environment variables
    dotenv().ok();
    let address = env::var("SOCKET_ADDRESS")?.parse()?;
    let enable_file_log = env::var("FILE_LOGGING")?.parse()?;

    // log configuration
    let _guard = configure_logging(enable_file_log);

    // Kafka producer configuration
    let kafka_producer: Arc<FutureProducer>  = Arc::new(ClientConfig::new()
        .set("bootstrap.servers", env::var("KAFKA_PRODUCER")?)
        .set("message.timeout.ms", "5000")
        .set("acks", "all")
        .create()
        .map_err(|e| {
            error!("Failed to create Kafka producer: {}", e);
            Box::new(e) as Box<dyn Error>
        })?);

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
                        // info!("updated snapshot");
                    }
                }
            }
        })
    };

    // graceful shutdown task
    let shutdown_task = {
        let shutdown_notify_task_ref = Arc::clone(&shutdown_notify);
        tokio::spawn(async move {
            signal::ctrl_c().await.expect("failed to listen for shutdown signal");
            info!("shutdown signal received");
            shutdown_notify_task_ref.notify_waiters();
        })
    };

    // service configuration
    let server = Server::builder()
        .add_service(OrderDispatchService::create_no_interceptor(tx))
        .add_service(StatStreamer::create(10, 10, reader))
        .serve_with_shutdown(address, async {
            shutdown_task.await.expect("failed to shut down");
            info!("shutdown complete");
        });

    // starting the server and handling shutdown ops
    tokio::select! {
        result = server => {
            if let Err(e) = result {
                error!("error while starting server: {}", e);
            }
            info!("started gRPC server at: {}", address);
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