use crate::biz::hearthstone::model::*;
use crate::common::room::SafeRoom;
use datastructure::CycleArrayVector;
use rand::seq::SliceRandom;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::Mutex;

const MAX_HERO_HP: i32 = 30;

pub struct Game {
    room: SafeRoom,
    players: HashMap<i64, SafePlayer>,
    battlefields: HashMap<Camp, Battlefield>,
    turn: u64,
    current_turn_action: datastructure::CycleArrayVector<TurnAction>,
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

enum TurnAction {
    PlayerAction { uid: i64 },
    SwapFrontBack,
}

impl Game {
    pub async fn create(safe_room: SafeRoom) -> Game {
        let game_room = safe_room.clone();
        let game_room = game_room.lock().await;
        let mut players = HashMap::new();
        game_room.players().iter().for_each(|player| {
            players.insert(
                player.get_user_id(),
                Arc::new(Mutex::new(Player::new(player.get_user_id()))),
            );
        });
        let mut battlefields = HashMap::new();
        battlefields.insert(Camp::A, Battlefield::new());
        battlefields.insert(Camp::B, Battlefield::new());

        Game {
            room: safe_room,
            players,
            battlefields,
            turn: 0,
            current_turn_action: CycleArrayVector::new(vec![TurnAction::SwapFrontBack]),
            game_end: false,
        }
    }

    pub async fn run(&mut self) {
        // 全局初始化，分组、下发游戏开局信息
        self.global_init().await;
        // 玩家选择前后场，决定起始手牌
        self.init_players().await;
        // 主回合
        while !self.game_end {
            self.do_main_turn().await;
            // 回合计数
            self.turn += 1;
            // 切换行动角色
            self.current_turn_action.move_to_next();
        }
        // 游戏结束
        self.do_game_end().await;
    }

    async fn global_init(&mut self) {
        let mut rng = self.room.lock().await.new_rng();
        // 全体玩家id
        let mut player_ids: Vec<i64> = self.players.iter().map(|player| *player.0).collect();
        assert!(player_ids.len() == 4);
        // 随机排序
        player_ids.shuffle(&mut rng);
        // 前两个是A阵营，后两个是B阵营
        for i in 0..4 {
            let camp = if i < 2 { Camp::A } else { Camp::B };
            self.players
                .get(&player_ids[i])
                .expect("player should exist")
                .lock()
                .await
                .camp = camp;
        }
        // 按照 0 2 1 3 顺序行动
        let mut action_turn = Vec::with_capacity(5);
        for id in vec![0, 2, 1, 3] {
            action_turn.push(TurnAction::PlayerAction {
                uid: player_ids[id],
            });
        }
        action_turn.push(TurnAction::SwapFrontBack);
        self.current_turn_action = CycleArrayVector::new(action_turn);

        // TODO: 下发分组信息
    }

    async fn init_players(&mut self) {}

    async fn do_main_turn(&mut self) {}

    async fn do_game_end(&mut self) {}
}

impl Player {
    fn new(user_id: i64) -> Player {
        Player {
            user_id,
            camp: Camp::A,
            figntline: Fightline::Front,
            hero_hp: MAX_HERO_HP,
            hand_cards: vec![],
            deck_cards: vec![],
        }
    }
}

impl Battlefield {
    fn new() -> Battlefield {
        Battlefield { minions: vec![] }
    }
}

impl Damageable for Player {
    fn damage(&mut self, damage: i32) {
        self.hero_hp -= damage;
        if self.hero_hp > MAX_HERO_HP {
            self.hero_hp = MAX_HERO_HP;
        }
    }
}
