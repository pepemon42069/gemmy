use std::sync::Arc;
use tokio::signal;
use tokio::sync::Notify;
use tracing::info;

pub struct Shutdown {
    pub shutdown_notification: Arc<Notify>
}

unsafe impl Send for Shutdown {}
unsafe impl Sync for Shutdown {}

impl Shutdown {
    pub fn new(shutdown_notification: Arc<Notify>) -> Self {
        Self { shutdown_notification }
    }
    pub async fn run(&self) {
        signal::ctrl_c().await.expect("failed to listen for shutdown signal");
        info!("shutdown signal received");
        self.shutdown_notification.notify_waiters();
        info!("notified all waiters for shutdown");
    }
}