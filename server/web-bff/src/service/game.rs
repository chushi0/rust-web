use anyhow::Result;
use web_db::user::{query_user, QueryUserParam};
use web_db::{begin_tx, create_connection, RDS};

pub async fn pack_game_room_player(
    players: &Vec<idl_gen::bff_websocket::RoomPlayer>,
) -> Result<Vec<idl_gen::bff_websocket_client::RoomPlayer>> {
    let mut result = Vec::new();

    let mut conn = create_connection(RDS::User).await?;
    let mut tx = begin_tx(&mut conn).await?;

    for player in players {
        let user = query_user(
            &mut tx,
            QueryUserParam::ByUid {
                uid: player.user_id,
            },
        )
        .await?;

        let event_player = idl_gen::bff_websocket_client::RoomPlayer {
            account: user.account,
            display_name: user.username,
            index: player.index,
            ready: player.ready,
            ..Default::default()
        };
        result.push(event_player);
    }

    Ok(result)
}
