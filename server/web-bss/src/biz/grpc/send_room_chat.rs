use crate::util::protobuf;
use anyhow::Result;
use idl_gen::{
    bss_websocket::{SendRoomChatRequest, SendRoomChatResponse},
    bss_websocket_client::PlayerChatEvent,
};

pub async fn handle(req: &SendRoomChatRequest) -> Result<SendRoomChatResponse> {
    let msg = PlayerChatEvent {
        player_index: req.sender_user_index,
        receiver_player_indexes: req.receiver_user_indexes.clone(),
        message: req.content.clone().into(),
        ..Default::default()
    };
    let msg = protobuf::pack_message(msg)?;

    let mut success_players = vec![];
    let mut fail_players = vec![];
    for user_id in &req.receiver_user_ids {
        let key = crate::ws::game::RoomKey {
            user_id: *user_id,
            game_type: req.game_type,
            room_id: req.room_id,
        };

        match crate::ws::game::get_room_wscon(&key).await {
            Some(wscon) => {
                if let Err(e) = wscon.send_binary(msg.clone()) {
                    log::warn!("send room chat message error: {}", e);
                    fail_players.push(*user_id)
                } else {
                    success_players.push(*user_id);
                }
            }
            None => fail_players.push(*user_id),
        }
    }

    Ok(SendRoomChatResponse::default())
}
