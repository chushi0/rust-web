use idl_gen::{bss_hearthstone::JoinRoomExtraData, game_backend::GameType};
use protobuf::Message;

use crate::common::room::{create_room, force_start_game, join_room, BizRoom};

#[test]
fn test_game() {
    log4rs::init_file("log4rs.yaml", Default::default()).unwrap();

    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap()
        .block_on(async { test_game_main().await })
}

async fn test_game_main() {
    let safe_room = create_room(GameType::Hearthstone).await;
    for i in 1..=4 {
        join_room(safe_room.clone(), i, Some(gen_test_deck_data()))
            .await
            .expect("should join room successfully");
    }
    unsafe { force_start_game(safe_room).await }
}

fn gen_test_deck_data() -> Vec<u8> {
    let mut data = JoinRoomExtraData::default();
    data.card_code.push("value".to_string());
    data.write_to_bytes().expect("should gen data successfully")
}
