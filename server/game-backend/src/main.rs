#![feature(type_alias_impl_trait)]

use std::net::SocketAddr;

use idl_gen::game_backend::*;
use volo_grpc::{server::ServiceBuilder, Request, Response, Status};

struct S;

pub mod biz;
pub mod common;
pub mod handler;
pub mod rpc;

#[tokio::main]
async fn main() {
    log4rs::init_file("log4rs.game-backend.yaml", Default::default()).unwrap();

    let addr: SocketAddr = "127.0.0.1:13201".parse().unwrap();
    let addr = volo::net::Address::from(addr);

    volo_grpc::server::Server::new()
        .add_service(
            ServiceBuilder::new(idl_gen::game_backend::GameBackendServiceServer::new(S)).build(),
        )
        .run(addr)
        .await
        .unwrap();
}

impl idl_gen::game_backend::GameBackendService for S {
    async fn join_room(
        &self,
        req: Request<JoinRoomRequest>,
    ) -> Result<Response<JoinRoomResponse>, Status> {
        handler::join_room::handle(req).await
    }

    async fn set_player_ready(
        &self,
        req: Request<SetPlayerReadyRequest>,
    ) -> Result<Response<SetPlayerReadyResponse>, Status> {
        handler::room_interactive::handle_set_player_ready(req).await
    }

    async fn set_room_public(
        &self,
        req: Request<SetRoomPublicRequest>,
    ) -> Result<Response<SetRoomPublicResponse>, Status> {
        handler::room_interactive::handle_set_room_public(req).await
    }

    async fn leave_room(
        &self,
        req: Request<LeaveRoomRequest>,
    ) -> Result<Response<LeaveRoomResponse>, Status> {
        handler::room_interactive::handle_leave_room(req).await
    }

    async fn send_game_chat(
        &self,
        req: Request<SendGameChatRequest>,
    ) -> Result<Response<SendGameChatResponse>, Status> {
        handler::room_interactive::handle_send_room_chat(req).await
    }

    async fn submit_player_action(
        &self,
        req: Request<SubmitPlayerActionRequest>,
    ) -> Result<Response<SubmitPlayerActionResponse>, Status> {
        handler::room_interactive::handle_submit_player_action(req).await
    }
}
