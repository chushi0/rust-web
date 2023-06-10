use crate::common::room::{BizRoom, SafeRoom};
use async_trait::async_trait;

#[derive(Debug)]
pub struct Room {}

impl Room {
    pub fn new() -> Room {
        Room {}
    }
}

#[async_trait]
impl BizRoom for Room {
    async fn do_game_logic(&self, safe_room: SafeRoom) {
        log::info!("game start");
    }

    async fn check_start(&self, player_count: usize) -> bool {
        player_count == 2
    }

    async fn max_player_count(&self) -> usize {
        2
    }
}
