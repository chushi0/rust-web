use idl_gen::game_backend::*;
use volo_grpc::{Code, Request, Response, Status};

use crate::common::room;

pub async fn handle(req: Request<JoinRoomRequest>) -> Result<Response<JoinRoomResponse>, Status> {
    let req = req.get_ref();

    check_request(req)?;

    let result = match req.strategy {
        JoinRoomStrategy::Create => create_room(req).await,
        JoinRoomStrategy::Join => join_room(req).await,
        JoinRoomStrategy::Mate => mate_room(req).await,
    }?;

    Ok(Response::new(result))
}

fn check_request(req: &JoinRoomRequest) -> Result<(), Status> {
    if req.user_id <= 0 {
        return Err(Status::new(Code::Unauthenticated, "user_id < 0"));
    }
    Ok(())
}

async fn create_room(req: &JoinRoomRequest) -> Result<JoinRoomResponse, Status> {
    let room = room::create_room(req.game_type).await;

    room::join_room(room.clone(), req.user_id).await?;

    let mut room = room.lock().await;
    if req.public.unwrap_or(false) {
        room.set_public().await;
    }

    Ok(JoinRoomResponse {
        room_id: room.get_room_id(),
    })
}

async fn join_room(req: &JoinRoomRequest) -> Result<JoinRoomResponse, Status> {
    let room_id = req
        .room_id
        .ok_or_else(|| Status::new(Code::InvalidArgument, "missing room_id"))?;

    if room_id < room::MIN_ROOM_ID || room_id > room::MAX_ROOM_ID {
        return Err(Status::new(Code::OutOfRange, "room_id out of range"));
    }

    let room = room::get_room(req.game_type, room_id)
        .await
        .ok_or_else(|| Status::new(Code::NotFound, "room not exist"))?;

    room::join_room(room.clone(), req.user_id).await?;

    let room = room.lock().await;

    Ok(JoinRoomResponse {
        room_id: room.get_room_id(),
    })
}

async fn mate_room(req: &JoinRoomRequest) -> Result<JoinRoomResponse, Status> {
    let room = room::mate_room(req.game_type, req.user_id).await?;

    let room = room.lock().await;

    Ok(JoinRoomResponse {
        room_id: room.get_room_id(),
    })
}
