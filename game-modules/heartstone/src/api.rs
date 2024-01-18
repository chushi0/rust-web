use crate::{
    game::Game,
    model::{Buff, Camp, Card, Fightline, Minion, Target},
};
use std::fmt::Debug;

#[async_trait::async_trait]
pub trait GameStartingNotifier {
    /// 在游戏初始化阶段的事件同步
    /// 此时还没有生成游戏对象，因此没有Game引用
    async fn flush_at_starting(&self);

    /// 阵营决定
    fn camp_decide(&self, player: u64, camp: Camp);
    /// 初始手牌抽取
    fn starting_card(&self, player: u64, cards: Vec<Card>);
    /// 更换起始手牌
    fn change_starting_card(&self, player: u64, change_card_index: &[usize], new_cards: Vec<Card>);
    /// 前后排选择
    fn fightline_choose(&self, player: u64, fightline: Option<Fightline>);
    /// 前后排锁定
    fn fightline_lock(&self, player: u64, fightline: Fightline);
    /// 前后排解锁
    fn fightline_unlock(&self, player: u64);
    /// 前后排最终决定
    fn fightline_decide(&self, player: u64, fightline: Fightline);
}

#[async_trait::async_trait]
pub trait GameRunningNotifier {
    /// 将事件同步给玩家。
    /// 如果事件为空，则不执行任何操作
    ///
    /// game: 用于获取事件详细数据的游戏对象
    async fn flush(&self, game: &Game);

    /// 新回合（玩家回合或交换前后排回合）
    fn new_turn(&self, current_turn: TurnAction);

    /// 玩家法力值变化
    fn player_mana_change(&self, player: u64, mana: i32);
    /// 玩家抽卡
    fn player_draw_card(&self, player: u64, card: PlayerDrawCard);
    /// 玩家使用卡
    fn player_use_card(&self, player: u64, card: Card, cost_mana: i32);
    /// 玩家卡牌效果结束
    fn player_card_effect_end(&self);
    /// 玩家交换前后排
    fn player_swap_fightline(&self, player: u64, new_fightline: Fightline);

    /// 随从入场
    fn minion_summon(&self, minion: Minion, camp: Camp);
    /// 随从战吼效果触发
    fn minion_battlecry(&self, minion: Minion);
    /// 随从攻击
    fn minion_attack(&self, minion: Minion, target: Target);
    /// 随从离场
    fn minion_death(&self, minion: Minion);
    /// 随从亡语
    fn minion_deathrattle(&self, minion: Minion);

    /// 伤害处理
    fn deal_damage(&self, target: Target, damage: i64);
    /// buff
    fn buff(&self, target: Target, buff: Buff);
}

/// 游戏事件通知
///
/// 当事件产生时，应当将信息暂存在内存中。当flush函数被调用时，再将信息同步给玩家。
pub trait GameNotifier: Debug + Send + Sync + GameStartingNotifier + GameRunningNotifier {}

#[derive(Debug, Clone)]
pub enum TurnAction {
    PlayerTurn(u64),
    SwapFightline,
}

#[derive(Debug)]
pub enum PlayerDrawCard {
    Draw(Card),
    Fire(Card),
    Tired(u32),
}

#[derive(Debug)]
pub(crate) struct NopGameNotifier;

impl GameNotifier for NopGameNotifier {}

#[async_trait::async_trait]
#[allow(unused_variables)]
impl GameStartingNotifier for NopGameNotifier {
    async fn flush_at_starting(&self) {}

    fn camp_decide(&self, player: u64, camp: Camp) {}
    fn starting_card(&self, player: u64, cards: Vec<Card>) {}
    fn change_starting_card(&self, player: u64, change_card_index: &[usize], new_cards: Vec<Card>) {
    }
    fn fightline_choose(&self, player: u64, fightline: Option<Fightline>) {}
    fn fightline_lock(&self, player: u64, fightline: Fightline) {}
    fn fightline_unlock(&self, player: u64) {}
    fn fightline_decide(&self, player: u64, fightline: Fightline) {}
}

#[async_trait::async_trait]
#[allow(unused_variables)]
impl GameRunningNotifier for NopGameNotifier {
    async fn flush(&self, game: &Game) {}

    fn new_turn(&self, current_turn: TurnAction) {}

    fn player_mana_change(&self, player: u64, mana: i32) {}
    fn player_draw_card(&self, player: u64, card: PlayerDrawCard) {}
    fn player_use_card(&self, player: u64, card: Card, cost_mana: i32) {}
    fn player_card_effect_end(&self) {}
    fn player_swap_fightline(&self, player: u64, new_fightline: Fightline) {}

    fn minion_summon(&self, minion: Minion, camp: Camp) {}
    fn minion_battlecry(&self, minion: Minion) {}
    fn minion_attack(&self, minion: Minion, target: Target) {}
    fn minion_death(&self, minion: Minion) {}
    fn minion_deathrattle(&self, minion: Minion) {}

    fn deal_damage(&self, target: Target, damage: i64) {}
    fn buff(&self, target: Target, buff: Buff) {}
}
