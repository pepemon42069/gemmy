use tracing_appender::non_blocking::WorkerGuard;
use tracing_appender::rolling::{RollingFileAppender, Rotation};
use crate::engine::constants::property_loader::LogProperties;

pub struct LogConfiguration {
    pub log_properties: LogProperties,
    pub worker_guard: Option<WorkerGuard>
}

impl LogConfiguration {
    pub fn load(log_properties: LogProperties) -> LogConfiguration {
        let mut worker_guard = None;
        if log_properties.enable_file_log {
            let file_appender = RollingFileAppender::new(
                Rotation::DAILY, "log", "gemmy.log");
            let (file_writer, guard) = 
                tracing_appender::non_blocking(file_appender);
            tracing_subscriber::fmt()
                .with_ansi(false)
                .with_max_level(tracing::Level::INFO)
                .with_writer(file_writer)
                .init();
            worker_guard = Some(guard);
        } else {
            tracing_subscriber::fmt()
                .with_ansi(true)
                .with_max_level(tracing::Level::INFO)
                .init();
        }
        LogConfiguration { log_properties, worker_guard }
    }
}