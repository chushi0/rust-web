use dialoguer::Input;
use heartstone::{
    api::{GameNotifier, PlayerDrawCard, TurnAction},
    game::{Config, Game, PlayerConfig},
    model::{
        Buff, Camp, Card, CardModel, Damageable, Fightline, HeroTrait, Minion, MinionTrait, Target,
    },
    player::{Player, PlayerBehavior, PlayerTrait, PlayerTurnAction},
};
use std::{collections::HashMap, sync::Arc, time::Duration};

#[derive(Debug)]
struct StdBehavior;

#[derive(Debug)]
struct StdNotifier;

#[tokio::main]
async fn main() {
    println!("炉石 2V2 SNAPSHOT - 20231211");
    println!("已完成基本功能开发");
    println!("卡牌已录入数据库");
    println!("测试说明：");
    println!("  - 所有卡牌（非衍生牌）各一张 组成套牌");
    println!("  - 可代表任意玩家进行行动");
    println!("  - 可以看到所有玩家的手牌，但不能看到牌库");
    println!("  - 仅限单机，无法联机，没有 AI");
    println!("  - 没有输入检查，这意味着您可以透支法力值（无需还款），或选择一个无效的目标");
    println!("已修复：");
    println!("  - 游戏详情没有展示随从uuid");
    println!("  - 随从死亡没有通知");
    println!("  - 释放法术后没有死亡检查");
    println!("  - 亡语效果无效");
    println!("  - 指定自身英雄的战吼效果无效");
    println!("待开发功能：");
    println!("  - 游戏开始时，选择前后排及起始手牌逻辑");
    println!("  - 法术伤害+X 标记");
    println!("  - 随从狂战标记");
    println!("  - 指定随从出现位置");
    println!("您可以更改数据库 （heartstone.db）中的卡牌配置来自定义卡牌，具体修改方式见项目 OnePage 文档");
    println!();
    println!("** 静待 5 秒后，游戏将自动开始 **");
    println!();
    println!();

    std::thread::sleep(Duration::from_secs(5));

    // 加载数据库
    let card_pool = load_card_pool().await;

    let config = Config {
        game_notifier: Arc::new(StdNotifier),
        card_pool: card_pool.clone(),
        ..Default::default()
    };

    let mut players = Vec::new();
    for i in 0..4 {
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

    let s: String = Input::new()
        .with_prompt("输入任意内容退出")
        .interact_text()
        .expect("input anything to exit");
    _ = s;
}

async fn load_card_pool() -> HashMap<i64, Arc<CardModel>> {
    let mut db = web_db::create_connection_with_path("heartstone.db")
        .await
        .unwrap();
    let mut tx = web_db::begin_tx(&mut db).await.unwrap();

    let cards = web_db::hearthstone::get_all_cards(&mut tx).await.unwrap();

    let mut map = HashMap::new();
    for card in cards {
        let card_info = serde_json::from_str(&card.card_info).unwrap();
        map.insert(card.rowid, Arc::new(CardModel { card, card_info }));
    }

    map
}

#[async_trait::async_trait]
#[allow(unused_variables)]
impl GameNotifier for StdNotifier {
    async fn flush(&self, game: &Game) {
        println!("========================================");
        println!("当前游戏状态：");
        println!("----------------------------------------");
        for player in game.players() {
            println!("玩家 [{}]：", player.uuid().await);
            println!("  法力值：{}", player.mana().await);
            println!("  生命值：{}", player.get_hero().await.hp().await);
            println!("  阵营：{}", camp_desc(player.camp().await));
            println!(
                "  位置：{}",
                fightline_desc(player.get_hero().await.fightline().await)
            );
            println!("  手牌：");
            let mut index = 0;
            for card in player.hand_cards().await {
                let model = card.get().await.model().clone();
                println!("    #{index} {}", model.card.name);
                index += 1;
            }
        }
        println!("----------------------------------------");
        for camp in [Camp::A, Camp::B] {
            println!("阵营 {} 随从：", camp_desc(camp));
            let minions = game.battlefield_minions(camp).await;
            for minion in minions {
                let model = minion.get().await.model().clone();
                println!(
                    "  #{} {} {}/{}",
                    minion.uuid().await,
                    model.card.name,
                    minion.atk().await,
                    minion.hp().await
                );
            }
        }

        println!("========================================");
    }

    fn new_turn(&self, current_turn: TurnAction) {
        match current_turn {
            TurnAction::PlayerTurn(player) => {
                println!("#################### 玩家 [{player}] 的回合 ####################")
            }
            TurnAction::SwapFightline => {
                println!("#################### 交换前后排回合 ####################")
            }
        }
        std::thread::sleep(Duration::from_secs(1));
    }

    fn player_mana_change(&self, player: u64, mana: i32) {
        println!("玩家 [{player}] 的法力值变为 {mana}")
    }
    fn player_draw_card(&self, player: u64, card: PlayerDrawCard) {
        match card {
            PlayerDrawCard::Draw(card) => {
                println!("玩家 [{player}] 抽到了 {}", card.model().card.name)
            }
            PlayerDrawCard::Fire(card) => {
                println!(
                    "玩家 [{player}] 抽到了 {}，但因为手牌已满，这张牌爆掉了",
                    card.model().card.name
                )
            }
            PlayerDrawCard::Tired(tried) => {
                println!("玩家 [{player}] 抽牌，但因为牌库已空，受到了 {tried} 点疲劳伤害");
            }
        }
    }
    fn player_use_card(&self, player: u64, card: Card, cost_mana: i32) {
        println!(
            "玩家 [{player}] 消耗 {cost_mana} 点法力值，使用了 [{}]",
            card.model().card.name
        )
    }
    fn player_card_effect_end(&self) {}
    fn player_swap_fightline(&self, player: u64, new_fightline: Fightline) {
        println!(
            "玩家 [{player}] 交换了前后排，当前位置为 {}",
            fightline_desc(new_fightline)
        )
    }

    fn minion_summon(&self, minion: Minion, camp: Camp) {
        println!(
            "随从 [{}] 加入了阵营 [{}]，身材为 {}/{}，随从ID为 [{}]",
            minion.model().card.name,
            camp_desc(camp),
            minion.atk(),
            minion.hp(),
            minion.uuid()
        )
    }
    fn minion_battlecry(&self, minion: Minion) {
        println!(
            "随从 [{}:{}] 战吼效果发动",
            minion.uuid(),
            minion.model().card.name
        )
    }
    fn minion_attack(&self, minion: Minion, target: Target) {
        match target {
            Target::Minion(target_minion) => println!(
                "随从 [{}:{}] 攻击了 随从 [{}]",
                minion.uuid(),
                minion.model().card.name,
                target_minion,
            ),
            Target::Hero(target_hero) => println!(
                "随从 [{}:{}] 攻击了 英雄 [{}]",
                minion.uuid(),
                minion.model().card.name,
                target_hero,
            ),
        }
    }
    fn minion_death(&self, minion: Minion) {
        println!("随从 [{}:{}] 死亡", minion.uuid(), minion.model().card.name)
    }
    fn minion_deathrattle(&self, minion: Minion) {
        println!(
            "随从 [{}:{}] 亡语效果发动",
            minion.uuid(),
            minion.model().card.name
        )
    }

    fn deal_damage(&self, target: Target, damage: i64) {
        if damage > 0 {
            match target {
                Target::Minion(target_minion) => {
                    println!("随从 [{}] 受到伤害，生命值 -{}", target_minion, damage)
                }
                Target::Hero(target_hero) => {
                    println!("英雄 [{}] 受到伤害生命值 -{}", target_hero, damage)
                }
            }
        } else if damage < 0 {
            match target {
                Target::Minion(target_minion) => {
                    println!("随从 [{}] 受到治疗生命值 +{}", target_minion, -damage)
                }
                Target::Hero(target_hero) => {
                    println!("英雄 [{}] 受到治疗生命值 +{}", target_hero, -damage)
                }
            }
        }
    }
    fn buff(&self, target: Target, buff: Buff) {
        match target {
            Target::Minion(target_minion) => {
                println!(
                    "随从 [{}] 获得 Buff {}/{}",
                    target_minion,
                    has_sign_num(buff.atk_boost()),
                    has_sign_num(buff.hp_boost())
                )
            }
            Target::Hero(target_hero) => {
                println!(
                    "英雄 [{}] 获得 Buff {}/{}",
                    target_hero,
                    has_sign_num(buff.atk_boost()),
                    has_sign_num(buff.hp_boost())
                )
            }
        }
    }
}

#[async_trait::async_trait]
impl PlayerBehavior for StdBehavior {
    async fn assign_uuid(&self, uuid: u64) {}

    async fn next_action(&self, game: &Game, player: &Player) -> PlayerTurnAction {
        loop {
            let turn_action_type: u8 = Input::new()
                .with_prompt("输入回合指令(1: 使用卡牌, 2: 随从攻击, 0: 结束回合) > ")
                .interact_text()
                .expect("input turn action type");

            match turn_action_type {
                0 => return PlayerTurnAction::EndTurn,
                1 => {
                    let hand_index = Input::new()
                        .with_prompt(" 输入牌序号(从0开始) > ")
                        .interact_text()
                        .expect("input hand_index");

                    let target = input_target();

                    return PlayerTurnAction::PlayCard { hand_index, target };
                }
                2 => {
                    let attacker = Input::new()
                        .with_prompt(" 输入攻击随从ID > ")
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
                    return Some(Target::Minion(id));
                } else {
                    return Some(Target::Hero(id));
                }
            }
            _ => continue,
        }
    }
}

fn fightline_desc(fightline: Fightline) -> &'static str {
    match fightline {
        Fightline::Front => "前排",
        Fightline::Back => "后排",
    }
}

fn camp_desc(camp: Camp) -> &'static str {
    match camp {
        Camp::A => "A",
        Camp::B => "B",
    }
}

fn has_sign_num(n: i32) -> String {
    if n < 0 {
        format!("{n}")
    } else {
        format!("+{n}")
    }
}
