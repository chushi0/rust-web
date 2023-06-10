use idl_gen::bss_websocket::*;
use volo_grpc::{Request, Response, Status};

pub struct S;

#[volo::async_trait]
impl idl_gen::bss_websocket::BssWebsocketService for S {
    async fn send_room_common_change(
        &self,
        req: Request<SendRoomCommonChangeRequest>,
    ) -> Result<Response<SendRoomCommonChangeResponse>, Status> {
        todo!()
    }

    async fn send_game_event(
        &self,
        req: Request<SendGameEventRequest>,
    ) -> Result<Response<SendGameEventResponse>, Status> {
        todo!()
    }
}
