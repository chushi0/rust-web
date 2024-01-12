use crate::biz::grpc;
use idl_gen::bss_websocket::*;
use volo_grpc::{Request, Response, Status};

pub struct S;

macro_rules! rpc {
    ($name:tt($req:tt) -> $resp:tt) => {
        async fn $name(&self, req: Request<$req>) -> Result<Response<$resp>, Status> {
            let req = req.get_ref();

            match grpc::$name::handle(req).await {
                Ok(resp) => Ok(Response::new(resp)),
                Err(e) => Err(Status::internal(e.to_string())),
            }
        }
    };
}

impl idl_gen::bss_websocket::BssWebsocketService for S {
    rpc!(send_room_common_change(SendRoomCommonChangeRequest) -> SendRoomCommonChangeResponse);
    rpc!(send_room_chat(SendRoomChatRequest) -> SendRoomChatResponse);
    rpc!(send_game_event(SendGameEventRequest) -> SendGameEventResponse);
}
