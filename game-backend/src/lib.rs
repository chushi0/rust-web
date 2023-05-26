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
}
