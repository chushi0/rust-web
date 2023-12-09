use crate::game::{Game, TurnAction};
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
    async fn flush(&mut self, game: &Game);

    fn new_turn(&mut self, current_turn: TurnAction);
}

#[derive(Debug)]
pub(crate) struct NopGameNotifier;

#[async_trait::async_trait]
impl GameNotifier for NopGameNotifier {
    async fn flush(&mut self, _game: &Game) {
        log::info!("[NopGameNotifier] flush");
    }

    fn new_turn(&mut self, current_turn: TurnAction) {
        log::info!("[NopGameNotifier] new_turn, current_turn={current_turn:?}");
    }
}
