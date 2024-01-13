use crate::{
    common::{input::InputManager, room::SafeRoom},
    rpc,
};
use heartstone::{
    game::Game,
    player::{Player, PlayerBehavior, PlayerTurnAction},
};
use idl_gen::{
    bss_heartstone::{MyTurnStartEvent, PlayerEndTurnAction, PlayerTurnActionEnum},
    bss_websocket::{GameEvent, SendGameEventRequest},
    game_backend::GameType,
};
use protobuf::{Message, MessageField};
use std::{sync::Arc, time::Duration};
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

    async fn next_action(&self, _game: &Game, player: &Player) -> PlayerTurnAction {
        let room_id = self.safe_room.lock().await.get_room_id();

        let on_start_listen_input = || async {
            let payload = MyTurnStartEvent::default();
            let payload = match payload.write_to_bytes() {
                Ok(v) => v,
                Err(err) => {
                    log::error!("write to bytes error: {err}");
                    return;
                }
            };

            let req = SendGameEventRequest {
                user_id: vec![player.custom_id()],
                game_type: GameType::Hearthstone as i32,
                room_id,
                event_list: vec![GameEvent {
                    event_type: MyTurnStartEvent::NAME.into(),
                    payload: payload.into(),
                }],
                ..Default::default()
            };

            if let Err(err) = rpc::bss::client().send_game_event(req).await {
                log::error!("send game event error: {err}");
            }
        };
        let on_stop_listen_input = async {
            let payload = MyTurnStartEvent::default();
            let payload = match payload.write_to_bytes() {
                Ok(v) => v,
                Err(err) => {
                    log::error!("write to bytes error: {err}");
                    return;
                }
            };

            let req = SendGameEventRequest {
                user_id: vec![player.custom_id()],
                game_type: GameType::Hearthstone as i32,
                room_id,
                event_list: vec![GameEvent {
                    event_type: MyTurnStartEvent::NAME.into(),
                    payload: payload.into(),
                }],
                ..Default::default()
            };

            if let Err(err) = rpc::bss::client().send_game_event(req).await {
                log::error!("send game event error: {err}");
            }
        };

        let default_value = || idl_gen::bss_heartstone::PlayerTurnAction {
            action_type: PlayerTurnActionEnum::PlayerEndTurn.into(),
            player_end_turn: MessageField::some(PlayerEndTurnAction::default()),
            ..Default::default()
        };

        let input = self
            .input
            .wait_for_input(
                self.id,
                Duration::from_secs(90),
                default_value,
                Some(on_start_listen_input),
            )
            .await;

        on_stop_listen_input.await;

        match input.action_type.enum_value() {
            Ok(PlayerTurnActionEnum::PlayerUseCard) => input
                .player_use_card
                .map(|action| {
                    let target = action.target.map(parse_target).unwrap_or(None);
                    PlayerTurnAction::PlayCard {
                        hand_index: action.card_index as usize,
                        target: target,
                    }
                })
                .unwrap_or(PlayerTurnAction::EndTurn),
            Ok(PlayerTurnActionEnum::PlayerOperateMinion) => input
                .player_operate_minion
                .map(|action| {
                    let Some(target) = action.target.map(parse_target).unwrap_or(None) else {
                        return PlayerTurnAction::EndTurn;
                    };
                    PlayerTurnAction::MinionAttack {
                        attacker: action.minion_id,
                        target,
                    }
                })
                .unwrap_or(PlayerTurnAction::EndTurn),
            Ok(PlayerTurnActionEnum::PlayerEndTurn) => PlayerTurnAction::EndTurn,
            Err(_) => PlayerTurnAction::EndTurn,
        }
    }
}

fn parse_target(target: idl_gen::bss_heartstone::Target) -> Option<heartstone::model::Target> {
    match (target.minion_id, target.player) {
        (None, None) => None,
        (None, Some(id)) => Some(heartstone::model::Target::Hero(id)),
        (Some(id), None) => Some(heartstone::model::Target::Minion(id)),
        (Some(_), Some(_)) => None, // not support this case
    }
}
