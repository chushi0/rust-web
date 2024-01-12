use crate::common::room::{BizRoom, SafeRoom};
use async_trait::async_trait;
use idl_gen::bss_websocket_client::BoxProtobufPayload;

#[derive(Debug)]
pub struct Room {}

impl Default for Room {
    fn default() -> Self {
        Self::new()
    }
}

impl Room {
    pub fn new() -> Room {
        Room {}
    }
}

#[async_trait]
impl BizRoom for Room {
    async fn do_game_logic(&self, safe_room: SafeRoom) {
        let mut game = super::game::Game::create(safe_room.clone()).await;
        game.run().await;
    }

    async fn check_start(&self, player_count: usize) -> bool {
        player_count == 2
    }

    async fn max_player_count(&self) -> usize {
        2
    }

    async fn player_input(&self, _user_id: i64, _data: BoxProtobufPayload) {}
}
