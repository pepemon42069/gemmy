use std::sync::Arc;
use crate::core::models::{LimitOrder, MarketOrder, Operation, Side};
use crate::protobuf::models::{CancelLimitOrderRequest, CreateLimitOrderRequest, CreateMarketOrderRequest, ModifyLimitOrderRequest, StringResponse};
use crate::protobuf::{
    services::order_dispatcher_server::{OrderDispatcher, OrderDispatcherServer},
};
use tokio::sync::mpsc;
use tokio::sync::mpsc::Sender;
use tonic::{codegen::InterceptedService, Request, Response, Status};
use tracing::{error, info};
use crate::engine::configuration::kafka_configuration::KafkaConfiguration;
use crate::engine::configuration::server_configuration::ServerConfiguration;
use crate::engine::state::server_state::ServerState;
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
        server_configuration: Arc<ServerConfiguration>,
        kafka_configuration: Arc<KafkaConfiguration>,
        state: Arc<ServerState>,
        task_manager: &mut TaskManager
    ) -> DispatchService {
        let (tx, rx) = mpsc::channel(10000);
        task_manager.register("order_exec_task", {
            async move {
                Executor::new(
                    server_configuration,
                    kafka_configuration,
                    state,
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
