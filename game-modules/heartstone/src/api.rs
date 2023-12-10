use datastructure::SyncHandle;

use crate::{
    game::{Game, TurnAction},
    model::{Buff, Camp, Card, Fightline, Minion, Target},
    player::Player,
};
use std::fmt::Debug;

/// 游戏事件通知
///
/// 当事件产生时，应当将信息暂存在内存中。当flush函数被调用时，再将信息同步给玩家。
#[async_trait::async_trait]
pub trait GameNotifier: Debug + Send + Sync {
    /// 将事件同步给玩家。
    /// 如果事件为空，则不执行任何操作
    ///
    /// game: 用于获取事件详细数据的游戏对象
    async fn flush(&self, game: &Game);

    fn new_turn(&self, current_turn: TurnAction);

    fn player_mana_change(&self, player: u64, mana: i32);
    fn player_draw_card(&self, player: u64, card: PlayerDrawCard);
    fn player_use_card(&self, player: u64, card: Card, cost_mana: i32);
    fn player_card_effect_end(&self);
    fn player_swap_fightline(&self, player: u64, new_fightline: Fightline);

    fn minion_summon(&self, minion: Minion, camp: Camp);
    fn minion_battlecry(&self, minion: Minion);
    fn minion_attack(&self, minion: Minion, target: Target);
    fn minion_death(&self, minion: Minion);
    fn minion_deathrattle(&self, minion: Minion);

    fn deal_damage(&self, target: Target, damage: i64);
    fn buff(&self, target: Target, buff: Buff);
}

#[derive(Debug)]
pub enum PlayerDrawCard {
    Draw(Card),
    Fire(Card),
    Tired(u32),
}

#[derive(Debug)]
pub(crate) struct NopGameNotifier;

#[async_trait::async_trait]
#[allow(unused_variables)]
impl GameNotifier for NopGameNotifier {
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
