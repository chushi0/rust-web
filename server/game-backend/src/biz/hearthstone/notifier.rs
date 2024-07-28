use std::collections::HashMap;

use crate::{common::room::SafeRoom, rpc};
use anyhow::Result;
use datastructure::AsyncIter;
use heartstone::{
    api::{GameNotifier, GameRunningNotifier, GameStartingNotifier, PlayerDrawCard, TurnAction},
    game::Game,
    model::{
        Buff, Buffable, Camp, Card, Damageable, Fightline, HeroTrait, Minion, MinionTrait, Target,
    },
    player::PlayerTrait,
};
use idl_gen::{
    bff_heartstone::{
        GamePlayer, GameStartEvent, MinionStatus, PlayerStatus, Position, RandomGroupEvent,
        SyncGameStatus,
    },
    bff_websocket::{GameEvent, SendGameEventRequest},
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
    starting_sender: UnboundedSender<StartingNotifyEvent>,
    running_sender: UnboundedSender<RunningNotifyEvent>,
}

#[derive(Debug)]
struct NotifierInternal {
    safe_room: SafeRoom,
    starting_receiver: UnboundedReceiver<StartingNotifyEvent>,
    running_receiver: UnboundedReceiver<RunningNotifyEvent>,

    players: HashMap<u64, Player>,
}

#[derive(Debug)]
struct Player {
    index: Option<usize>,
    uuid: u64,
    user_id: Option<i64>,
    camp: Option<Camp>,
}

impl Notifier {
    pub fn new(safe_room: SafeRoom) -> Self {
        let (starting_sender, starting_receiver) = tokio::sync::mpsc::unbounded_channel();
        let (running_sender, running_receiver) = tokio::sync::mpsc::unbounded_channel();

        Self {
            inner: Mutex::new(NotifierInternal {
                safe_room,
                starting_receiver,
                running_receiver,
                players: HashMap::new(),
            }),
            starting_sender,
            running_sender,
        }
    }
}

impl NotifierInternal {
    async fn flush_at_starting(&mut self) {
        let mut events = Vec::new();
        let mut game_start_event = false;
        let mut random_group_event = false;

        while let Ok(event) = self.starting_receiver.try_recv() {
            if let StartingNotifyEvent::PlayerUuid { uuid, custom_id } = event {
                let index = self
                    .safe_room
                    .lock()
                    .await
                    .player_index(custom_id)
                    .expect("player should in room");

                self.players
                    .entry(uuid)
                    .and_modify(|player| player.user_id = Some(custom_id))
                    .and_modify(|player| player.index = Some(index))
                    .or_insert(Player {
                        index: Some(index),
                        user_id: Some(custom_id),
                        uuid,
                        camp: None,
                    });

                game_start_event = true;
                continue;
            }
            if let StartingNotifyEvent::CampDecide { player, camp } = event {
                self.players
                    .entry(player)
                    .and_modify(|player| player.camp = Some(camp))
                    .or_insert(Player {
                        index: None,
                        user_id: None,
                        uuid: player,
                        camp: Some(camp),
                    });
                random_group_event = true;
                continue;
            }

            events.push(event);
        }

        if events.is_empty() && !game_start_event && !random_group_event {
            return;
        }

        let room_id = self.safe_room.lock().await.get_room_id();

        self.players
            .values()
            .filter(|player| match player.user_id {
                Some(id) => id > 0,
                None => false,
            })
            .async_for_each(|player| {
                let players = &self.players;

                let events = events
                    .iter()
                    .map(|event| event.to_user_event(player.uuid, players.values().collect()))
                    .collect::<Result<Vec<Option<GameEvent>>>>();

                async move {
                    let Ok(events) = events else {
                        log::error!("pack game event fail");
                        return;
                    };

                    let mut events: Vec<_> = events.into_iter().flatten().collect();

                    if game_start_event {
                        let event = match pack_game_event(GameStartEvent {
                            players: players
                                .values()
                                .map(|player| GamePlayer {
                                    index: player.index.expect("index should exist") as i32,
                                    uuid: player.uuid,
                                    ..Default::default()
                                })
                                .collect(),
                            ..Default::default()
                        }) {
                            Ok(v) => v,
                            Err(err) => {
                                log::error!("write to bytes error: {err}");
                                return;
                            }
                        };

                        events.push(event)
                    }

                    if random_group_event {
                        let event = match pack_game_event(RandomGroupEvent {
                            group_players_1: players
                                .values()
                                .filter(|player| {
                                    player.camp.map(|camp| camp == Camp::A).unwrap_or(false)
                                })
                                .map(|player| player.uuid)
                                .collect(),
                            group_players_2: players
                                .values()
                                .filter(|player| {
                                    player.camp.map(|camp| camp == Camp::B).unwrap_or(false)
                                })
                                .map(|player| player.uuid)
                                .collect(),
                            ..Default::default()
                        }) {
                            Ok(v) => v,
                            Err(err) => {
                                log::error!("write to bytes error: {err}");
                                return;
                            }
                        };

                        events.push(event)
                    }

                    let req = SendGameEventRequest {
                        user_id: vec![player.user_id.expect("we should get user_id now")],
                        game_type: GameType::HEARTHSTONE.inner(),
                        room_id,
                        event_list: events,
                    };

                    if let Err(err) = rpc::bff::client().send_game_event(req).await {
                        log::error!(
                            "send game event fail: uuid={}, id={}, err={err:?}",
                            player.uuid,
                            player.user_id.expect("we should get user_id now")
                        );
                    }
                }
            })
            .await
    }

    async fn flush(&mut self, game: &Game) {
        let mut events = Vec::new();
        while let Ok(event) = self.running_receiver.try_recv() {
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
                        game_type: GameType::HEARTHSTONE.inner(),
                        room_id,
                        event_list: events,
                    };

                    if let Err(err) = rpc::bff::client().send_game_event(req).await {
                        log::error!("send game event fail: uuid={uuid}, id={id}, err={err:?}");
                    }
                }
            })
            .await;
    }
}

#[derive(Debug)]
enum StartingNotifyEvent {
    PlayerUuid {
        uuid: u64,
        custom_id: i64,
    },
    CampDecide {
        player: u64,
        camp: Camp,
    },
    StartingCard {
        player: u64,
        cards: Vec<Card>,
    },
    ChangeStartingCard {
        player: u64,
        change_card_index: Vec<usize>,
        cards: Vec<Card>,
    },
    FightlineChoose {
        player: u64,
        fightline: Option<Fightline>,
    },
    FightlineLock {
        player: u64,
    },
    FightlineUnlock {
        player: u64,
    },
    FightlineDecide {
        player: u64,
        fightline: Fightline,
    },
}

impl StartingNotifyEvent {
    fn to_user_event(&self, uuid: u64, players: Vec<&Player>) -> Result<Option<GameEvent>> {
        use idl_gen::bff_heartstone::*;

        let is_same_camp = |player1, player2| {
            if player1 == player2 {
                return true;
            }

            let mut camp = None;
            for player in &players {
                if player.uuid == player1 || player.uuid == player2 {
                    match (camp, player.camp) {
                        (_, None) => return false,
                        (None, c) => camp = c,
                        (Some(c1), Some(c2)) => return c1 == c2,
                    }
                }
            }

            false
        };

        match self {
            StartingNotifyEvent::PlayerUuid {
                uuid: _,
                custom_id: _,
            } => Ok(None),
            StartingNotifyEvent::CampDecide { player: _, camp: _ } => Ok(None),
            StartingNotifyEvent::StartingCard { player, cards } => {
                if *player == uuid {
                    Ok(Some(pack_game_event(DrawStartingCardEvent {
                        cards: cards
                            .iter()
                            .map(|card| Card {
                                card_code: card.model().card.code.clone(),
                                ..Default::default()
                            })
                            .collect(),
                        ..Default::default()
                    })?))
                } else {
                    Ok(None)
                }
            }
            StartingNotifyEvent::ChangeStartingCard {
                player,
                change_card_index,
                cards,
            } => {
                if *player == uuid {
                    Ok(Some(pack_game_event(ReplaceStartingCardAction {
                        card_index: change_card_index
                            .iter()
                            .map(|index| *index as i32)
                            .collect(),
                        cards: cards
                            .iter()
                            .map(|card| Card {
                                card_code: card.model().card.code.clone(),
                                ..Default::default()
                            })
                            .collect(),
                        ..Default::default()
                    })?))
                } else {
                    Ok(None)
                }
            }
            StartingNotifyEvent::FightlineChoose { player, fightline } => {
                if is_same_camp(*player, uuid) {
                    Ok(Some(pack_game_event(PlayerSelectPositionEvent {
                        player: *player,
                        position: fightline.map(|fightline| {
                            match fightline {
                                Fightline::Front => Position::Front,
                                Fightline::Back => Position::Back,
                            }
                            .into()
                        }),
                        ..Default::default()
                    })?))
                } else {
                    Ok(None)
                }
            }
            StartingNotifyEvent::FightlineLock { player } => {
                if is_same_camp(*player, uuid) {
                    Ok(Some(pack_game_event(PlayerLockPositionEvent {
                        player: *player,
                        ..Default::default()
                    })?))
                } else {
                    Ok(None)
                }
            }
            StartingNotifyEvent::FightlineUnlock { player } => {
                if is_same_camp(*player, uuid) {
                    Ok(Some(pack_game_event(PlayerUnlockPositionEvent {
                        player: *player,
                        ..Default::default()
                    })?))
                } else {
                    Ok(None)
                }
            }
            StartingNotifyEvent::FightlineDecide { player, fightline } => {
                Ok(Some(pack_game_event(ServerDecidePlayerPositionEvent {
                    player: *player,
                    position: match fightline {
                        Fightline::Front => Position::Front,
                        Fightline::Back => Position::Back,
                    }
                    .into(),
                    ..Default::default()
                })?))
            }
        }
    }
}

#[derive(Debug)]
enum RunningNotifyEvent {
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

impl RunningNotifyEvent {
    fn to_user_event(&self, uuid: u64) -> Result<GameEvent> {
        use idl_gen::bff_heartstone::*;

        match self {
            RunningNotifyEvent::NewTurn { current_turn } => pack_game_event(match current_turn {
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
            RunningNotifyEvent::PlayerManaChange { player, mana } => {
                pack_game_event(PlayerManaChange {
                    player_uuid: *player,
                    mana: *mana,
                    ..Default::default()
                })
            }
            RunningNotifyEvent::PlayerDrawCard { player, card } => {
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
            RunningNotifyEvent::PlayerUseCard {
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
            RunningNotifyEvent::PlayerCardEffectEnd => {
                pack_game_event(PlayerUseCardEndEvent::default())
            }
            RunningNotifyEvent::PlayerSwapFightline {
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
            RunningNotifyEvent::MinionSummon { minion, camp } => {
                pack_game_event(MinionEnterEvent {
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
                })
            }
            RunningNotifyEvent::MinionBattlecry { minion } => pack_game_event(MinionEffectEvent {
                minion_id: minion.uuid(),
                minion_effect: MinionEffect::Battlecry.into(),
                ..Default::default()
            }),
            RunningNotifyEvent::MinionAttack { minion, target } => {
                pack_game_event(MinionAttackEvent {
                    minion_id: minion.uuid(),
                    target: MessageField::some(pack_target(target)),
                    ..Default::default()
                })
            }
            RunningNotifyEvent::MinionDeath { minion } => pack_game_event(MinionRemoveEvent {
                minion_id: minion.uuid(),
                ..Default::default()
            }),
            RunningNotifyEvent::MinionDeathrattle { minion } => {
                pack_game_event(MinionEffectEvent {
                    minion_id: minion.uuid(),
                    minion_effect: MinionEffect::Deathrattle.into(),
                    ..Default::default()
                })
            }
            RunningNotifyEvent::DealDamage { target, damage } => pack_game_event(DamageEvent {
                target: MessageField::some(pack_target(target)),
                damage: *damage,
                ..Default::default()
            }),
            RunningNotifyEvent::Buff { target, buff } => pack_game_event(BuffEvent {
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
                                idl_gen::bff_heartstone::Card {
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
                    card: MessageField::some(idl_gen::bff_heartstone::Card {
                        card_code: minion.model().await.card.code.clone(),
                        ..Default::default()
                    }),
                    atk: minion.atk().await,
                    hp: minion.hp().await,
                    buff_list: minion
                        .buff_list()
                        .await
                        .iter()
                        .map(|buff| idl_gen::bff_heartstone::Buff {
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

fn pack_target(target: &heartstone::model::Target) -> idl_gen::bff_heartstone::Target {
    match target {
        heartstone::model::Target::Minion(id) => idl_gen::bff_heartstone::Target {
            minion_id: Some(*id),
            ..Default::default()
        },
        heartstone::model::Target::Hero(id) => idl_gen::bff_heartstone::Target {
            player: Some(*id),
            ..Default::default()
        },
    }
}

#[async_trait::async_trait]
impl GameStartingNotifier for Notifier {
    async fn flush_at_starting(&self) {
        self.inner.lock().await.flush_at_starting().await
    }

    fn player_uuid(&self, uuid: u64, custom_id: i64) {
        self.starting_sender
            .send(StartingNotifyEvent::PlayerUuid { uuid, custom_id })
            .unwrap()
    }

    fn camp_decide(&self, player: u64, camp: Camp) {
        self.starting_sender
            .send(StartingNotifyEvent::CampDecide { player, camp })
            .unwrap()
    }

    fn starting_card(&self, player: u64, cards: Vec<Card>) {
        self.starting_sender
            .send(StartingNotifyEvent::StartingCard { player, cards })
            .unwrap()
    }

    fn change_starting_card(&self, player: u64, change_card_index: &[usize], new_cards: Vec<Card>) {
        self.starting_sender
            .send(StartingNotifyEvent::ChangeStartingCard {
                player,
                change_card_index: change_card_index.to_vec(),
                cards: new_cards,
            })
            .unwrap()
    }

    fn fightline_choose(&self, player: u64, fightline: Option<Fightline>) {
        self.starting_sender
            .send(StartingNotifyEvent::FightlineChoose { player, fightline })
            .unwrap()
    }

    fn fightline_lock(&self, player: u64, _fightline: Fightline) {
        self.starting_sender
            .send(StartingNotifyEvent::FightlineLock { player })
            .unwrap()
    }

    fn fightline_unlock(&self, player: u64) {
        self.starting_sender
            .send(StartingNotifyEvent::FightlineUnlock { player })
            .unwrap()
    }

    fn fightline_decide(&self, player: u64, fightline: Fightline) {
        self.starting_sender
            .send(StartingNotifyEvent::FightlineDecide { player, fightline })
            .unwrap()
    }
}

#[async_trait::async_trait]
impl GameRunningNotifier for Notifier {
    async fn flush(&self, game: &Game) {
        self.inner.lock().await.flush(game).await
    }

    fn new_turn(&self, current_turn: TurnAction) {
        self.running_sender
            .send(RunningNotifyEvent::NewTurn { current_turn })
            .unwrap()
    }

    fn player_mana_change(&self, player: u64, mana: i32) {
        self.running_sender
            .send(RunningNotifyEvent::PlayerManaChange { player, mana })
            .unwrap()
    }

    fn player_draw_card(&self, player: u64, card: PlayerDrawCard) {
        self.running_sender
            .send(RunningNotifyEvent::PlayerDrawCard { player, card })
            .unwrap()
    }

    fn player_use_card(&self, player: u64, card: Card, cost_mana: i32) {
        self.running_sender
            .send(RunningNotifyEvent::PlayerUseCard {
                player,
                card,
                cost_mana,
            })
            .unwrap()
    }

    fn player_card_effect_end(&self) {
        self.running_sender
            .send(RunningNotifyEvent::PlayerCardEffectEnd)
            .unwrap()
    }

    fn player_swap_fightline(&self, player: u64, new_fightline: Fightline) {
        self.running_sender
            .send(RunningNotifyEvent::PlayerSwapFightline {
                player,
                new_fightline,
            })
            .unwrap()
    }

    fn minion_summon(&self, minion: Minion, camp: Camp) {
        self.running_sender
            .send(RunningNotifyEvent::MinionSummon { minion, camp })
            .unwrap()
    }

    fn minion_battlecry(&self, minion: Minion) {
        self.running_sender
            .send(RunningNotifyEvent::MinionBattlecry { minion })
            .unwrap()
    }

    fn minion_attack(&self, minion: Minion, target: Target) {
        self.running_sender
            .send(RunningNotifyEvent::MinionAttack { minion, target })
            .unwrap()
    }

    fn minion_death(&self, minion: Minion) {
        self.running_sender
            .send(RunningNotifyEvent::MinionDeath { minion })
            .unwrap()
    }

    fn minion_deathrattle(&self, minion: Minion) {
        self.running_sender
            .send(RunningNotifyEvent::MinionDeathrattle { minion })
            .unwrap()
    }

    fn deal_damage(&self, target: Target, damage: i64) {
        self.running_sender
            .send(RunningNotifyEvent::DealDamage { target, damage })
            .unwrap()
    }

    fn buff(&self, target: Target, buff: Buff) {
        self.running_sender
            .send(RunningNotifyEvent::Buff { target, buff })
            .unwrap()
    }
}

impl GameNotifier for Notifier {}
