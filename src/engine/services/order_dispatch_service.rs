use std::sync::Arc;
use std::time::Duration;
use rdkafka::producer::FutureProducer;
use schema_registry_converter::async_impl::schema_registry::SrSettings;
use crate::core::models::{LimitOrder, MarketOrder, Operation, Side};
use crate::protobuf::models::{CancelLimitOrderRequest, CreateLimitOrderRequest, CreateMarketOrderRequest, ModifyLimitOrderRequest, StringResponse};
use crate::protobuf::{
    services::order_dispatcher_server::{OrderDispatcher, OrderDispatcherServer},
};
use tokio::sync::{mpsc, Notify};
use tokio::sync::mpsc::Sender;
use tonic::{codegen::InterceptedService, Request, Response, Status};
use tracing::{error, info};
use crate::engine::services::orderbook_manager_service::OrderbookManager;
use crate::engine::tasks::order_exec_task::Executor;
use crate::engine::tasks::task_manager::TaskManager;

pub type DispatchService = InterceptedService<
    OrderDispatcherServer<OrderDispatchService>,
    fn(Request<()>) -> Result<Request<()>, Status>,
>;

#[derive(Debug)]
pub struct OrderDispatchService {
    tx: Sender<Operation>,
}

impl OrderDispatchService {
    pub fn create(
        batch_size: usize,
        batch_timeout: Duration,
        shutdown_notification: Arc<Notify>,
        orderbook_manager: Arc<OrderbookManager>,
        kafka_topic: String,
        kafka_producer: Arc<FutureProducer>,
        schema_registry_url : SrSettings,
        task_manager: &mut TaskManager
    ) -> DispatchService {
        let (tx, rx) = mpsc::channel(10000);
        task_manager.register("order_exec_task", {
            async move {
                Executor::new(
                    batch_size, 
                    batch_timeout, 
                    shutdown_notification, 
                    orderbook_manager,
                    kafka_topic,
                    kafka_producer,
                    schema_registry_url,
                    rx).run().await;
            }
        });
        OrderDispatcherServer::with_interceptor(OrderDispatchService { tx }, Self::interceptor)
    }

    fn build_limit_payload(request: Request<CreateLimitOrderRequest>) -> Operation {
        let request = request.into_inner();
        Operation::Limit(LimitOrder::new_uuid_v4(
            request.price,
            request.quantity,
            Side::from(request.side),
        ))
    }

    fn build_market_payload(request: Request<CreateMarketOrderRequest>) -> Operation {
        let request = request.into_inner();
        Operation::Market(MarketOrder::new_uuid_v4(
            request.quantity,
            Side::from(request.side),
        ))
    }

    fn build_modify_payload(request: Request<ModifyLimitOrderRequest>) -> Operation {
        let request = request.into_inner();
        Operation::Modify(LimitOrder::new(
            u128::from_be_bytes(request.order_id.try_into().unwrap()),
            request.price,
            request.quantity,
            Side::from(request.side),
        ))
    }

    fn build_cancel_payload(request: Request<CancelLimitOrderRequest>) -> Operation {
        let request = request.into_inner();
        Operation::Cancel(u128::from_be_bytes(request.order_id.try_into().unwrap()))
    }

    fn interceptor(request: Request<()>) -> Result<Request<()>, Status> {
        if let Some(token) = request.metadata().get("bearer") {
            info!("gRPC request received: {:?}", token);
        }
        info!("passing through interceptor");
        Ok(request)
    }

    async fn execute(&self, payload: Operation) -> Result<Response<StringResponse>, Status> {
        match self.tx.send(payload).await {
            Ok(_) => (),
            Err(e) => {
                error!("failed to dispatch message: {}", e);
                return Err(Status::internal("internal server error"));
            }
        }
        Ok(Response::new(StringResponse {
            message: "ok".to_string(),
        }))
    }
}

#[tonic::async_trait]
impl OrderDispatcher for OrderDispatchService {
    async fn limit(
        &self,
        request: Request<CreateLimitOrderRequest>,
    ) -> Result<Response<StringResponse>, Status> {
        self.execute(Self::build_limit_payload(request)).await
    }

    async fn market(
        &self,
        request: Request<CreateMarketOrderRequest>,
    ) -> Result<Response<StringResponse>, Status> {
        self.execute(Self::build_market_payload(request)).await
    }

    async fn modify(
        &self,
        request: Request<ModifyLimitOrderRequest>,
    ) -> Result<Response<StringResponse>, Status> {
        self.execute(Self::build_modify_payload(request)).await
    }

    async fn cancel(
        &self,
        request: Request<CancelLimitOrderRequest>,
    ) -> Result<Response<StringResponse>, Status> {
        self.execute(Self::build_cancel_payload(request)).await
    }
}
