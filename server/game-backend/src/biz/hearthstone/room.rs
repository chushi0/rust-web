use crate::common::{
    input::InputManager,
    room::{BizRoom, SafeRoom},
};
use anyhow::Result;
use async_trait::async_trait;
use heartstone::{
    game::{Config, Game, PlayerConfig},
    model::{CardModel, CardPool},
};
use idl_gen::bss_websocket_client::BoxProtobufPayload;
use std::sync::Arc;

#[derive(Debug)]
pub struct Room {
    input: Arc<InputManager>,
}

impl Default for Room {
    fn default() -> Self {
        Self::new()
    }
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
        let game = match self.create_game(safe_room).await {
            Ok(game) => game,
            Err(err) => {
                log::error!("failed to create heartstone game instance: {err}");
                return;
            }
        };
        log::info!("game initialize finish, start main logic");
        let result = game.run().await;
        log::info!("game result: {result:?}");
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

impl Room {
    async fn create_game(&self, safe_room: SafeRoom) -> Result<Game> {
        let config = Config {
            card_pool: Self::get_all_card().await?,
            game_notifier: Arc::new(super::notifier::Notifier::new(safe_room.clone())),
            ..Default::default()
        };

        let room = safe_room.lock().await;
        let players = room.players();
        let players = players
            .iter()
            .map(|player| PlayerConfig {
                custom_id: player.get_user_id(),
                behavior: Arc::new(super::behavior::SocketPlayerBehavior::new(
                    player.get_user_id(),
                    safe_room.clone(),
                    self.input.clone(),
                )),
                ..Default::default()
            })
            .collect();

        let game = Game::new(config, players).await;
        Ok(game)
    }

    async fn get_all_card() -> Result<CardPool> {
        use web_db::hearthstone::get_all_cards;
        use web_db::{begin_tx, create_connection, RDS};

        let mut conn = create_connection(RDS::Hearthstone).await?;
        let mut tx = begin_tx(&mut conn).await?;

        let cards = get_all_cards(&mut tx).await?;
        let card_pool = cards
            .into_iter()
            .map(|card| {
                let card_info = serde_json::from_str(&card.card_info)?;
                Ok(CardModel { card, card_info })
            })
            .collect::<Result<Vec<CardModel>>>()?
            .into_iter()
            .map(|card_model| (card_model.card.rowid, Arc::new(card_model)))
            .collect();

        Ok(card_pool)
    }
}
