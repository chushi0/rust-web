use crate::biz::hearthstone::model::*;
use crate::common::input::InputManager;
use crate::common::input::InputWatcher;
use crate::common::room::SafeRoom;
use anyhow::Result;
use datastructure::CycleArrayVector;
use idl_gen::bss_hearthstone::JoinRoomExtraData;
use idl_gen::bss_hearthstone::PlayerTurnAction;
use idl_gen::bss_hearthstone::Position;
use idl_gen::bss_hearthstone::ReplacePrepareCardAction;
use idl_gen::bss_hearthstone::SelectPositionAction;
use protobuf::Message;
use rand::seq::SliceRandom;
use std::collections::HashMap;
use std::future::Future;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::mpsc;
use tokio::sync::Mutex;
use tokio::sync::MutexGuard;

const MAX_HERO_HP: i32 = 30;

pub struct Game {
    room: SafeRoom,
    input: Arc<InputManager>,
    players: HashMap<i64, SafePlayer>,
    battlefields: HashMap<Camp, Battlefield>,
    turn: u64,
    current_turn_action: datastructure::CycleArrayVector<TurnAction>,
    game_end: bool,
}

pub type SafePlayer = Arc<Mutex<Player>>;

pub struct Player {
    user_id: i64,

    camp: Camp,
    fightline: Fightline,

    hero_hp: i32,
    hand_cards: Vec<Card>,
    deck_cards: Vec<Card>,

    tired: i32,
}

pub struct Battlefield {
    minions: Vec<Mutex<Minion>>,
}

#[derive(Debug, Clone, Copy)]
enum TurnAction {
    PlayerAction { uid: i64 },
    SwapFrontBack,
}

impl Game {
    pub async fn create(safe_room: SafeRoom, input_manager: Arc<InputManager>) -> Result<Game> {
        let game_room = safe_room.clone();
        let game_room: tokio::sync::MutexGuard<'_, crate::common::room::Room> =
            game_room.lock().await;
        let mut players = HashMap::new();
        for player in game_room.players() {
            let extra_data = match player.get_extra_data() {
                Some(data) => data,
                None => return Err(anyhow::anyhow!("extra data is empty")),
            };
            players.insert(
                player.get_user_id(),
                Arc::new(Mutex::new(
                    Player::with_extra_data(player.get_user_id(), extra_data).await?,
                )),
            );
        }
        let mut battlefields = HashMap::new();
        battlefields.insert(Camp::A, Battlefield::new());
        battlefields.insert(Camp::B, Battlefield::new());

        Ok(Game {
            room: safe_room,
            input: input_manager,
            players,
            battlefields,
            turn: 0,
            current_turn_action: CycleArrayVector::new(vec![TurnAction::SwapFrontBack]),
            game_end: false,
        })
    }

    pub async fn run(mut self) {
        // 全局初始化，分组、下发游戏开局信息
        self.global_init().await;
        // 玩家选择前后场，决定起始手牌
        self.player_init().await;
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
        assert!(
            player_ids.len() == 4,
            "len of player_ids: {}",
            player_ids.len()
        );
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
        // let mut action_turn = Vec::with_capacity(5);
        // for id in vec![0, 2, 1, 3] {
        //     action_turn.push(TurnAction::PlayerAction {
        //         uid: player_ids[id],
        //     });
        // }
        // action_turn.push(TurnAction::SwapFrontBack);
        // self.current_turn_action = CycleArrayVector::new(action_turn);

        // TODO: 下发分组信息
    }

    async fn player_init(&mut self) {
        // 选择前后
        let (task_a, timeout_a) = self.init_player_select_fightline(Camp::A).await;
        let (task_b, timeout_b) = self.init_player_select_fightline(Camp::B).await;
        let select_fightline_task = tokio::spawn(async {
            tokio::join!(task_a, task_b);
        });
        tokio::time::sleep(tokio::time::Duration::from_secs(20)).await;
        timeout_a.send(()).await.expect("should be sent");
        timeout_b.send(()).await.expect("should be sent");
        select_fightline_task.await.expect("should exit normal");
        // 选择起始手牌
        let mut player_ids = vec![];
        for (id, _) in &self.players {
            player_ids.push(id.clone());
        }
        assert!(
            player_ids.len() == 4,
            "len of player_ids: {}",
            player_ids.len()
        );
        let task_a = self.init_player_start_cards(player_ids[0]).await;
        let task_b = self.init_player_start_cards(player_ids[1]).await;
        let task_c = self.init_player_start_cards(player_ids[2]).await;
        let task_d = self.init_player_start_cards(player_ids[3]).await;
        let start_cards_task = tokio::spawn(async {
            tokio::join!(task_a, task_b, task_c, task_d);
        });
        start_cards_task.await.expect("should exit normal");
    }

    async fn init_player_select_fightline(
        &mut self,
        camp: Camp,
    ) -> (impl Future<Output = ()>, mpsc::Sender<()>) {
        let (sender, mut receiver) = mpsc::channel(1);

        let mut players = vec![];
        for (player_id, safe_player) in &self.players {
            let player = safe_player.lock().await;
            if player.camp != camp {
                continue;
            }
            players.push((player_id.clone(), safe_player.clone()));
        }
        assert!(players.len() == 2, "len of players: {}", players.len());

        let input = self.input.clone();
        let mut input_watcher_1: InputWatcher<SelectPositionAction> =
            input.register_input_watcher(players[0].0).await;
        let mut input_watcher_2: InputWatcher<SelectPositionAction> =
            input.register_input_watcher(players[1].0).await;

        let mut pos = vec![Position::Undefined, Position::Undefined];
        let random_result = self.room.lock().await.random(0, 2);

        let task = async move {
            loop {
                tokio::select! {
                    _ = receiver.recv() => {
                        let first_position = if pos[0] == Position::Undefined || pos[1] == Position::Undefined || pos[0] == pos[1] {
                            if random_result == 0 {
                                Fightline::Front
                            } else {
                                Fightline::Back
                            }
                        } else if pos[0] == Position::Front {
                            Fightline::Front
                        } else {
                            Fightline::Back
                        };

                        let mut player = players[0].1.lock().await;
                        player.fightline = first_position;
                        let mut player = players[1].1.lock().await;
                        player.fightline = first_position.swap();

                        // TODO: 发送完成选择信息
                        // 结束协程
                        break;
                    },
                    Ok(input) = input_watcher_1.get_next_input() => {
                        let input = input.position.enum_value().expect("position");
                        pos[0] = input;
                        pos[1] = match input {
                            Position::Undefined => Position::Undefined,
                            Position::Front => Position::Back,
                            Position::Back => Position::Front,
                        };
                        // TODO: 发送玩家选择信息
                    },
                    Ok(input) = input_watcher_2.get_next_input() => {
                        let input = input.position.enum_value().expect("position");
                        pos[1] = input;
                        pos[0] = match input {
                            Position::Undefined => Position::Undefined,
                            Position::Front => Position::Back,
                            Position::Back => Position::Front,
                        };

                        // TODO: 发送玩家选择信息
                    }
                }
            }

            input.unregister_input_watcher(input_watcher_1).await;
            input.unregister_input_watcher(input_watcher_2).await;
        };

        (task, sender)
    }

    async fn init_player_start_cards(&self, player_id: i64) -> impl Future<Output = ()> {
        let mut rng = self.room.lock().await.new_rng();
        let player = self
            .players
            .get(&player_id)
            .expect("should has player")
            .clone();
        let input = self.input.clone();

        async move {
            let mut player = player.lock().await;
            // 刷新牌库
            player.deck_cards.shuffle(&mut rng);

            // 起始手牌
            let mut cards = player.deck_cards[..3].to_vec();
            player.deck_cards = player.deck_cards[3..].to_vec();

            let input: ReplacePrepareCardAction = input
                .wait_for_input(
                    player_id,
                    Duration::from_secs(20),
                    || ReplacePrepareCardAction::default(),
                    Some(|| {
                        // TODO: 发送起始手牌
                        ()
                    }),
                )
                .await;

            // 换牌
            let mut index = 0;
            for i in input.card_index {
                (player.deck_cards[index], cards[i as usize]) =
                    (cards[i as usize].clone(), player.deck_cards[index].clone());
                index += 1;
            }

            player.hand_cards = cards;
        }
    }

    async fn do_main_turn(&mut self) {
        match *self.current_turn_action {
            TurnAction::PlayerAction { uid } => self.do_player_turn(uid).await,
            TurnAction::SwapFrontBack => self.do_swap_front_back_turn().await,
        }
    }

    async fn do_player_turn(&mut self, uid: i64) {
        // 获取当前玩家
        let player = self.players.get(&uid).expect("should exist").clone();
        let mut player = player.lock().await;
        // 抽牌
        player.draw_card(1).await;
        // 注册输入
        let mut input: InputWatcher<PlayerTurnAction> =
            self.input.register_input_watcher(uid).await;
        // 循环获取输入，处理回合事件
        // TODO: 超时
        while let Ok(action) = input.get_next_input().await {
            match action.action_type {};
        }
        // 回合结束
    }

    async fn do_swap_front_back_turn(&mut self) {
        // TODO: 交换玩家前后排
        // TODO: 扳机
    }

    async fn do_game_end(&mut self) {}
}

impl Game {
    pub async fn get_minion(&self, camp: &Camp, minion_id: u64) -> Option<MutexGuard<Minion>> {
        let Some(battlefield) = self.battlefields.get(camp) else {
            return None;
        };
        for minion in &battlefield.minions {
            let minion = minion.lock().await;
            if minion.id() == minion_id {
                return Some(minion);
            }
        }
        None
    }

    pub async fn get_minions(&self, camp: &Camp) -> Vec<MutexGuard<Minion>> {
        let mut result = vec![];
        let Some(battlefield) = self.battlefields.get(camp) else {
            return result;
        };
        for minion in &battlefield.minions {
            let minion = minion.lock().await;
            result.push(minion);
        }
        result
    }

    pub async fn get_all_minions(&self) -> Vec<MutexGuard<Minion>> {
        let mut result = vec![];
        for (_, battlefield) in &self.battlefields {
            for minion in &battlefield.minions {
                let minion = minion.lock().await;
                result.push(minion);
            }
        }
        result
    }

    pub async fn get_player(&self, uid: i64) -> Option<MutexGuard<Player>> {
        Some(self.players.get(&uid)?.lock().await)
    }

    pub async fn get_player_by_camp_pos(
        &self,
        camp: &Camp,
        fightline: Fightline,
    ) -> Option<MutexGuard<Player>> {
        for (_, player) in &self.players {
            let player = player.lock().await;
            if player.camp == *camp && player.fightline == fightline {
                return Some(player);
            }
        }
        None
    }

    pub async fn get_player_by_pos(&self, fightline: Fightline) -> Option<MutexGuard<Player>> {
        for (_, player) in &self.players {
            let player = player.lock().await;
            if player.fightline == fightline {
                return Some(player);
            }
        }
        None
    }

    pub async fn get_player_by_camp(&self, camp: &Camp) -> Vec<MutexGuard<Player>> {
        let mut result = vec![];
        for (_, player) in &self.players {
            let player = player.lock().await;
            if player.camp == *camp {
                result.push(player);
            }
        }
        result
    }

    pub async fn get_all_players(&self) -> Vec<MutexGuard<Player>> {
        let mut result = vec![];
        for (_, player) in &self.players {
            result.push(player.lock().await);
        }
        result
    }
}

impl Player {
    async fn with_extra_data(user_id: i64, extra_data: &Vec<u8>) -> Result<Player> {
        let data = JoinRoomExtraData::parse_from_bytes(extra_data)?;

        let mut deck_cards = Vec::with_capacity(data.card_code.len());
        for code in data.card_code {
            deck_cards.push(Card::from_cache(code).await?)
        }

        Ok(Player {
            user_id,
            camp: Camp::A,
            fightline: Fightline::Front,
            hero_hp: MAX_HERO_HP,
            hand_cards: vec![],
            deck_cards,
            tired: 0,
        })
    }

    pub async fn draw_card(&mut self, c: i32) {
        for _ in 0..c {
            self.draw_card_internal();
        }
        // TODO: 通知玩家
    }

    fn draw_card_internal(&mut self) {
        if self.deck_cards.is_empty() {
            self.tired += 1;
            self.damage(self.tired);
            return;
        }
        self.hand_cards.push(self.deck_cards[0].clone());
        self.deck_cards.remove(0);
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
