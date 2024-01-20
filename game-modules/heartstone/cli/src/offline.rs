use datastructure::AsyncIter;
use heartstone::{
    api::{GameNotifier, GameRunningNotifier, GameStartingNotifier, PlayerDrawCard, TurnAction},
    game::{Config, Game, PlayerConfig},
    model::{
        Buff, Buffable, Camp, Card, CardModel, Damageable, Fightline, HeroTrait, Minion,
        MinionTrait, Target,
    },
    player::{Player, PlayerBehavior, PlayerStartingAction, PlayerTrait, PlayerTurnAction},
};
use idl_gen::bss_heartstone::{
    BuffEvent, DamageEvent, DrawCardEvent, DrawCardResult, MinionAttackEvent, MinionEffect,
    MinionEffectEvent, MinionEnterEvent, MinionRemoveEvent, MinionStatus, NewTurnEvent,
    PlayerManaChange, PlayerStatus, PlayerTurnEvent, PlayerUseCardEndEvent, PlayerUseCardEvent,
    Position, SwapFrontBackEvent, SwapTurnEvent, SyncGameStatus, TurnTypeEnum,
};
use protobuf::{EnumOrUnknown, MessageField};
use std::{collections::HashMap, sync::Arc};

#[derive(Debug)]
pub struct Client;

#[derive(Debug)]
struct StdBehavior;

#[derive(Debug)]
struct StdNotifier;

#[async_trait::async_trait]
impl crate::Client for Client {
    async fn run(&self) {
        // 加载数据库
        let card_pool = load_card_pool().await;

        let config = Config {
            game_notifier: Arc::new(StdNotifier),
            card_pool: card_pool.clone(),
            ..Default::default()
        };

        let mut players = Vec::new();
        for _ in 0..4 {
            // 牌库是每张牌（衍生牌除外）各一张
            let deck = card_pool
                .iter()
                .filter(|(_, model)| !model.card.derive)
                .map(|(id, _)| (*id, 1))
                .collect();

            let behavior = Arc::new(StdBehavior) as Arc<dyn PlayerBehavior>;

            players.push(PlayerConfig {
                behavior,
                deck,
                ..Default::default()
            })
        }

        Game::new(config, players).await.run().await;
        println!("游戏结束");
    }
}

async fn load_card_pool() -> HashMap<i64, Arc<CardModel>> {
    let mut db = web_db::create_connection(web_db::RDS::Hearthstone)
        .await
        .unwrap();
    let mut tx = web_db::begin_tx(&mut db).await.unwrap();

    let cards = web_db::hearthstone::get_all_cards(&mut tx).await.unwrap();

    let mut map = HashMap::new();
    for card in cards {
        let card_info = serde_json::from_str(&card.card_info).unwrap();
        map.insert(card.rowid, Arc::new(CardModel { card, card_info }));
    }

    crate::io().cache_cards(&map.values().map(Arc::clone).collect::<Vec<_>>());

    map
}

#[async_trait::async_trait]
#[allow(unused_variables)]
impl GameStartingNotifier for StdNotifier {
    async fn flush_at_starting(&self) {}

    fn player_uuid(&self, uuid: u64, custom_id: i64) {}

    fn camp_decide(&self, player: u64, camp: Camp) {
        println!("camp decide: player={player}, camp={camp:?}")
    }

    fn starting_card(&self, player: u64, cards: Vec<Card>) {
        println!("starting_card: player={player}, cards={cards:?}")
    }

    fn change_starting_card(&self, player: u64, change_card_index: &[usize], new_cards: Vec<Card>) {
        println!("change_starting_card: player={player}, change_card_index={change_card_index:?}, new_cards={new_cards:?}")
    }

    fn fightline_choose(&self, player: u64, fightline: Option<Fightline>) {
        println!("fightline_choose: player={player}, fightline={fightline:?}")
    }

    fn fightline_lock(&self, player: u64, fightline: Fightline) {
        println!("fightline lock: player={player}, fightline={fightline:?}")
    }

    fn fightline_unlock(&self, player: u64) {
        println!("fightline unlock: player={player}")
    }

    fn fightline_decide(&self, player: u64, fightline: Fightline) {
        println!("fightline decide: player={player}, fightline={fightline:?}")
    }
}

#[async_trait::async_trait]
#[allow(unused_variables)]
impl GameRunningNotifier for StdNotifier {
    async fn flush(&self, game: &Game) {
        let game_status = SyncGameStatus {
            player_status: game
                .players()
                .iter()
                .async_map(|player| async move {
                    PlayerStatus {
                        uuid: player.uuid().await,
                        room_index: player.get_custom_id().await,
                        card_count: player.hand_cards().await.len() as i32,
                        cards: player
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
                            .collect(),
                        hp: player.get_hero().await.hp().await,
                        mana: player.mana().await,
                        position: EnumOrUnknown::new(
                            match player.get_hero().await.fightline().await {
                                Fightline::Front => Position::Front,
                                Fightline::Back => Position::Back,
                            },
                        ),
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
        };

        crate::io().print_game_status(game_status);
    }

    fn new_turn(&self, current_turn: TurnAction) {
        let event = match current_turn {
            TurnAction::PlayerTurn(uuid) => NewTurnEvent {
                turn_type: TurnTypeEnum::PlayerTurn.into(),
                player_turn: MessageField::some(PlayerTurnEvent {
                    player_uuid: uuid,
                    ..Default::default()
                }),
                ..Default::default()
            },
            TurnAction::SwapFightline => NewTurnEvent {
                turn_type: TurnTypeEnum::SwapTurn.into(),
                swap_turn: MessageField::some(SwapTurnEvent::default()),
                ..Default::default()
            },
        };

        crate::io().print_new_turn(event);
    }

    fn player_mana_change(&self, player: u64, mana: i32) {
        let event = PlayerManaChange {
            player_uuid: player,
            mana,
            ..Default::default()
        };
        crate::io().print_player_mana_change(event);
    }

    fn player_draw_card(&self, player: u64, card: PlayerDrawCard) {
        let card_model = match &card {
            PlayerDrawCard::Draw(card) => Some(card),
            PlayerDrawCard::Fire(card) => Some(card),
            PlayerDrawCard::Tired(_) => None,
        }
        .map(|card| idl_gen::bss_heartstone::Card {
            card_code: card.model().card.code.clone(),
            ..Default::default()
        });

        let draw_card_result = match &card {
            PlayerDrawCard::Draw(_) => DrawCardResult::Ok,
            PlayerDrawCard::Fire(_) => DrawCardResult::Fire,
            PlayerDrawCard::Tired(_) => DrawCardResult::Tired,
        }
        .into();

        let tired = match card {
            PlayerDrawCard::Tired(v) => Some(v),
            _ => None,
        };

        let event = DrawCardEvent {
            player_uuid: player,
            card: card_model.into(),
            draw_card_result,
            tired,
            ..Default::default()
        };

        crate::io().print_player_draw_card(event);
    }

    fn player_use_card(&self, player: u64, card: Card, cost_mana: i32) {
        let event = PlayerUseCardEvent {
            player_uuid: player,
            card_index: 0,
            card: MessageField::some(idl_gen::bss_heartstone::Card {
                card_code: card.model().card.code.clone(),
                ..Default::default()
            }),
            cost_mana,
            ..Default::default()
        };

        crate::io().print_player_use_card(event);
    }

    fn player_card_effect_end(&self) {
        crate::io().print_player_card_effect_end(PlayerUseCardEndEvent::default());
    }

    fn player_swap_fightline(&self, player: u64, new_fightline: Fightline) {
        let event = SwapFrontBackEvent {
            player_uuid: player,
            new_position: match new_fightline {
                Fightline::Front => Position::Front,
                Fightline::Back => Position::Back,
            }
            .into(),
            ..Default::default()
        };

        crate::io().print_player_swap_fightline(event);
    }

    fn minion_summon(&self, minion: Minion, camp: Camp) {
        let event = MinionEnterEvent {
            minion_id: minion.uuid(),
            card: MessageField::some(idl_gen::bss_heartstone::Card {
                card_code: minion.model().card.code.clone(),
                ..Default::default()
            }),
            group: camp as i32,
            index: 0,
            atk: minion.atk(),
            hp: minion.hp(),
            ..Default::default()
        };

        crate::io().print_minion_enter(event);
    }

    fn minion_battlecry(&self, minion: Minion) {
        let event = MinionEffectEvent {
            minion_id: minion.uuid(),
            minion_effect: MinionEffect::Battlecry.into(),
            ..Default::default()
        };

        crate::io().print_minion_effect(event);
    }

    fn minion_attack(&self, minion: Minion, target: Target) {
        let event = MinionAttackEvent {
            minion_id: minion.uuid(),
            target: MessageField::some(pack_target(target)),
            ..Default::default()
        };

        crate::io().print_minion_attack(event);
    }

    fn minion_death(&self, minion: Minion) {
        let event = MinionRemoveEvent {
            minion_id: minion.uuid(),
            ..Default::default()
        };

        crate::io().print_minion_remove(event);
    }

    fn minion_deathrattle(&self, minion: Minion) {
        let event = MinionEffectEvent {
            minion_id: minion.uuid(),
            minion_effect: MinionEffect::Deathrattle.into(),
            ..Default::default()
        };

        crate::io().print_minion_effect(event);
    }

    fn deal_damage(&self, target: Target, damage: i64) {
        let event = DamageEvent {
            target: MessageField::some(pack_target(target)),
            damage,
            ..Default::default()
        };

        crate::io().print_deal_damage(event);
    }

    fn buff(&self, target: Target, buff: Buff) {
        let event = BuffEvent {
            target: MessageField::some(pack_target(target)),
            buff: MessageField::some(idl_gen::bss_heartstone::Buff {
                atk_boost: buff.atk_boost(),
                hp_boost: buff.hp_boost(),
                ..Default::default()
            }),
            ..Default::default()
        };

        crate::io().print_buff(event);
    }
}

impl GameNotifier for StdNotifier {}

#[async_trait::async_trait]
impl PlayerBehavior for StdBehavior {
    async fn assign_uuid(&self, _uuid: u64) {}

    async fn next_starting_action(&self, _player: &Player) -> Option<PlayerStartingAction> {
        None
    }

    async fn finish_starting_action(&self, _player: &Player) {}

    async fn next_action(&self, _game: &Game, _player: &Player) -> PlayerTurnAction {
        let action = crate::io().next_action();

        match action.action_type.enum_value().unwrap() {
            idl_gen::bss_heartstone::PlayerTurnActionEnum::PlayerEndTurn => {
                PlayerTurnAction::EndTurn
            }
            idl_gen::bss_heartstone::PlayerTurnActionEnum::PlayerUseCard => {
                let player_use_card = action.player_use_card.unwrap();
                PlayerTurnAction::PlayCard {
                    hand_index: player_use_card.card_index as usize,
                    target: parse_target(player_use_card.target),
                }
            }
            idl_gen::bss_heartstone::PlayerTurnActionEnum::PlayerOperateMinion => {
                let player_operate_minion = action.player_operate_minion.unwrap();
                PlayerTurnAction::MinionAttack {
                    attacker: player_operate_minion.minion_id,
                    target: parse_target(player_operate_minion.target).unwrap(),
                }
            }
        }
    }
}

fn parse_target(target: MessageField<idl_gen::bss_heartstone::Target>) -> Option<Target> {
    target
        .map(|target| match (target.minion_id, target.player) {
            (None, Some(player)) => Some(Target::Hero(player)),
            (Some(minion), None) => Some(Target::Minion(minion)),
            _ => panic!("target is not acceptable"),
        })
        .unwrap_or(None)
}

fn pack_target(target: heartstone::model::Target) -> idl_gen::bss_heartstone::Target {
    match target {
        heartstone::model::Target::Minion(id) => idl_gen::bss_heartstone::Target {
            minion_id: Some(id),
            ..Default::default()
        },
        heartstone::model::Target::Hero(id) => idl_gen::bss_heartstone::Target {
            player: Some(id),
            ..Default::default()
        },
    }
}
