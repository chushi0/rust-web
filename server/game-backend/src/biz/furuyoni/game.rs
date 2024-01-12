use crate::biz::furuyoni::dal::get_random_characters;

use crate::common::room::SafeRoom;
use rand_chacha::ChaCha8Rng;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::Mutex;

pub struct Game {
    room: SafeRoom,
    players: HashMap<i64, SafePlayer>,
    turn: u64,
    current_user_id: i64,
    game_end: bool,
}

type SafePlayer = Arc<Mutex<Player>>;

#[derive(Debug)]
pub struct Player {
    user_id: i64,

    // 护甲
    shield: i8,
    // 命
    health: i8,
    // 集中力
    movement_point: i8,

    /// 角色
    character: Vec<super::data::Character>,

    /// 手牌
    hand_cards: Vec<super::data::Card>,
    /// 盖牌
    hidden_cards: Vec<super::data::Card>,
    /// 使用过的牌
    used_cards: Vec<super::data::Card>,
    /// 王牌
    ace_cards: Vec<super::data::Card>,
    /// 牌库的牌
    deck_cards: Vec<super::data::Card>,

    // 玩家状态
    status: Vec<super::data::Status>,
}

impl Player {
    fn new(user_id: i64) -> Player {
        Player {
            user_id,
            shield: 3,
            health: 10,
            movement_point: 0,
            character: vec![],
            hand_cards: vec![],
            hidden_cards: vec![],
            used_cards: vec![],
            ace_cards: vec![],
            deck_cards: vec![],
            status: vec![],
        }
    }
}

impl Game {
    pub async fn create(room: SafeRoom) -> Game {
        let game_room = room.clone();
        let game_room = game_room.lock().await;
        let mut players = HashMap::new();
        game_room.players().iter().for_each(|player| {
            players.insert(
                player.get_user_id(),
                Arc::new(Mutex::new(Player::new(player.get_user_id()))),
            );
        });

        Game {
            room,
            players,
            turn: 0,
            current_user_id: 0,
            game_end: false,
        }
    }

    pub async fn run(&mut self) {
        // 玩家选择角色、决定起始手牌，决定先后手
        self.init_players().await;
        // 主回合
        while !self.game_end {
            self.do_main_turn().await;
            // 回合计数
            self.turn += 1;
            // 切换行动角色
            self.current_user_id = self.another_user_id(self.current_user_id);
        }
    }

    async fn init_players(&mut self) {
        let mut join_handles = vec![];
        let mut room = self.room.lock().await;
        self.players.iter_mut().for_each(|(user_id, player)| {
            join_handles.push(tokio::spawn(Game::init_player(
                *user_id,
                player.clone(),
                room.new_rng(),
            )))
        });
        drop(room);
        for join_handle in join_handles {
            join_handle.await.expect("init_player should not be error");
        }

        let player_user_ids: Vec<i64> = self.players.keys().cloned().collect();
        let mut room = self.room.lock().await;
        let user_id = player_user_ids[room.random(0, 2) as usize];
        self.current_user_id = user_id;
    }

    async fn init_player(_user_id: i64, player: SafePlayer, rng: ChaCha8Rng) {
        let mut rng = rng;
        let _player = player.lock().await;
        let _characters = get_random_characters(&mut rng, 6)
            .await
            .expect("should get enough character");
        todo!("send selectable characters");
        todo!("get player select characters");
        todo!("send select character to all player");
        todo!("get player select cards");
        todo!("shuffle cards & draw cards & redraw if player want");
    }

    fn another_user_id(&self, this_user_id: i64) -> i64 {
        *self
            .players
            .iter()
            .filter(|(user_id, _player)| this_user_id != **user_id)
            .last()
            .expect("another user should in game")
            .0
    }

    async fn do_main_turn(&mut self) {
        let _player = self
            .players
            .get(&self.current_user_id)
            .expect("user should in game");
        // 准备阶段
        if self.turn >= 2 {}
        // 主阶段
        // 弃牌阶段
    }
}
