use crate::biz::grpc;
use idl_gen::bss_websocket::*;
use volo_grpc::{Request, Response, Status};

pub struct S;

impl idl_gen::bss_websocket::BssWebsocketService for S {
    async fn send_room_common_change(
        &self,
        req: Request<SendRoomCommonChangeRequest>,
    ) -> Result<Response<SendRoomCommonChangeResponse>, Status> {
        let req = req.get_ref();

        match grpc::send_room_common_change::handle(req).await {
            Ok(resp) => Ok(Response::new(resp)),
            Err(e) => Err(Status::internal(e.to_string())),
        }
    }

    async fn send_game_event(
        &self,
        _req: Request<SendGameEventRequest>,
    ) -> Result<Response<SendGameEventResponse>, Status> {
        todo!()
    }
}
