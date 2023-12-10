use datastructure::SyncHandle;
use dialoguer::Input;
use heartstone::{
    api::{GameNotifier, PlayerDrawCard},
    game::{Config, Game, PlayerConfig, TurnAction},
    model::{Camp, Card, CardModel, Fightline, Minion, Target},
    player::{Player, PlayerBehavior, PlayerTurnAction},
};
use std::{collections::HashMap, sync::Arc};
use web_db::hearthstone::{CardType, MinionCardInfo};

#[derive(Debug)]
struct StdBehavior;

#[derive(Debug)]
struct StdNotifier;

#[tokio::main]
async fn main() {
    let test_card = Arc::new(CardModel {
        card: web_db::hearthstone::Card {
            rowid: 1,
            code: "".to_string(),
            name: "".to_string(),
            card_type: CardType::Minion.into(),
            mana_cost: 1,
            derive: false,
            need_select_target: true,
            card_info: "".to_string(),
            create_info: 0,
            update_time: 0,
            enable: true,
        },
        card_info: web_db::hearthstone::CardInfo {
            common_card_info: web_db::hearthstone::CommonCardInfo {},
            special_card_info: web_db::hearthstone::SpecialCardInfo::Minion(MinionCardInfo {
                attack: 1,
                health: 1,
                effects: vec![],
            }),
        },
    });

    let mut card_pool = HashMap::new();
    card_pool.insert(1, test_card);

    let config = Config {
        game_notifier: Arc::new(StdNotifier),
        card_pool,
        ..Default::default()
    };

    let mut players = Vec::new();
    for _ in 0..4 {
        let mut deck = HashMap::new();
        deck.insert(1, 5);
        players.push(PlayerConfig {
            behavior: Arc::new(StdBehavior),
            deck,
            ..Default::default()
        })
    }

    let result = Game::new(config, players).run().await;
    println!("result: {:?}", result);
}

#[async_trait::async_trait]
impl GameNotifier for StdNotifier {
    async fn flush(&self, game: &Game) {
        println!("flush")
    }

    fn new_turn(&self, current_turn: TurnAction) {
        println!("New Turn: {current_turn:?}")
    }

    fn player_mana_change(&self, player: u64, mana: i32) {
        println!("player mana change: {player:?}, mana: {mana}")
    }
    fn player_draw_card(&self, player: u64, card: PlayerDrawCard) {
        println!("player draw card: {player:?}, card: {card:?}")
    }
    fn player_use_card(&self, player: u64, card: Card, cost_mana: i32) {
        println!("player use card: {player:?}, card: {card:?} cost_mana: {cost_mana}")
    }
    fn player_card_effect_end(&self) {
        println!("player card effect end")
    }
    fn player_swap_fightline(&self, player: u64, new_fightline: Fightline) {
        println!("player swap fightline: {player:?}, new_fightline: {new_fightline:?}")
    }

    fn minion_summon(&self, minion: Minion, camp: Camp) {
        println!("minion summon: {minion:?}, camp: {camp:?}")
    }
    fn minion_battlecry(&self, minion: Minion) {
        println!("minion battlecry: {minion:?}")
    }
    fn minion_attack(&self, minion: Minion, target: Target) {
        println!("minion attack: {minion:?}, target: {target:?}")
    }
    fn minion_death(&self, minion: Minion) {
        println!("minion death: {minion:?}")
    }
    fn minion_deathrattle(&self, minion: Minion) {
        println!("minion deathrattle: {minion:?}")
    }

    fn deal_damage(&self, target: Target, damage: i64) {
        println!("deal damage: {target:?}, damage: {damage}")
    }
}

#[async_trait::async_trait]
impl PlayerBehavior for StdBehavior {
    async fn next_action(&self, game: &Game, player: &Player) -> PlayerTurnAction {
        loop {
            let turn_action_type: u8 = Input::new()
                .with_prompt("TurnAction(1: PlayCard, 2: MinionAttack, 0: EndTurn) > ")
                .interact_text()
                .expect("input turn action type");

            match turn_action_type {
                0 => return PlayerTurnAction::EndTurn,
                1 => {
                    let hand_index = Input::new()
                        .with_prompt(" hand_index > ")
                        .interact_text()
                        .expect("input hand_index");

                    let target = input_target();

                    return PlayerTurnAction::PlayCard { hand_index, target };
                }
                2 => {
                    let attacker = Input::new()
                        .with_prompt(" attacker > ")
                        .interact_text()
                        .expect("input attacker");

                    let Some(target) = input_target() else {
                        continue;
                    };

                    return PlayerTurnAction::MinionAttack { attacker, target };
                }
                _ => continue,
            }
        }
    }
}

fn input_target() -> Option<Target> {
    loop {
        let option: u8 = Input::new()
            .with_prompt(" target(0: None, 1: Minion, 2: Hero) > ")
            .interact_text()
            .expect("input target");
        match option {
            0 => return None,
            v if v == 1 || v == 2 => {
                let id: u64 = Input::new()
                    .with_prompt(" target(0: None, 1: Minion, 2: Hero) > ")
                    .interact_text()
                    .expect("input target");
                if v == 1 {
                    return Some(Target::Minion(id));
                } else {
                    return Some(Target::Hero(id));
                }
            }
            _ => continue,
        }
    }
}
