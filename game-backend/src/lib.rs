#![feature(type_alias_impl_trait)]

use idl_gen::game_backend::*;
use volo_grpc::{Request, Response, Status};

pub struct S;

pub mod biz;
pub mod common;
pub mod handler;

#[volo::async_trait]
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
}
