use std::collections::HashMap;
use std::future::Future;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::Notify;
use tokio::task::JoinHandle;
use tracing::info;
use crate::engine::services::orderbook_manager_service::OrderbookManager;
use crate::engine::tasks::shutdown_task::Shutdown;
use crate::engine::tasks::snapshot_task::Snapshot;

pub struct TaskManager {
    tasks: HashMap<&'static str, JoinHandle<()>>,
}

impl TaskManager {
    pub fn init(
        shutdown_notification: Arc<Notify>,
        orderbook_manager: Arc<OrderbookManager>,
        snapshot_interval: Duration,
    ) -> Self {
        let mut task_manager = TaskManager { tasks: HashMap::new() };
        task_manager.register("shutdown_task", {
            let shutdown_notify = Arc::clone(&shutdown_notification);
            async move {
                Shutdown::new(shutdown_notify).run().await;
            }
        });
        task_manager.register("snapshot_task", {
            let shutdown_notify = Arc::clone(&shutdown_notification);
            let manager = Arc::clone(&orderbook_manager);
            async move {
                Snapshot::new(shutdown_notify, manager, snapshot_interval).run().await;
            }
        });
        task_manager
    }

    pub fn register<F>(&mut self, id: &'static str, task: F)
    where
        F: Future<Output = ()> + Send + 'static,
    {
        self.tasks.insert(id, tokio::spawn(task));
        info!("successfully registered task: {}", id);
    }

    pub fn deregister(&mut self, id: &'static str) -> JoinHandle<()> {
        self.tasks.remove(&id).unwrap()
    }
}