use crate::{
    common::{
        input::{InputManager, InputWatcher},
        room::SafeRoom,
    },
    rpc,
};
use heartstone::{
    game::Game,
    model::Fightline,
    player::{Player, PlayerBehavior, PlayerStartingAction, PlayerTurnAction},
};
use idl_gen::{
    bss_heartstone::{
        MyTurnEndEvent, MyTurnStartEvent, PlayerEndTurnAction, PlayerTurnActionEnum, Position,
        StartingTurnAction, StartingTurnActionEnum, StartingTurnStartEvent,
    },
    bss_websocket::{GameEvent, SendGameEventRequest},
    game_backend::GameType,
};
use protobuf::{Message, MessageField};
use std::{fmt::Debug, sync::Arc, time::Duration};
use tokio::sync::{Mutex, RwLock};

pub struct SocketPlayerBehavior {
    id: i64,
    uuid: RwLock<Option<u64>>,
    safe_room: SafeRoom,
    input: Arc<InputManager>,
    starting_watch: Mutex<Option<InputWatcher<StartingTurnAction>>>,
}

impl Debug for SocketPlayerBehavior {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SocketPlayerBehavior")
            .field("id", &self.id)
            .field("uuid", &self.uuid)
            .field("safe_room", &self.safe_room)
            .field("input", &self.input)
            .finish()
    }
}

impl SocketPlayerBehavior {
    pub fn new(id: i64, safe_room: SafeRoom, input: Arc<InputManager>) -> Self {
        Self {
            id,
            uuid: RwLock::new(None),
            safe_room,
            input,
            starting_watch: Mutex::new(None),
        }
    }
}

#[async_trait::async_trait]
impl PlayerBehavior for SocketPlayerBehavior {
    async fn assign_uuid(&self, uuid: u64) {
        *self.uuid.write().await = Some(uuid);
    }

    async fn next_starting_action(&self, player: &Player) -> Option<PlayerStartingAction> {
        let mut watcher = self.starting_watch.lock().await;
        if watcher.is_none() {
            let input_watcher = self.input.register_input_watcher(self.id).await;
            *watcher = Some(input_watcher);
            let room_id = self.safe_room.lock().await.get_room_id();
            let payload = StartingTurnStartEvent::default();
            let payload = match payload.write_to_bytes() {
                Ok(v) => v,
                Err(err) => {
                    log::error!("write to bytes error: {err}");
                    return None;
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
            };

            if let Err(err) = rpc::bss::client().send_game_event(req).await {
                log::error!("send game event error: {err}");
            }
        }

        let Some(ref mut watcher) = *watcher else {
            log::error!("watcher is none, but we should create it before");
            return None;
        };

        loop {
            let input = match watcher.get_next_input().await {
                Ok(v) => v,
                Err(err) => {
                    log::error!("get next input error: {err}");
                    return None;
                }
            };

            match input.action.enum_value() {
                Ok(StartingTurnActionEnum::Stop) => return None,
                Ok(StartingTurnActionEnum::SelectPosition) => {
                    if let Some(action) = input.select_position_action.into_option() {
                        let fightline = match action.position.map(|position| position.enum_value())
                        {
                            Some(Ok(Position::Front)) => Some(Fightline::Front),
                            Some(Ok(Position::Back)) => Some(Fightline::Back),
                            None => None,
                            Some(Err(unknown_enum)) => {
                                log::error!("unknown enum: {unknown_enum}");
                                continue;
                            }
                        };

                        return Some(PlayerStartingAction::ChooseFightline { fightline });
                    }
                }
                Ok(StartingTurnActionEnum::LockPosition) => {
                    return Some(PlayerStartingAction::LockFightline);
                }
                Ok(StartingTurnActionEnum::UnlockPosition) => {
                    return Some(PlayerStartingAction::UnlockFightline);
                }
                Ok(StartingTurnActionEnum::ReplaceStartingCard) => {
                    if let Some(action) = input.replace_starting_card_action.into_option() {
                        // 需要在此处检查位置是否有效
                        if action
                            .card_index
                            .iter()
                            .any(|index| *index < 0 || *index > 4)
                        {
                            continue;
                        }
                        let cards_index = action
                            .card_index
                            .iter()
                            .map(|index| *index as usize)
                            .collect();
                        return Some(PlayerStartingAction::SwapStartingCards { cards_index });
                    }
                }
                Err(unknown_enum) => {
                    log::error!("unknown enum: {unknown_enum}");
                }
            }
        }
    }

    async fn finish_starting_action(&self, _player: &Player) {
        if let Some(watcher) = self.starting_watch.lock().await.take() {
            self.input.unregister_input_watcher(watcher).await;
        }
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
            };

            if let Err(err) = rpc::bss::client().send_game_event(req).await {
                log::error!("send game event error: {err}");
            }
        };
        let on_stop_listen_input = async {
            let payload = MyTurnEndEvent::default();
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
                    event_type: MyTurnEndEvent::NAME.into(),
                    payload: payload.into(),
                }],
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
                        target,
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
