use crate::common::{input::InputManager, room::SafeRoom};
use heartstone::{
    game::Game,
    player::{Player, PlayerBehavior, PlayerTurnAction},
};
use std::sync::Arc;
use tokio::sync::RwLock;

#[derive(Debug)]
pub struct SocketPlayerBehavior {
    id: i64,
    uuid: RwLock<Option<u64>>,
    safe_room: SafeRoom,
    input: Arc<InputManager>,
}

impl SocketPlayerBehavior {
    pub fn new(id: i64, safe_room: SafeRoom, input: Arc<InputManager>) -> Self {
        Self {
            id,
            uuid: RwLock::new(None),
            safe_room,
            input,
        }
    }
}

#[async_trait::async_trait]
impl PlayerBehavior for SocketPlayerBehavior {
    async fn assign_uuid(&self, uuid: u64) {
        *self.uuid.write().await = Some(uuid);
    }

    async fn next_action(&self, game: &Game, player: &Player) -> PlayerTurnAction {
        todo!()
    }
}
