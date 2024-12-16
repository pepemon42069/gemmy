use tokio::sync::mpsc::Receiver;
use tokio::task::JoinHandle;
use tracing::info;
use crate::core::models::Operation;
use crate::core::orderbook::OrderBook;

pub fn executor(rx: Receiver<Operation>, mut orderbook: OrderBook) -> JoinHandle<()> {
    let mut rx = rx;
    tokio::spawn(async move {
        while let Some(order) = rx.recv().await {
            info!("starting order executor : {:#?}", order);
            let result = orderbook.execute(order);
            info!("gRPC execution result: {:#?}", result);
        }
    })
}