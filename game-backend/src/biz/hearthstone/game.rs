use crate::biz::hearthstone::model::*;
use crate::common::room::SafeRoom;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::Mutex;

pub struct Game {
    room: SafeRoom,
    players: HashMap<i64, SafePlayer>,
    battlefields: HashMap<Camp, Battlefield>,
    turn: u64,
    current_user_id: i64,
    game_end: bool,
}

type SafePlayer = Arc<Mutex<Player>>;

pub struct Player {
    user_id: i64,

    camp: Camp,
    figntline: Fightline,

    hero_hp: i32,
    hand_cards: Vec<Card>,
    deck_cards: Vec<Card>,
}

pub struct Battlefield {
    minions: Vec<Minion>,
}

impl Game {
    pub async fn create(safe_room: SafeRoom) -> Game {
        Game {
            room: safe_room,
            players: todo!(),
            battlefields: todo!(),
            turn: 0,
            current_user_id: todo!(),
            game_end: false,
        }
    }

    pub async fn run(&mut self) {}
}
