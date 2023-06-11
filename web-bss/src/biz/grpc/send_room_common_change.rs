use anyhow::Result;
use idl_gen::{
    bss_websocket::{SendRoomCommonChangeRequest, SendRoomCommonChangeResponse},
    bss_websocket_client::{RoomPlayer, RoomPlayerChangeEvent},
};
use web_db::user::{query_user, QueryUserParam};
use web_db::{begin_tx, create_connection, RDS};

use crate::util::protobuf;

pub async fn handle(req: &SendRoomCommonChangeRequest) -> Result<SendRoomCommonChangeResponse> {
    let mut conn = create_connection(RDS::User).await?;
    let mut tx = begin_tx(&mut conn).await?;

    let mut msg = RoomPlayerChangeEvent::default();
    msg.public = req.public;
    for player in &req.room_players {
        let user = query_user(
            &mut tx,
            QueryUserParam::ByUid {
                uid: player.user_id,
            },
        )
        .await?;

        let mut event_player = RoomPlayer::default();
        event_player.account = user.account;
        event_player.display_name = user.username;
        event_player.index = player.index;
        event_player.ready = player.ready;
        msg.players.push(event_player);
    }
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
                if let Err(_) = wscon.send_binary(msg.clone()) {
                    fail_players.push(*user_id)
                } else {
                    success_players.push(*user_id);
                }
            }
            None => fail_players.push(*user_id),
        }
    }

    Ok(SendRoomCommonChangeResponse {
        success_user_ids: success_players,
        failed_user_ids: fail_players,
    })
}
