use clap::Parser;
use dialoguer::Input;
use heartstone::model::{Camp, CardModel};
use idl_gen::bss_heartstone::{
    BuffEvent, Card, DamageEvent, DrawCardEvent, MinionAttackEvent, MinionEffectEvent,
    MinionEnterEvent, MinionRemoveEvent, NewTurnEvent, PlayerEndTurnAction, PlayerManaChange,
    PlayerOperateMinionAction, PlayerTurnAction, PlayerTurnActionEnum, PlayerUseCardAction,
    PlayerUseCardEndEvent, PlayerUseCardEvent, Position, SwapFrontBackEvent, SyncGameStatus,
    Target, TurnTypeEnum,
};
use protobuf::{EnumOrUnknown, MessageField};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

#[cfg(feature = "offline")]
mod offline;
#[cfg(feature = "online")]
mod online;

#[cfg(not(any(feature = "offline", feature = "online")))]
compile_error!(
    "heartstone-cli should built with offline feature or online feature (at least one feature on!)"
);

#[derive(Debug, Parser)]
enum Args {
    /// 本地离线运行，使用本地数据库，无AI，可以看到所有信息
    #[cfg(feature = "offline")]
    Offline,
    /// 联机运行，使用服务端数据库，需要账号，模拟真实玩家对战环境
    ///
    /// 默认连接本地客户端，需要在本地搭建游戏服务器
    #[cfg(feature = "online")]
    Online {
        /// Websocket服务端地址
        #[arg(short, long, default_value = "ws://127.0.0.1:3000")]
        ws_server_ip: String,
        /// http服务端地址
        #[arg(short = 's', long, default_value = "http://127.0.0.1:8080")]
        http_server_ip: String,
        /// 账号
        #[arg(short, long)]
        account: String,
        /// 密码
        #[arg(short, long)]
        password: String,
        /// 房间号
        ///
        /// 有效房间号范围为 100000 ~ 999999.
        /// 0表示创建房间，-1表示以匹配房间方式加入
        #[arg(short, long, default_value = "-1")]
        room_id: i32,
    },
}

#[derive(Debug, Default)]
pub struct StdInAndOut {
    cards: HashMap<String, Arc<CardModel>>,
}

lazy_static::lazy_static! {
    static ref STD_IN_AND_OUT: Mutex<StdInAndOut> = Mutex::new(StdInAndOut::default());
}

#[tokio::main]
async fn main() {
    let args = Args::parse();

    let client: Box<dyn Client> = match args {
        #[cfg(feature = "offline")]
        Args::Offline => Box::new(offline::Client),
        #[cfg(feature = "online")]
        Args::Online {
            ws_server_ip,
            http_server_ip,
            account,
            password,
            room_id,
        } => Box::new(online::Client {
            ws_server_ip,
            http_server_ip,
            account,
            password,
            room_id,
        }),
    };

    client.run().await;

    let s: String = Input::new()
        .with_prompt("输入任意内容退出")
        .interact_text()
        .expect("input anything to exit");
    _ = s;
}

pub fn io() -> impl std::ops::DerefMut<Target = StdInAndOut> {
    STD_IN_AND_OUT.lock().unwrap()
}

#[async_trait::async_trait]
pub trait Client {
    async fn run(&self);
}

impl StdInAndOut {
    pub fn cache_cards(&mut self, cards: &[Arc<CardModel>]) {
        for card in cards {
            self.cards.insert(card.card.code.clone(), card.clone());
        }
    }

    pub fn next_action(&self) -> PlayerTurnAction {
        loop {
            let turn_action_type: u8 = Input::new()
                .with_prompt("输入回合指令(1: 使用卡牌, 2: 随从攻击, 0: 结束回合) > ")
                .interact_text()
                .expect("input turn action type");

            match turn_action_type {
                0 => {
                    return PlayerTurnAction {
                        action_type: EnumOrUnknown::new(PlayerTurnActionEnum::PlayerEndTurn),
                        player_end_turn: MessageField::some(PlayerEndTurnAction::default()),
                        ..Default::default()
                    }
                }
                1 => {
                    let hand_index = Input::new()
                        .with_prompt(" 输入牌序号(从0开始) > ")
                        .interact_text()
                        .expect("input hand_index");

                    let target = Self::input_target();

                    return PlayerTurnAction {
                        action_type: EnumOrUnknown::new(PlayerTurnActionEnum::PlayerUseCard),
                        player_use_card: MessageField::some(PlayerUseCardAction {
                            card_index: hand_index,
                            target: target.into(),
                            ..Default::default()
                        }),
                        ..Default::default()
                    };
                }
                2 => {
                    let attacker = Input::new()
                        .with_prompt(" 输入攻击随从ID > ")
                        .interact_text()
                        .expect("input attacker");

                    let Some(target) = Self::input_target() else {
                        continue;
                    };

                    return PlayerTurnAction {
                        action_type: EnumOrUnknown::new(PlayerTurnActionEnum::PlayerOperateMinion),
                        player_operate_minion: MessageField::some(PlayerOperateMinionAction {
                            minion_id: attacker,
                            target: MessageField::some(target),
                            ..Default::default()
                        }),
                        ..Default::default()
                    };
                }
                _ => continue,
            }
        }
    }

    fn input_target() -> Option<Target> {
        loop {
            let option: u8 = Input::new()
                .with_prompt(" 输入目标类型(0: 无, 1: 随从, 2: 英雄) > ")
                .interact_text()
                .expect("input target");
            match option {
                0 => return None,
                v if v == 1 || v == 2 => {
                    let id: u64 = Input::new()
                        .with_prompt(" 输入目标ID > ")
                        .interact_text()
                        .expect("input target uuid");
                    if v == 1 {
                        return Some(Target {
                            minion_id: Some(id),
                            ..Default::default()
                        });
                    } else {
                        return Some(Target {
                            player: Some(id),
                            ..Default::default()
                        });
                    }
                }
                _ => continue,
            }
        }
    }

    pub fn print_game_status(&self, game: SyncGameStatus) {
        println!("========================================");
        println!("当前游戏状态：");
        println!("----------------------------------------");
        for player in game.player_status {
            println!("玩家 [{}]：", player.uuid);
            println!("  法力值：{}", player.mana);
            println!("  生命值：{}", player.hp);
            println!("  阵营：{}", Self::camp_desc(player.camp));
            println!("  位置：{}", Self::fightline_desc(player.position));
            println!("  手牌：剩余 {} 张", player.card_count);
            player
                .cards
                .into_iter()
                .enumerate()
                .for_each(|(index, card)| println!("    #{index} {}", self.get_card_info(&card)));
        }
        println!("----------------------------------------");

        for camp in [Camp::A as i32, Camp::B as i32] {
            println!("阵营 {} 随从：", Self::camp_desc(camp));
            game.minion_status
                .iter()
                .filter(|minion| minion.camp == camp)
                .for_each(|minion| {
                    println!(
                        "  #{} {} {}/{}",
                        minion.uuid,
                        self.get_card_info(minion.card.as_ref().unwrap()),
                        minion.atk,
                        minion.hp
                    );
                });
        }

        println!("========================================");
    }

    pub fn print_new_turn(&self, event: NewTurnEvent) {
        match event.turn_type.enum_value().unwrap() {
            TurnTypeEnum::PlayerTurn => {
                println!(
                    "#################### 玩家 [{}] 的回合 ####################",
                    event.player_turn.unwrap().player_uuid
                )
            }
            TurnTypeEnum::SwapTurn => {
                println!("#################### 交换前后排回合 ####################")
            }
        }
    }

    pub fn print_player_mana_change(&self, event: PlayerManaChange) {
        println!("玩家 [{}] 的法力值变为 {}", event.player_uuid, event.mana);
    }

    pub fn print_player_draw_card(&self, event: DrawCardEvent) {
        match event.draw_card_result.enum_value().unwrap() {
            idl_gen::bss_heartstone::DrawCardResult::Ok => match event.card.0 {
                Some(card) => println!(
                    "玩家 [{}] 抽到了 {}",
                    event.player_uuid,
                    self.get_card_info(card.as_ref())
                ),
                None => println!("玩家 [{}] 抽了 1 张牌", event.player_uuid),
            },
            idl_gen::bss_heartstone::DrawCardResult::Fire => println!(
                "玩家 [{}] 抽到了 {}，但因为手牌已满，这张牌爆掉了",
                event.player_uuid,
                self.get_card_info(event.card.as_ref().unwrap())
            ),
            idl_gen::bss_heartstone::DrawCardResult::Tired => {
                println!(
                    "玩家 [{}] 抽牌，但因为牌库已空，受到了 {} 点疲劳伤害",
                    event.player_uuid,
                    event.tired.unwrap()
                )
            }
        }
    }

    pub fn print_player_use_card(&self, event: PlayerUseCardEvent) {
        println!(
            "玩家 [{}] 消耗 {} 点法力值，使用了 [{}]",
            event.player_uuid,
            event.cost_mana,
            self.get_card_info(event.card.as_ref().unwrap())
        )
    }

    pub fn print_player_card_effect_end(&self, _event: PlayerUseCardEndEvent) {}

    pub fn print_player_swap_fightline(&self, event: SwapFrontBackEvent) {
        println!(
            "玩家 [{}] 交换了前后排，当前位置为 {}",
            event.player_uuid,
            Self::fightline_desc(event.new_position)
        )
    }

    pub fn print_minion_enter(&self, event: MinionEnterEvent) {
        println!(
            "随从 [{}] 加入了阵营 [{}]，身材为 {}/{}，随从ID为 [{}]",
            self.get_card_info(event.card.as_ref().unwrap()),
            Self::camp_desc(event.group),
            event.atk,
            event.hp,
            event.minion_id
        )
    }

    pub fn print_minion_effect(&self, event: MinionEffectEvent) {
        match event.minion_effect.enum_value().unwrap() {
            idl_gen::bss_heartstone::MinionEffect::Other => unimplemented!(),
            idl_gen::bss_heartstone::MinionEffect::Battlecry => {
                println!("随从 [{}] 战吼效果发动", event.minion_id)
            }
            idl_gen::bss_heartstone::MinionEffect::Deathrattle => {
                println!("随从 [{}] 亡语效果发动", event.minion_id)
            }
        }
    }

    pub fn print_minion_attack(&self, event: MinionAttackEvent) {
        let target = event.target.unwrap();
        match (target.minion_id, target.player) {
            (Some(id), None) => println!("随从 [{}] 攻击了 随从 [{}]", event.minion_id, id),
            (None, Some(id)) => println!("随从 [{}] 攻击了 英雄 [{}]", event.minion_id, id),
            _ => panic!("invalid target"),
        }
    }

    pub fn print_minion_remove(&self, event: MinionRemoveEvent) {
        println!("随从 [{}] 死亡", event.minion_id)
    }

    pub fn print_deal_damage(&self, event: DamageEvent) {
        let target = event.target.unwrap();
        match (event.damage.cmp(&0), target.minion_id, target.player) {
            (std::cmp::Ordering::Less, Some(target_minion), None) => {
                println!("随从 [{}] 受到治疗生命值 +{}", target_minion, -event.damage)
            }
            (std::cmp::Ordering::Less, None, Some(target_hero)) => {
                println!("英雄 [{}] 受到治疗生命值 +{}", target_hero, -event.damage)
            }
            (std::cmp::Ordering::Greater, Some(target_minion), None) => {
                println!(
                    "随从 [{}] 受到伤害，生命值 -{}",
                    target_minion, event.damage
                )
            }
            (std::cmp::Ordering::Greater, None, Some(target_hero)) => {
                println!("英雄 [{}] 受到伤害生命值 -{}", target_hero, event.damage)
            }
            _ => unreachable!(),
        }
    }

    pub fn print_buff(&self, event: BuffEvent) {
        let target = event.target.unwrap();
        match (target.minion_id, target.player) {
            (None, Some(target_hero)) => println!(
                "英雄 [{}] 获得 Buff {}/{}",
                target_hero,
                Self::has_sign_num(event.buff.atk_boost),
                Self::has_sign_num(event.buff.hp_boost)
            ),
            (Some(target_minion), None) => println!(
                "随从 [{}] 获得 Buff {}/{}",
                target_minion,
                Self::has_sign_num(event.buff.atk_boost),
                Self::has_sign_num(event.buff.hp_boost)
            ),
            _ => panic!("invalid target"),
        }
    }

    fn get_card_info(&self, card: &Card) -> String {
        self.cards
            .get(&card.card_code)
            .map(|model| model.card.name.to_string())
            .unwrap_or("<Unknown>".to_string())
    }

    fn fightline_desc(fightline: EnumOrUnknown<Position>) -> &'static str {
        match fightline.enum_value() {
            Ok(Position::Front) => "前排",
            Ok(Position::Back) => "后排",
            _ => "未知",
        }
    }

    fn camp_desc(camp: i32) -> &'static str {
        match camp {
            1 => "A",
            2 => "B",
            _ => "未知",
        }
    }

    fn has_sign_num(n: i32) -> String {
        if n < 0 {
            format!("{n}")
        } else {
            format!("+{n}")
        }
    }
}
