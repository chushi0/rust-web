use idl_gen::{bff_websocket_client::BoxProtobufPayload, game_backend::*};
use volo_grpc::{Code, Request, Response, Status};

use crate::common::room;

pub async fn handle_set_player_ready(
    req: Request<SetPlayerReadyRequest>,
) -> Result<Response<SetPlayerReadyResponse>, Status> {
    let req = req.get_ref();
    check_request(req.user_id, req.room_id)?;

    let room = room::get_room(req.game_type, req.room_id)
        .await
        .ok_or_else(|| Status::new(Code::NotFound, "missing room"))?;

    room::set_player_ready(room, req.user_id, req.ready).await?;

    Ok(Response::new(SetPlayerReadyResponse::default()))
}

pub async fn handle_set_room_public(
    req: Request<SetRoomPublicRequest>,
) -> Result<Response<SetRoomPublicResponse>, Status> {
    let req = req.get_ref();
    check_request(req.user_id, req.room_id)?;

    let room = room::get_room(req.game_type, req.room_id)
        .await
        .ok_or_else(|| Status::new(Code::NotFound, "missing room"))?;

    let mut room = room.lock().await;
    if room.master_user_id() != req.user_id {
        return Err(Status::new(Code::PermissionDenied, "not master user"));
    }
    room.set_public().await;

    Ok(Response::new(SetRoomPublicResponse::default()))
}

pub async fn handle_leave_room(
    req: Request<LeaveRoomRequest>,
) -> Result<Response<LeaveRoomResponse>, Status> {
    let req = req.get_ref();
    check_request(req.user_id, req.room_id)?;

    let room = room::get_room(req.game_type, req.room_id)
        .await
        .ok_or_else(|| Status::new(Code::NotFound, "missing room"))?;

    room::leave_room(room, req.user_id).await?;

    Ok(Response::new(LeaveRoomResponse::default()))
}

pub async fn handle_send_room_chat(
    req: Request<SendGameChatRequest>,
) -> Result<Response<SendGameChatResponse>, Status> {
    let req = req.get_ref();
    check_request(req.user_id, req.room_id)?;

    let room = room::get_room(req.game_type, req.room_id)
        .await
        .ok_or_else(|| Status::new(Code::NotFound, "missing room"))?;

    room::room_chat(
        room,
        req.user_id,
        &req.receiver_user_id,
        req.content.clone(),
    )
    .await?;

    Ok(Response::new(SendGameChatResponse::default()))
}

pub async fn handle_submit_player_action(
    req: Request<SubmitPlayerActionRequest>,
) -> Result<Response<SubmitPlayerActionResponse>, Status> {
    let req = req.into_inner();
    check_request(req.user_id, req.room_id)?;

    let room = room::get_room(req.game_type, req.room_id)
        .await
        .ok_or_else(|| Status::new(Code::NotFound, "missing room"))?;

    let room = room.lock().await;
    if !room.is_in_game() {
        return Err(Status::new(Code::Unavailable, "game is not start"));
    }
    if !room
        .players()
        .iter()
        .any(|player| player.get_user_id() == req.user_id)
    {
        return Err(Status::new(Code::NotFound, "user is not in this room"));
    }
    let biz_room = room.biz_room();
    // release room lock
    drop(room);

    biz_room
        .player_input(
            req.user_id,
            BoxProtobufPayload {
                name: req.action_name.into(),
                payload: req.payload.into(),
                ..Default::default()
            },
        )
        .await;

    Ok(Response::new(SubmitPlayerActionResponse::default()))
}

fn check_request(user_id: i64, room_id: i32) -> Result<(), Status> {
    if user_id <= 0 {
        return Err(Status::new(Code::Unauthenticated, "user_id < 0"));
    }

    if !(room::MIN_ROOM_ID..=room::MAX_ROOM_ID).contains(&room_id) {
        return Err(Status::new(Code::OutOfRange, "room_id out of range"));
    }
    Ok(())
}
