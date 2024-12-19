use crate::core::models::{MarketOrder, Side};
use crate::engine::services::manager::Manager;
use crate::protobuf::models::{CreateMarketOrderRequest, RfqResult};
use crate::protobuf::services::stat_stream_server::{StatStream, StatStreamServer};
use std::sync::Arc;
use tonic::codegen::tokio_stream::wrappers::ReceiverStream;
use tonic::{Request, Response, Status};

pub struct StatStreamer {
    max_quote_count: usize,
    max_buffer_size: usize,
    manager: Arc<Manager>,
}
impl StatStreamer {
    pub fn create(
        max_quote_count: usize,
        max_buffer_size: usize,
        manager: Arc<Manager>,
    ) -> StatStreamServer<StatStreamer> {
        StatStreamServer::new(StatStreamer {
            max_quote_count,
            max_buffer_size,
            manager,
        })
    }

    fn build_rfq_payload(request: Request<CreateMarketOrderRequest>) -> MarketOrder {
        let request = request.into_inner();
        MarketOrder::new(0, request.quantity, Side::from(request.side))
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
        let manager = Arc::clone(&self.manager);
        tokio::spawn(async move {
            loop {
                if tx.is_closed() || counter >= max_quote_count {
                    break;
                }
                counter += 1;
                let result = unsafe {
                    (*manager.get_secondary())
                        .request_for_quote(payload)
                        .to_protobuf()
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
