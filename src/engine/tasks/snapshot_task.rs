use std::sync::Arc;
use std::time::Duration;
use tokio::sync::Notify;
use tokio::time::sleep;
use tracing::info;
use crate::engine::services::orderbook_manager_service::OrderbookManager;

pub struct Snapshot {
    pub shutdown_notification: Arc<Notify>,
    pub orderbook_manager: Arc<OrderbookManager>
}

impl Snapshot {
    pub fn new(shutdown_notification: Arc<Notify>, orderbook_manager: Arc<OrderbookManager>) -> Self {
        Self {
            shutdown_notification,
            orderbook_manager
        }
    }

    pub async fn run(&self) {
        loop {
            tokio::select! {
                _ = self.shutdown_notification.notified() => {
                    info!("shutting down snapshot_task");
                    break;
                },
                _ = sleep(Duration::from_millis(250)) => {
                    self.orderbook_manager.snapshot();
                }
            }
        }
    }
}