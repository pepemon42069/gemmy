use crate::engine::services::orderbook_manager_service::OrderbookManager;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::Notify;
use tokio::time::sleep;
use tracing::info;

pub struct Snapshot {
    pub shutdown_notification: Arc<Notify>,
    pub orderbook_manager: Arc<OrderbookManager>,
    pub snapshot_interval: Duration,
}

impl Snapshot {
    pub fn new(
        shutdown_notification: Arc<Notify>,
        orderbook_manager: Arc<OrderbookManager>,
        snapshot_interval: Duration,
    ) -> Self {
        Self {
            shutdown_notification,
            orderbook_manager,
            snapshot_interval,
        }
    }

    pub async fn run(&self) {
        loop {
            tokio::select! {
                _ = self.shutdown_notification.notified() => {
                    info!("shutting down snapshot_task");
                    break;
                },
                _ = sleep(self.snapshot_interval) => {
                    self.orderbook_manager.snapshot();
                }
            }
        }
    }
}
