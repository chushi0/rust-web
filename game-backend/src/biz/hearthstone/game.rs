use crate::biz::hearthstone::model::*;
use crate::common::input::InputManager;
use crate::common::input::InputWatcher;
use crate::common::room::SafeRoom;
use anyhow::Result;
use datastructure::CycleArrayVector;
use idl_gen::bss_hearthstone::JoinRoomExtraData;
use idl_gen::bss_hearthstone::PlayerTurnAction;
use idl_gen::bss_hearthstone::PlayerTurnActionEnum;
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
use web_db::hearthstone::CardType;

use super::card_action_interpreter::EffectTarget;
use super::card_action_interpreter::EventType;
use super::card_action_interpreter::Interpreter;

const MAX_HERO_HP: i32 = 30;

pub struct GameLogic {
    input: Arc<InputManager>,
    game: Mutex<Game>,
}

// impl GameLogic {
//     pub async fn create(safe_room: SafeRoom, input_manager: Arc<InputManager>) -> Result<Game> {
//         let game_room = safe_room.clone();
//         let game_room: tokio::sync::MutexGuard<'_, crate::common::room::Room> =
//             game_room.lock().await;
//         let mut players = HashMap::new();
//         for player in game_room.players() {
//             let extra_data = match player.get_extra_data() {
//                 Some(data) => data,
//                 None => return Err(anyhow::anyhow!("extra data is empty")),
//             };
//             players.insert(
//                 player.get_user_id(),
//                 Arc::new(Mutex::new(
//                     Player::with_extra_data(player.get_user_id(), extra_data).await?,
//                 )),
//             );
//         }
//         let mut battlefields = HashMap::new();
//         battlefields.insert(Camp::A, Battlefield::new());
//         battlefields.insert(Camp::B, Battlefield::new());

//         Ok(Game {
//             room: safe_room,
//             input: input_manager,
//             players,
//             battlefields,
//             turn: 0,
//             current_turn_action: CycleArrayVector::new(vec![TurnAction::SwapFrontBack]),
//             game_end: false,
//             global_id: 0,
//         })
//     }

//     pub async fn run(mut self) {
//         log::debug!("start game");
//         // 全局初始化，分组、下发游戏开局信息
//         self.global_init().await;
//         // 玩家选择前后场，决定起始手牌
//         self.player_init().await;
//         // 初始化最后阶段：决定回合顺序
//         self.final_init().await;
//         // 主回合
//         while !self.game_end {
//             self.do_main_turn().await;
//             // 回合计数
//             self.turn += 1;
//             // 切换行动角色
//             self.current_turn_action.move_to_next();
//         }
//         // 游戏结束
//         self.do_game_end().await;
//         log::debug!("end game");
//     }

//     async fn global_init(&mut self) {
//         let mut rng = self.room.lock().await.new_rng();
//         // 全体玩家id
//         let mut player_ids: Vec<i64> = self.players.iter().map(|player| *player.0).collect();
//         assert!(
//             player_ids.len() == 4,
//             "len of player_ids: {}",
//             player_ids.len()
//         );
//         // 随机排序
//         player_ids.shuffle(&mut rng);
//         // 前两个是A阵营，后两个是B阵营
//         for i in 0..4 {
//             let camp = if i < 2 { Camp::A } else { Camp::B };
//             self.players
//                 .get(&player_ids[i])
//                 .expect("player should exist")
//                 .lock()
//                 .await
//                 .camp = camp;
//         }

//         // TODO: 下发分组信息

//         log::debug!("global init result: {:?}", &self.players)
//     }

//     async fn player_init(&mut self) {
//         // 选择前后
//         let (task_a, timeout_a) = self.init_player_select_fightline(Camp::A).await;
//         let (task_b, timeout_b) = self.init_player_select_fightline(Camp::B).await;
//         let select_fightline_task = tokio::spawn(async {
//             tokio::join!(task_a, task_b);
//         });
//         tokio::time::sleep(tokio::time::Duration::from_secs(20)).await;
//         timeout_a.send(()).await.expect("should be sent");
//         timeout_b.send(()).await.expect("should be sent");
//         select_fightline_task.await.expect("should exit normal");
//         // 选择起始手牌
//         let mut player_ids = vec![];
//         for (id, _) in &self.players {
//             player_ids.push(id.clone());
//         }
//         assert!(
//             player_ids.len() == 4,
//             "len of player_ids: {}",
//             player_ids.len()
//         );
//         let task_a = self.init_player_start_cards(player_ids[0]).await;
//         let task_b = self.init_player_start_cards(player_ids[1]).await;
//         let task_c = self.init_player_start_cards(player_ids[2]).await;
//         let task_d = self.init_player_start_cards(player_ids[3]).await;
//         let start_cards_task = tokio::spawn(async {
//             tokio::join!(task_a, task_b, task_c, task_d);
//         });
//         start_cards_task.await.expect("should exit normal");

//         log::debug!("player init result: {:?}", &self.players);
//     }

//     async fn init_player_select_fightline(
//         &mut self,
//         camp: Camp,
//     ) -> (impl Future<Output = ()>, mpsc::Sender<()>) {
//         let (sender, mut receiver) = mpsc::channel(1);

//         let mut players = vec![];
//         for (player_id, safe_player) in &self.players {
//             let player = safe_player.lock().await;
//             if player.camp != camp {
//                 continue;
//             }
//             players.push((player_id.clone(), safe_player.clone()));
//         }
//         assert!(players.len() == 2, "len of players: {}", players.len());

//         let input = self.input.clone();
//         let mut input_watcher_1: InputWatcher<SelectPositionAction> =
//             input.register_input_watcher(players[0].0).await;
//         let mut input_watcher_2: InputWatcher<SelectPositionAction> =
//             input.register_input_watcher(players[1].0).await;

//         let mut pos = vec![Position::Undefined, Position::Undefined];
//         let random_result = self.room.lock().await.random(0, 2);

//         let task = async move {
//             loop {
//                 tokio::select! {
//                     _ = receiver.recv() => {
//                         let first_position = if pos[0] == Position::Undefined || pos[1] == Position::Undefined || pos[0] == pos[1] {
//                             if random_result == 0 {
//                                 Fightline::Front
//                             } else {
//                                 Fightline::Back
//                             }
//                         } else if pos[0] == Position::Front {
//                             Fightline::Front
//                         } else {
//                             Fightline::Back
//                         };

//                         let mut player = players[0].1.lock().await;
//                         player.fightline = first_position;
//                         let mut player = players[1].1.lock().await;
//                         player.fightline = first_position.swap();

//                         // TODO: 发送完成选择信息
//                         // 结束协程
//                         break;
//                     },
//                     Ok(input) = input_watcher_1.get_next_input() => {
//                         let input = input.position.enum_value().expect("position");
//                         pos[0] = input;
//                         pos[1] = match input {
//                             Position::Undefined => Position::Undefined,
//                             Position::Front => Position::Back,
//                             Position::Back => Position::Front,
//                         };
//                         // TODO: 发送玩家选择信息
//                     },
//                     Ok(input) = input_watcher_2.get_next_input() => {
//                         let input = input.position.enum_value().expect("position");
//                         pos[1] = input;
//                         pos[0] = match input {
//                             Position::Undefined => Position::Undefined,
//                             Position::Front => Position::Back,
//                             Position::Back => Position::Front,
//                         };

//                         // TODO: 发送玩家选择信息
//                     }
//                 }
//             }

//             input.unregister_input_watcher(input_watcher_1).await;
//             input.unregister_input_watcher(input_watcher_2).await;
//         };

//         (task, sender)
//     }

//     async fn init_player_start_cards(&self, player_id: i64) -> impl Future<Output = ()> {
//         let mut rng = self.room.lock().await.new_rng();
//         let player = self
//             .players
//             .get(&player_id)
//             .expect("should has player")
//             .clone();
//         let input = self.input.clone();

//         async move {
//             let mut player = player.lock().await;
//             // 刷新牌库
//             player.deck_cards.shuffle(&mut rng);

//             // 起始手牌
//             let mut cards = player.deck_cards[..3].to_vec();
//             player.deck_cards = player.deck_cards[3..].to_vec();

//             let input: ReplacePrepareCardAction = input
//                 .wait_for_input(
//                     player_id,
//                     Duration::from_secs(20),
//                     || ReplacePrepareCardAction::default(),
//                     Some(|| {
//                         // TODO: 发送起始手牌
//                         ()
//                     }),
//                 )
//                 .await;

//             // 换牌
//             let mut index = 0;
//             for i in input.card_index {
//                 (player.deck_cards[index], cards[i as usize]) =
//                     (cards[i as usize].clone(), player.deck_cards[index].clone());
//                 index += 1;
//             }

//             player.hand_cards = cards;
//         }
//     }

//     async fn final_init(&mut self) {
//         let mut player_ids = vec![0; 4];
//         for (uid, player) in &self.players {
//             let player = player.lock().await;
//             let mut index = 0;
//             if let Camp::B = player.camp {
//                 index &= 1;
//             }
//             if let Fightline::Back = player.fightline {
//                 index &= 2;
//             }
//             player_ids[index] = *uid;
//         }

//         let mut action_turn = Vec::with_capacity(5);
//         for i in 0..4 {
//             action_turn.push(TurnAction::PlayerAction { uid: player_ids[i] });
//         }
//         action_turn.push(TurnAction::SwapFrontBack);
//         self.current_turn_action = CycleArrayVector::new(action_turn);
//     }

//     async fn do_main_turn(&mut self) {
//         match *self.current_turn_action {
//             TurnAction::PlayerAction { uid } => self.do_player_turn(uid).await,
//             TurnAction::SwapFrontBack => self.do_swap_front_back_turn().await,
//         }
//     }

//     async fn do_player_turn(&mut self, uid: i64) {
//         // 获取当前玩家
//         let safe_player = self.players.get(&uid).expect("should exist").clone();
//         let mut player = safe_player.lock().await;
//         // 法力水晶
//         if player.maxmana < 10 {
//             player.maxmana += 1;
//         }
//         player.mana = player.maxmana;
//         // 抽牌
//         player.draw_card(1).await;
//         // 为避免后续卡牌效果执行时出现问题，暂时释放player的锁
//         drop(player);
//         // 注册输入
//         let mut input: InputWatcher<PlayerTurnAction> =
//             self.input.register_input_watcher(uid).await;
//         // 循环获取输入，处理回合事件
//         // TODO: 超时
//         while let Ok(action) = input.get_next_input().await {
//             match action.action_type.unwrap() {
//                 PlayerTurnActionEnum::PlayerEndTurn => break,
//                 PlayerTurnActionEnum::PlayerUseCard => {
//                     let info = action.player_use_card;
//                     let mut player = safe_player.lock().await;
//                     let Some(_card) = player.hand_cards.get(info.card_index as usize) else {
//                         continue;
//                     };
//                     let card = player.hand_cards.remove(info.card_index as usize);
//                     player.mana -= card.get_mana_cost();
//                     let camp = player.camp;
//                     let fightline = player.fightline;
//                     let uid = player.user_id;
//                     drop(player);

//                     let card_model = card.get_model().card.clone();

//                     // TODO: 死亡结算、交换前后排
//                     match card_model.card_type.try_into() {
//                         Ok(CardType::Minion) => {
//                             // 生成随从
//                             let minion_id = self.spawn_minion(camp, &card).await;

//                             let mut interpreter = Interpreter::new(
//                                 self,
//                                 EffectTarget::Minion { camp, minion_id },
//                                 None,
//                                 card,
//                             );
//                             // 战吼效果
//                             interpreter.perform(EventType::Battlecry, None).await;
//                         }
//                         Ok(CardType::Spell) => {
//                             let mut interpreter = Interpreter::new(
//                                 self,
//                                 EffectTarget::Hero { camp, uid },
//                                 None,
//                                 card,
//                             );
//                             let normal_spell;
//                             // 法术前后排效果
//                             match fightline {
//                                 Fightline::Front => {
//                                     let result =
//                                         interpreter.perform(EventType::FrontUse, None).await;
//                                     normal_spell = !result.prevent_normal_effect;
//                                 }
//                                 Fightline::Back => {
//                                     let result =
//                                         interpreter.perform(EventType::BackUse, None).await;
//                                     normal_spell = !result.prevent_normal_effect;
//                                 }
//                             }
//                             // 通常法术效果
//                             if normal_spell {
//                                 interpreter.perform(EventType::NormalSpell, None).await;
//                             }
//                         }
//                         Err(_) => {
//                             log::error!(
//                                 "card type error: {} (card_id: {})",
//                                 card_model.card_type,
//                                 card_model.rowid
//                             )
//                         }
//                     }
//                 }
//                 PlayerTurnActionEnum::PlayerOperateMinion => {}
//             };
//         }
//         // 回合结束
//     }

//     async fn spawn_minion(&mut self, camp: Camp, card: &Card) -> u64 {
//         let id = self.next_id();
//         let minion = Minion::new(id, card);
//         let battlefield = self
//             .battlefields
//             .get_mut(&camp)
//             .expect("battlefield should exist");
//         battlefield.minions.push(Mutex::new(minion));

//         id
//     }

//     fn next_id(&mut self) -> u64 {
//         let id = self.global_id;
//         self.global_id += 1;
//         id
//     }

//     async fn do_swap_front_back_turn(&mut self) {
//         // TODO: 交换玩家前后排
//         // TODO: 扳机
//     }

//     async fn do_game_end(&mut self) {}
// }

// impl GameLogic {
//     async fn with_extra_data(user_id: i64, extra_data: &Vec<u8>) -> Result<Player> {
//         let data = JoinRoomExtraData::parse_from_bytes(extra_data)?;

//         let mut deck_cards = Vec::with_capacity(data.card_code.len());
//         for code in data.card_code {
//             deck_cards.push(Card::from_cache(code).await?)
//         }

//         Ok(Player {
//             user_id,
//             camp: Camp::A,
//             fightline: Fightline::Front,
//             hero_hp: MAX_HERO_HP,
//             hand_cards: vec![],
//             deck_cards,
//             mana: 0,
//             maxmana: 0,
//             tired: 0,
//         })
//     }
// }
