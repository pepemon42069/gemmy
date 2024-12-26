use crate::core::models::{Granularity, MarketOrder, Side};
use crate::engine::services::orderbook_manager_service::OrderbookManager;
use crate::protobuf::models::{CreateMarketOrderRequest, OrderbookData, OrderbookDataRequest, RfqResult};
use crate::protobuf::services::stat_stream_server::{StatStream, StatStreamServer};
use std::sync::Arc;
use tonic::codegen::tokio_stream::wrappers::ReceiverStream;
use tonic::{Request, Response, Status};
use crate::engine::utils::protobuf::{rfq_to_proto, orderbook_data_to_proto};

pub struct StatStreamer {
    max_quote_count: usize,
    max_buffer_size: usize,
    orderbook_manager: Arc<OrderbookManager>,
}
impl StatStreamer {
    pub fn create(
        max_quote_count: usize,
        max_buffer_size: usize,
        orderbook_manager: Arc<OrderbookManager>,
    ) -> StatStreamServer<StatStreamer> {
        StatStreamServer::new(StatStreamer {
            max_quote_count,
            max_buffer_size,
            orderbook_manager,
        })
    }

    fn build_rfq_payload(request: Request<CreateMarketOrderRequest>) -> MarketOrder {
        let request = request.into_inner();
        MarketOrder::new(0, request.quantity, Side::from(request.side))
    }

    fn build_orderbook_data_payload(request: Request<OrderbookDataRequest>) -> Granularity {
        let request = request.into_inner();
        match request.granularity {
            0 => Granularity::P00,
            1 => Granularity::P0,
            2 => Granularity::P,
            3 => Granularity::P10,
            4 => Granularity::P100,
            _ => Granularity::P00,
        }
    }
}

#[tonic::async_trait]
impl StatStream for StatStreamer {
    type rfqStream = ReceiverStream<Result<RfqResult, Status>>;
    async fn rfq(
        &self,
        request: Request<CreateMarketOrderRequest>,
    ) -> Result<Response<Self::rfqStream>, Status> {
        let max_quote_count = self.max_quote_count;
        let payload = Self::build_rfq_payload(request);
        let (tx, rx) = tokio::sync::mpsc::channel(self.max_buffer_size);
        let mut counter = 0;
        let orderbook_manager = Arc::clone(&self.orderbook_manager);
        tokio::spawn(async move {
            loop {
                if tx.is_closed() || counter >= max_quote_count {
                    break;
                }
                counter += 1;
                let result = unsafe {
                    rfq_to_proto((*orderbook_manager.get_secondary()).request_for_quote(payload)) 
                };
                if tx.send(Ok(result)).await.is_err() {
                    break;
                }
                tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
            }
        });
        Ok(Response::new(ReceiverStream::new(rx)))
    }

    type orderbookStream = ReceiverStream<Result<OrderbookData, Status>>;

    async fn orderbook(&self, request: Request<OrderbookDataRequest>) -> Result<Response<Self::orderbookStream>, Status> {
        let (tx, rx) = tokio::sync::mpsc::channel(self.max_buffer_size);
        let orderbook_manager = Arc::clone(&self.orderbook_manager);
        let payload = Self::build_orderbook_data_payload(request);
        tokio::spawn(async move {
            loop {
                if tx.is_closed() {
                    break;
                }
                let result = unsafe {
                    orderbook_data_to_proto(
                        (*orderbook_manager.get_secondary()).get_last_trade_price(),
                        (*orderbook_manager.get_secondary()).get_max_bid().unwrap_or(u64::MIN),
                        (*orderbook_manager.get_secondary()).get_min_ask().unwrap_or(u64::MAX),
                        (*orderbook_manager.get_secondary()).orderbook_data(payload)
                    )
                };
                if tx.send(Ok(result)).await.is_err() {
                    break;
                }
                tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
            }
        });
        Ok(Response::new(ReceiverStream::new(rx)))
    }
}
