use tokio::sync::mpsc;
use tokio::time::Instant;
use tonic::{Request, Response, Status, codegen::InterceptedService};
use tracing::{error, info};
use crate::core::models::{LimitOrder, MarketOrder, Operation, Side};
use crate::protobuf::{
    models::GenericMessage,
    services::order_dispatcher_server::{
        OrderDispatcher, 
        OrderDispatcherServer
    }
};
use crate::protobuf::models::{CancelLimitOrderRequest, CreateLimitOrderRequest, CreateMarketOrderRequest, ModifyLimitOrderRequest};

pub type DispatchService = InterceptedService<
    OrderDispatcherServer<OrderDispatchService>,
    fn(Request<()>) -> Result<Request<()>, Status>
>;

#[derive(Debug)]
pub struct OrderDispatchService {
    tx: mpsc::Sender<Operation>
}

impl OrderDispatchService {
    
    pub fn create_no_interceptor(
        tx: mpsc::Sender<Operation>) -> OrderDispatcherServer<OrderDispatchService> {
        OrderDispatcherServer::new(OrderDispatchService { tx })
    }
    
    pub fn create(tx: mpsc::Sender<Operation>) -> DispatchService {
        OrderDispatcherServer::with_interceptor(
            OrderDispatchService { tx }, 
            Self::interceptor
        )
    }

    fn build_limit_payload(request: Request<CreateLimitOrderRequest>) -> Operation {
        let request = request.into_inner();
        Operation::Limit(
            LimitOrder::new_uuid_v4(
                request.price,
                request.quantity,
                Side::from(request.side),
            )
        )
    }
    
    fn build_market_payload(request: Request<CreateMarketOrderRequest>) -> Operation {
        let request = request.into_inner();
        Operation::Market(
            MarketOrder::new_uuid_v4(
                request.quantity,
                Side::from(request.side),
            )
        )
    }
    
    fn build_modify_payload(request: Request<ModifyLimitOrderRequest>) -> Operation {
        let request = request.into_inner();
        Operation::Modify(
            LimitOrder::new(
                u128::from_be_bytes(request.order_id.try_into().unwrap()),
                request.price,
                request.quantity,
                Side::from(request.side)
            )
        )
    }
    
    fn build_cancel_payload(request: Request<CancelLimitOrderRequest>) -> Operation {
        let request = request.into_inner();
        Operation::Cancel(
            u128::from_be_bytes(request.order_id.try_into().unwrap())
        )
    }

    fn interceptor(request: Request<()>) -> Result<Request<()>, Status> {
        let start = Instant::now();
        if let Some(token) = request.metadata().get("bearer") {
            info!("gRPC request received: {:?}", token);
        }
        let elapsed = start.elapsed().as_micros();
        info!("gRPC interceptor time: {}", elapsed);
        Ok(request)
    }

    async fn execute(&self, payload: Operation) -> Result<Response<GenericMessage>, Status> {
        // info!("dispatching message: {:?}", payload);
        match self.tx.send(payload).await {
            Ok(_) => (),
            Err(e) => {
                error!("failed to dispatch message: {}", e);
                return Err(Status::internal("internal server error"));
            }
        }
        Ok(Response::new(GenericMessage { message: "ok".to_string() }))
    }
}

#[tonic::async_trait]
impl OrderDispatcher for OrderDispatchService {
    async fn limit(
        &self, 
        request: Request<CreateLimitOrderRequest>
    ) -> Result<Response<GenericMessage>, Status> {
        self.execute(Self::build_limit_payload(request)).await
    }

    async fn market(
        &self, 
        request: Request<CreateMarketOrderRequest>
    ) -> Result<Response<GenericMessage>, Status> {
        self.execute(Self::build_market_payload(request)).await
    }

    async fn modify(&self, request: Request<ModifyLimitOrderRequest>) -> Result<Response<GenericMessage>, Status> {
        self.execute(Self::build_modify_payload(request)).await
    }

    async fn cancel(&self, request: Request<CancelLimitOrderRequest>) -> Result<Response<GenericMessage>, Status> {
        self.execute(Self::build_cancel_payload(request)).await
    }
}