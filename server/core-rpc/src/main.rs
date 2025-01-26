use idl_gen::core_rpc::*;
use std::net::SocketAddr;
use volo_grpc::{server::ServiceBuilder, Request, Response, Status};

struct S;

pub mod handler;

#[tokio::main]
async fn main() {
    log4rs::init_file("conf/log4rs.yaml", Default::default()).unwrap();

    let addr: SocketAddr = "0.0.0.0:13000".parse().unwrap();
    let addr = volo::net::Address::from(addr);

    volo_grpc::server::Server::new()
        .add_service(ServiceBuilder::new(idl_gen::core_rpc::CoreRpcServiceServer::new(S)).build())
        .run(addr)
        .await
        .unwrap();
}

impl idl_gen::core_rpc::CoreRpcService for S {
    async fn create_github_activity_event(
        &self,
        req: Request<CreateGithubActivityEventRequest>,
    ) -> Result<Response<CreateGithubActivityEventResponse>, Status> {
        handler::create_github_activity_event::handle(req)
            .await
            .map_err(|e| {
                log::error!("create_github_activity_event fail: {:?}", e);
                Status::internal("")
            })
    }

    async fn list_github_activity_event(
        &self,
        req: Request<ListGithubActivityEventRequest>,
    ) -> Result<Response<ListGithubActivityEventResponse>, Status> {
        handler::list_github_activity_event::handle(req)
            .await
            .map_err(|e| {
                log::error!("list_github_activity_event fail: {:?}", e);
                Status::internal("")
            })
    }

    async fn list_display_event(
        &self,
        req: Request<ListDisplayEventRequest>,
    ) -> Result<Response<ListDisplayEventResponse>, Status> {
        handler::list_display_event::handle(req).await.map_err(|e| {
            log::error!("list_display_event fail: {:?}", e);
            Status::internal("")
        })
    }
}
