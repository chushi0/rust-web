use crate::{common::room::SafeRoom, rpc};
use anyhow::Result;
use datastructure::AsyncIter;
use heartstone::{
    api::{GameNotifier, PlayerDrawCard, TurnAction},
    game::Game,
    model::{
        Buff, Buffable, Camp, Card, Damageable, Fightline, HeroTrait, Minion, MinionTrait, Target,
    },
    player::PlayerTrait,
};
use idl_gen::{
    bss_heartstone::{MinionStatus, PlayerStatus, Position, SyncGameStatus},
    bss_websocket::{GameEvent, SendGameEventRequest},
    game_backend::GameType,
};
use protobuf::{EnumOrUnknown, Message, MessageField};
use tokio::sync::{
    mpsc::{UnboundedReceiver, UnboundedSender},
    Mutex,
};

#[derive(Debug)]
pub struct Notifier {
    inner: Mutex<NotifierInternal>,
    sender: UnboundedSender<NotifyEvent>,
}

#[derive(Debug)]
struct NotifierInternal {
    safe_room: SafeRoom,
    receiver: UnboundedReceiver<NotifyEvent>,
}

impl Notifier {
    pub fn new(safe_room: SafeRoom) -> Self {
        let (sender, receiver) = tokio::sync::mpsc::unbounded_channel();

        Self {
            inner: Mutex::new(NotifierInternal {
                safe_room,
                receiver,
            }),
            sender,
        }
    }
}

impl NotifierInternal {
    async fn flush(&mut self, game: &Game) {
        let mut events = Vec::new();
        while let Ok(event) = self.receiver.try_recv() {
            events.push(event);
        }
        if events.is_empty() {
            return;
        }

        let room_id = self.safe_room.lock().await.get_room_id();

        game.players()
            .iter()
            .async_map(|player| async {
                (
                    player.get_hero().await.uuid().await,
                    player.get_custom_id().await,
                )
            })
            .await
            .filter(|(_uuid, id)| *id > 0)
            .async_for_each(|(uuid, id)| {
                let events = events
                    .iter()
                    .map(|event| event.to_user_event(uuid))
                    .collect::<Result<Vec<GameEvent>>>();

                async move {
                    let Ok(mut events) = events else {
                        log::error!("pack game event fail");
                        return;
                    };
                    let Ok(sync_game_status) = pack_sync_game_status(game, uuid).await else {
                        log::error!("pack sync game status fail");
                        return;
                    };
                    events.push(sync_game_status);

                    let req = SendGameEventRequest {
                        user_id: vec![id],
                        game_type: GameType::Hearthstone as i32,
                        room_id,
                        event_list: events,
                    };

                    if let Err(err) = rpc::bss::client().send_game_event(req).await {
                        log::error!("send game event fail: uuid={uuid}, id={id}, err={err:?}");
                    }
                }
            })
            .await;
    }
}

#[derive(Debug)]
enum NotifyEvent {
    NewTurn {
        current_turn: TurnAction,
    },
    PlayerManaChange {
        player: u64,
        mana: i32,
    },
    PlayerDrawCard {
        player: u64,
        card: PlayerDrawCard,
    },
    PlayerUseCard {
        player: u64,
        card: Card,
        cost_mana: i32,
    },
    PlayerCardEffectEnd,
    PlayerSwapFightline {
        player: u64,
        new_fightline: Fightline,
    },
    MinionSummon {
        minion: Minion,
        camp: Camp,
    },
    MinionBattlecry {
        minion: Minion,
    },
    MinionAttack {
        minion: Minion,
        target: Target,
    },
    MinionDeath {
        minion: Minion,
    },
    MinionDeathrattle {
        minion: Minion,
    },
    DealDamage {
        target: Target,
        damage: i64,
    },
    Buff {
        target: Target,
        buff: Buff,
    },
}

impl NotifyEvent {
    fn to_user_event(&self, uuid: u64) -> Result<GameEvent> {
        use idl_gen::bss_heartstone::*;

        match self {
            NotifyEvent::NewTurn { current_turn } => pack_game_event(match current_turn {
                TurnAction::PlayerTurn(uuid) => NewTurnEvent {
                    turn_type: TurnTypeEnum::PlayerTurn.into(),
                    player_turn: MessageField::some(PlayerTurnEvent {
                        player_uuid: *uuid,
                        ..Default::default()
                    }),
                    ..Default::default()
                },
                TurnAction::SwapFightline => NewTurnEvent {
                    turn_type: TurnTypeEnum::SwapTurn.into(),
                    swap_turn: MessageField::some(SwapTurnEvent::default()),
                    ..Default::default()
                },
            }),
            NotifyEvent::PlayerManaChange { player, mana } => pack_game_event(PlayerManaChange {
                player_uuid: *player,
                mana: *mana,
                ..Default::default()
            }),
            NotifyEvent::PlayerDrawCard { player, card } => {
                let card_model = match card {
                    // 抽到手牌中，只有玩家自己可以知道是什么牌
                    PlayerDrawCard::Draw(card) => {
                        if *player == uuid {
                            Some(card)
                        } else {
                            None
                        }
                    }
                    // 摧毁，那么所有人都可以看到是什么牌
                    PlayerDrawCard::Fire(card) => Some(card),
                    PlayerDrawCard::Tired(_) => None,
                }
                .map(|card| Card {
                    card_code: card.model().card.code.clone(),
                    ..Default::default()
                });

                let draw_card_result = match card {
                    PlayerDrawCard::Draw(_) => DrawCardResult::Ok,
                    PlayerDrawCard::Fire(_) => DrawCardResult::Fire,
                    PlayerDrawCard::Tired(_) => DrawCardResult::Tired,
                }
                .into();

                let tired = match card {
                    PlayerDrawCard::Tired(v) => Some(*v),
                    _ => None,
                };

                pack_game_event(DrawCardEvent {
                    player_uuid: *player,
                    card: card_model.into(),
                    draw_card_result,
                    tired,
                    ..Default::default()
                })
            }
            NotifyEvent::PlayerUseCard {
                player,
                card,
                cost_mana,
            } => pack_game_event(PlayerUseCardEvent {
                player_uuid: *player,
                card_index: 0,
                card: MessageField::some(Card {
                    card_code: card.model().card.code.clone(),
                    ..Default::default()
                }),
                cost_mana: *cost_mana,
                ..Default::default()
            }),
            NotifyEvent::PlayerCardEffectEnd => pack_game_event(PlayerUseCardEndEvent::default()),
            NotifyEvent::PlayerSwapFightline {
                player,
                new_fightline,
            } => pack_game_event(SwapFrontBackEvent {
                player_uuid: *player,
                new_position: match new_fightline {
                    Fightline::Front => Position::Front,
                    Fightline::Back => Position::Back,
                }
                .into(),
                ..Default::default()
            }),
            NotifyEvent::MinionSummon { minion, camp } => pack_game_event(MinionEnterEvent {
                minion_id: minion.uuid(),
                card: MessageField::some(Card {
                    card_code: minion.model().card.code.clone(),
                    ..Default::default()
                }),
                group: *camp as i32,
                index: 0,
                atk: minion.atk(),
                hp: minion.hp(),
                ..Default::default()
            }),
            NotifyEvent::MinionBattlecry { minion } => pack_game_event(MinionEffectEvent {
                minion_id: minion.uuid(),
                minion_effect: MinionEffect::Battlecry.into(),
                ..Default::default()
            }),
            NotifyEvent::MinionAttack { minion, target } => pack_game_event(MinionAttackEvent {
                minion_id: minion.uuid(),
                target: MessageField::some(pack_target(target)),
                ..Default::default()
            }),
            NotifyEvent::MinionDeath { minion } => pack_game_event(MinionRemoveEvent {
                minion_id: minion.uuid(),
                ..Default::default()
            }),
            NotifyEvent::MinionDeathrattle { minion } => pack_game_event(MinionEffectEvent {
                minion_id: minion.uuid(),
                minion_effect: MinionEffect::Deathrattle.into(),
                ..Default::default()
            }),
            NotifyEvent::DealDamage { target, damage } => pack_game_event(DamageEvent {
                target: MessageField::some(pack_target(target)),
                damage: *damage,
                ..Default::default()
            }),
            NotifyEvent::Buff { target, buff } => pack_game_event(BuffEvent {
                target: MessageField::some(pack_target(target)),
                buff: MessageField::some(Buff {
                    atk_boost: buff.atk_boost(),
                    hp_boost: buff.hp_boost(),
                    ..Default::default()
                }),
                ..Default::default()
            }),
        }
    }
}

async fn pack_sync_game_status(game: &Game, uuid: u64) -> Result<GameEvent> {
    pack_game_event(SyncGameStatus {
        player_status: game
            .players()
            .iter()
            .async_map(|player| async move {
                PlayerStatus {
                    uuid: player.uuid().await,
                    room_index: player.get_custom_id().await,
                    card_count: player.hand_cards().await.len() as i32,
                    cards: if player.uuid().await == uuid {
                        player
                            .hand_cards()
                            .await
                            .into_iter()
                            .async_map(|card| async move {
                                idl_gen::bss_heartstone::Card {
                                    card_code: card.get().await.model().card.code.clone(),
                                    ..Default::default()
                                }
                            })
                            .await
                            .collect()
                    } else {
                        vec![]
                    },
                    hp: player.get_hero().await.hp().await,
                    mana: player.mana().await,
                    position: EnumOrUnknown::new(match player.get_hero().await.fightline().await {
                        Fightline::Front => Position::Front,
                        Fightline::Back => Position::Back,
                    }),
                    camp: player.camp().await as i32,
                    ..Default::default()
                }
            })
            .await
            .collect(),
        minion_status: game
            .battlefield_minions(Camp::A)
            .await
            .iter()
            .map(|minion| (Camp::A, minion))
            .chain(
                game.battlefield_minions(Camp::B)
                    .await
                    .iter()
                    .map(|minion| (Camp::B, minion)),
            )
            .async_map(|(camp, minion)| async move {
                MinionStatus {
                    uuid: minion.uuid().await,
                    card: MessageField::some(idl_gen::bss_heartstone::Card {
                        card_code: minion.model().await.card.code.clone(),
                        ..Default::default()
                    }),
                    atk: minion.atk().await,
                    hp: minion.hp().await,
                    buff_list: minion
                        .buff_list()
                        .await
                        .iter()
                        .map(|buff| idl_gen::bss_heartstone::Buff {
                            atk_boost: buff.atk_boost(),
                            hp_boost: buff.hp_boost(),
                            ..Default::default()
                        })
                        .collect(),
                    camp: camp as i32,
                    ..Default::default()
                }
            })
            .await
            .collect(),
        ..Default::default()
    })
}

fn pack_game_event<Event: Message>(event: Event) -> Result<GameEvent> {
    Ok(GameEvent {
        event_type: Event::NAME.into(),
        payload: event.write_to_bytes()?.into(),
    })
}

fn pack_target(target: &heartstone::model::Target) -> idl_gen::bss_heartstone::Target {
    match target {
        heartstone::model::Target::Minion(id) => idl_gen::bss_heartstone::Target {
            minion_id: Some(*id),
            ..Default::default()
        },
        heartstone::model::Target::Hero(id) => idl_gen::bss_heartstone::Target {
            player: Some(*id),
            ..Default::default()
        },
    }
}

#[async_trait::async_trait]
impl GameNotifier for Notifier {
    async fn flush(&self, game: &Game) {
        self.inner.lock().await.flush(game).await
    }

    fn new_turn(&self, current_turn: TurnAction) {
        self.sender
            .send(NotifyEvent::NewTurn { current_turn })
            .unwrap()
    }

    fn player_mana_change(&self, player: u64, mana: i32) {
        self.sender
            .send(NotifyEvent::PlayerManaChange { player, mana })
            .unwrap()
    }

    fn player_draw_card(&self, player: u64, card: PlayerDrawCard) {
        self.sender
            .send(NotifyEvent::PlayerDrawCard { player, card })
            .unwrap()
    }

    fn player_use_card(&self, player: u64, card: Card, cost_mana: i32) {
        self.sender
            .send(NotifyEvent::PlayerUseCard {
                player,
                card,
                cost_mana,
            })
            .unwrap()
    }

    fn player_card_effect_end(&self) {
        self.sender.send(NotifyEvent::PlayerCardEffectEnd).unwrap()
    }

    fn player_swap_fightline(&self, player: u64, new_fightline: Fightline) {
        self.sender
            .send(NotifyEvent::PlayerSwapFightline {
                player,
                new_fightline,
            })
            .unwrap()
    }

    fn minion_summon(&self, minion: Minion, camp: Camp) {
        self.sender
            .send(NotifyEvent::MinionSummon { minion, camp })
            .unwrap()
    }

    fn minion_battlecry(&self, minion: Minion) {
        self.sender
            .send(NotifyEvent::MinionBattlecry { minion })
            .unwrap()
    }

    fn minion_attack(&self, minion: Minion, target: Target) {
        self.sender
            .send(NotifyEvent::MinionAttack { minion, target })
            .unwrap()
    }

    fn minion_death(&self, minion: Minion) {
        self.sender
            .send(NotifyEvent::MinionDeath { minion })
            .unwrap()
    }

    fn minion_deathrattle(&self, minion: Minion) {
        self.sender
            .send(NotifyEvent::MinionDeathrattle { minion })
            .unwrap()
    }

    fn deal_damage(&self, target: Target, damage: i64) {
        self.sender
            .send(NotifyEvent::DealDamage { target, damage })
            .unwrap()
    }

    fn buff(&self, target: Target, buff: Buff) {
        self.sender
            .send(NotifyEvent::Buff { target, buff })
            .unwrap()
    }
}
