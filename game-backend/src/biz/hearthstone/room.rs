use std::sync::Arc;

use crate::{
    biz::hearthstone::game::GameLogic,
    common::{
        input::InputManager,
        room::{BizRoom, SafeRoom},
    },
};
use async_trait::async_trait;
use idl_gen::bss_websocket_client::BoxProtobufPayload;

pub struct Room {
    input: Arc<InputManager>,
}

impl Room {
    pub fn new() -> Room {
        Room {
            input: Arc::new(InputManager::default()),
        }
    }
}

#[async_trait]
impl BizRoom for Room {
    async fn do_game_logic(&self, safe_room: SafeRoom) {
        log::info!("game start");
        // let room = GameLogic::create(safe_room, self.input.clone()).await;
        // let room = match room {
        //     Ok(room) => room,
        //     Err(e) => {
        //         log::error!("create room err: {e}");
        //         return;
        //     }
        // };
        // room.run().await;
    }

    async fn check_start(&self, player_count: usize) -> bool {
        player_count == 4
    }

    async fn max_player_count(&self) -> usize {
        4
    }

    async fn player_input(&self, user_id: i64, data: BoxProtobufPayload) {
        self.input.player_input(user_id, data).await;
    }
}
