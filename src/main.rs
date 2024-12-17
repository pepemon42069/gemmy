use std::{env, error::Error, sync::Arc};
use dotenv::dotenv;
use tokio::{signal, sync::{Notify, mpsc}};
use tokio::sync::RwLock;
use tonic::transport::Server;
use tracing::{error, info};
use tracing_appender::{
    non_blocking::WorkerGuard,
    rolling::{RollingFileAppender, Rotation}
};
use gemmy::core::orderbook::OrderBook;
use gemmy::engine::services::{
    order_dispatcher::OrderDispatchService
};
use gemmy::engine::services::order_executor::executor;
use gemmy::engine::services::stat_streamer::StatStreamer;

fn configure_logging() -> Arc<WorkerGuard> {
    let file_appender = RollingFileAppender::new(
        Rotation::DAILY, "log", "gemmy.log");
    let (file_writer, guard) = tracing_appender::non_blocking(file_appender);
    tracing_subscriber::fmt()
        .with_ansi(false)
        .with_max_level(tracing::Level::INFO)
        .with_writer(file_writer)
        .init();
    Arc::new(guard)
}

#[tokio::main]
pub async fn main() -> Result<(), Box<dyn Error>> {
    // environment variables
    dotenv().ok();
    let address = env::var("SOCKET_ADDRESS")?.parse()?;
    let file_logging = env::var("FILE_LOGGING").unwrap_or_else(|_| "false".to_string());
    
    // log configuration
    let _guard;
    if file_logging == "true" {
        _guard = configure_logging();
    } else {
        tracing_subscriber::fmt()
            .with_ansi(true)
            .with_max_level(tracing::Level::INFO)
            .init();
    }

    // mpsc setup
    let (tx, rx) = mpsc::channel(10000);
    
    let orderbook = Arc::new(RwLock::new(OrderBook::default()));
    let orderbook_clone = Arc::clone(&orderbook);
    
    let order_executor = executor(rx, orderbook);

    // graceful shutdown configuration
    let shutdown_notify = Arc::new(Notify::new());
    let shutdown_notify_clone = Arc::clone(&shutdown_notify);
    let shutdown_signal = async {
        signal::ctrl_c().await.expect("failed to listen for shutdown signal");
        info!("shutdown signal received");
        shutdown_notify.notify_waiters();
    };

    // service configuration
    let server = Server::builder()
        .add_service(OrderDispatchService::create(tx))
        .add_service(StatStreamer::create(10, 10, orderbook_clone))
        .serve_with_shutdown(address, async {
            shutdown_signal.await;
        });
    
    // starting the server and handling shutdown ops
    tokio::select! {
        result = server => {
            if let Err(e) = result {
                error!("error while starting server: {}", e);
            }
            info!("started gRPC server at: {}", address);
        },
        _ = shutdown_notify_clone.notified() => {
            info!("initiating server shutdown");
        },
    }
    
    if let Err(e) = order_executor.await {
        error!("error while shutting down counter_processor: {}", e);
    }

    info!("gRPC server stopped gracefully");

    Ok(())
}