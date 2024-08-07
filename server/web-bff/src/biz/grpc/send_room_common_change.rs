use anyhow::Result;
use idl_gen::{
    bff_websocket::{SendRoomCommonChangeRequest, SendRoomCommonChangeResponse},
    bff_websocket_client::RoomPlayerChangeEvent,
};

use crate::{service, util::protobuf};

pub async fn handle(req: &SendRoomCommonChangeRequest) -> Result<SendRoomCommonChangeResponse> {
    let msg = RoomPlayerChangeEvent {
        public: req.public,
        players: service::game::pack_game_room_player(&req.room_players).await?,
        ..Default::default()
    };
    let msg = protobuf::pack_message(msg)?;

    let mut success_players = vec![];
    let mut fail_players = vec![];
    for user_id in &req.user_ids {
        let key = crate::ws::game::RoomKey {
            user_id: *user_id,
            game_type: req.game_type,
            room_id: req.room_id,
        };

        match crate::ws::game::get_room_wscon(&key).await {
            Some(wscon) => {
                if let Err(e) = wscon.send_binary(msg.clone()) {
                    log::warn!("send room common change error: {}", e);
                    fail_players.push(*user_id)
                } else {
                    success_players.push(*user_id);
                }
            }
            None => {
                log::warn!("player {user_id} not establish now");
                fail_players.push(*user_id);
            }
        }
    }

    Ok(SendRoomCommonChangeResponse {
        success_user_ids: success_players,
        failed_user_ids: fail_players,
    })
}
