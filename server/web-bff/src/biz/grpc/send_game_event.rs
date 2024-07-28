use crate::util::protobuf;
use anyhow::Result;
use idl_gen::{
    bff_websocket::{SendGameEventRequest, SendGameEventResponse},
    bff_websocket_client::{GameEvent, GameEventList},
};

pub async fn handle(req: &SendGameEventRequest) -> Result<SendGameEventResponse> {
    let msg = GameEventList {
        list: req
            .event_list
            .iter()
            .map(|event| GameEvent {
                event_type: event.event_type.to_string(),
                payload: event.payload.to_vec(),
                ..Default::default()
            })
            .collect(),
        ..Default::default()
    };
    let msg = protobuf::pack_message(msg)?;

    let mut success_players = vec![];
    let mut fail_players = vec![];
    for user_id in &req.user_id {
        let key = crate::ws::game::RoomKey {
            user_id: *user_id,
            game_type: req.game_type,
            room_id: req.room_id,
        };

        match crate::ws::game::get_room_wscon(&key).await {
            Some(wscon) => {
                if let Err(e) = wscon.send_binary(msg.clone()) {
                    log::warn!("send game event error: {}", e);
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
    Ok(SendGameEventResponse {
        success_user_ids: success_players,
        failed_user_ids: fail_players,
    })
}
