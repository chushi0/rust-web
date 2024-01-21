use crate::{
    game::{Game, PlayerConfig},
    model::{
        Camp, Card, CardPool, Deck, DeckTrait, Fightline, Hand, HandTrait, Hero, HeroTrait, Target,
    },
};
use datastructure::SyncHandle;
use std::{fmt::Debug, sync::Arc};

#[async_trait::async_trait]
pub trait PlayerBehavior: Debug + Send + Sync {
    /// 初始化玩家时授予的uuid，只会调用一次
    async fn assign_uuid(&self, uuid: u64);

    /// 准备阶段的行动
    ///
    /// 此阶段有全局时间限制，在时间达到限制时会立刻中断。如果函数依赖资源清理，需要实现Drop trait（或其他清理方式）。
    /// 当客户端明确表示不会再有下一步行动时可以返回None，将不再等待
    async fn next_starting_action(&self, player: &Player) -> Option<PlayerStartingAction>;
    /// 结束准备阶段
    /// 可用于清理next_starting_action被取消后，需要清理的资源
    async fn finish_starting_action(&self, player: &Player);

    /// 玩家回合行动
    async fn next_action(&self, game: &Game, player: &Player) -> PlayerTurnAction;
}

#[derive(Debug)]
pub struct Player {
    custom_id: i64,
    camp: Camp,

    behavior: Arc<dyn PlayerBehavior>,
    hero: SyncHandle<Hero>,
    hand: SyncHandle<Hand>,
    deck: SyncHandle<Deck>,

    mana: i32,
    max_mana: u16,
    tired: u32,
}

/// 玩家开始
pub enum PlayerStartingAction {
    SwapStartingCards { cards_index: Vec<usize> },
    ChooseFightline { fightline: Option<Fightline> },
    LockFightline,
    UnlockFightline,
}

/// 玩家回合基本行动
#[derive(Debug)]
pub enum PlayerTurnAction {
    /// 使用手牌
    PlayCard {
        hand_index: usize,
        target: Option<Target>,
    },
    /// 随从攻击
    MinionAttack { attacker: u64, target: Target },
    /// 结束回合
    EndTurn,
}

pub enum DrawCardResult {
    Draw(SyncHandle<Card>),
    Fire(SyncHandle<Card>),
    Tired(u32),
}

#[async_trait::async_trait]
pub trait PlayerTrait {
    async fn next_starting_action(&self) -> Option<PlayerStartingAction>;

    async fn finish_starting_action(&self);

    async fn next_action(&self, game: &Game) -> PlayerTurnAction;

    async fn get_hero(&self) -> SyncHandle<Hero>;

    async fn camp(&self) -> Camp;

    async fn remove_hand_card(&mut self, index: usize) -> Option<SyncHandle<Card>>;

    async fn turn_reset_mana(&mut self);

    async fn mana(&self) -> i32;

    async fn cost_mana(&mut self, mana_cost: i32);

    async fn draw_card(&mut self) -> DrawCardResult;

    async fn draw_starting_card(&mut self);

    async fn swap_starting_card<Rng: rand::Rng + Send>(&mut self, index: &[usize], rng: &mut Rng);

    async fn change_fightline_to(&mut self, fightline: Fightline) {
        self.get_hero().await.change_fightline_to(fightline).await
    }

    async fn hand_cards(&self) -> Vec<SyncHandle<Card>>;

    async fn uuid(&self) -> u64 {
        self.get_hero().await.uuid().await
    }

    async fn get_custom_id(&self) -> i64;
}

#[async_trait::async_trait]
impl PlayerTrait for SyncHandle<Player> {
    async fn next_starting_action(&self) -> Option<PlayerStartingAction> {
        let player = self.get().await;
        player.behavior.next_starting_action(&player).await
    }

    async fn finish_starting_action(&self) {
        let player = self.get().await;
        player.behavior.finish_starting_action(&player).await;
    }

    async fn next_action(&self, game: &Game) -> PlayerTurnAction {
        // 从behavior中读取action信息，并判定输入是否合法
        let player = self.get().await;
        loop {
            let action = player.behavior.next_action(game, &player).await;
            let check_action = match action {
                PlayerTurnAction::PlayCard {
                    hand_index: _,
                    target: _,
                } => {
                    // 检查牌存在
                    // 检查法力值足够
                    // 检查选择目标
                    true
                }
                PlayerTurnAction::MinionAttack {
                    attacker: _,
                    target: _,
                } => {
                    // 检查攻击随从存在
                    // 检查目标存在
                    // 检查当前回合尚未攻击
                    true
                }
                PlayerTurnAction::EndTurn => true,
            };
            if check_action {
                return action;
            }
        }
    }

    async fn get_hero(&self) -> SyncHandle<Hero> {
        self.get().await.hero.clone()
    }

    async fn camp(&self) -> Camp {
        self.get().await.camp
    }

    async fn remove_hand_card(&mut self, index: usize) -> Option<SyncHandle<Card>> {
        self.get_mut().await.hand.remove(index).await
    }

    async fn turn_reset_mana(&mut self) {
        let mut player = self.get_mut().await;
        if player.max_mana < 10 {
            player.max_mana += 1;
        }
        player.mana = player.max_mana as i32;
    }

    async fn mana(&self) -> i32 {
        self.get().await.mana
    }

    async fn cost_mana(&mut self, mana_cost: i32) {
        self.get_mut().await.mana -= mana_cost;
    }

    async fn draw_card(&mut self) -> DrawCardResult {
        let mut player = self.get_mut().await;

        let card = player.deck.draw().await;
        match card {
            Some(card) => {
                if player.hand.gain_card(card.clone()).await {
                    DrawCardResult::Draw(card)
                } else {
                    DrawCardResult::Fire(card)
                }
            }
            None => {
                player.tired += 1;
                DrawCardResult::Tired(player.tired)
            }
        }
    }

    async fn draw_starting_card(&mut self) {
        let mut player = self.get_mut().await;

        for _ in 0..4 {
            match player.deck.draw().await {
                Some(card) => {
                    let draw_result = player.hand.gain_card(card).await;
                    assert!(draw_result, "draw starting card should not be fail");
                }
                // 牌库连四张牌都没有吗？太惨了。。。
                None => break,
            }
        }
    }

    async fn swap_starting_card<Rng: rand::Rng + Send>(&mut self, index: &[usize], rng: &mut Rng) {
        let mut player = self.get_mut().await;
        for index in index {
            if *index >= 4 {
                continue;
            }
            match player.deck.draw().await {
                Some(card) => {
                    let card = player.hand.replace_card(*index, card).await;
                    player.deck.put(card, rng).await;
                }
                None => break,
            };
        }
    }

    async fn hand_cards(&self) -> Vec<SyncHandle<Card>> {
        self.get().await.hand.cards().await
    }

    async fn get_custom_id(&self) -> i64 {
        self.get().await.custom_id
    }
}

impl Player {
    pub async fn new<Rng: rand::Rng>(
        hero_uuid: u64,
        card_pool: &CardPool,
        config: PlayerConfig,
        camp: Camp,
        fightline: Fightline,
        rng: &mut Rng,
    ) -> SyncHandle<Player> {
        let behavior = config.behavior;
        let hero = Hero::new(hero_uuid, config.max_hero_hp, fightline);
        let hand = Hand::new();
        let deck = Deck::new(card_pool, config.deck, rng);

        behavior.assign_uuid(hero_uuid).await;

        let player = Player {
            custom_id: config.custom_id,

            camp,
            behavior,
            hero,
            hand,
            deck,

            mana: 0,
            max_mana: 0,
            tired: 0,
        };
        SyncHandle::new(player)
    }

    pub fn custom_id(&self) -> i64 {
        self.custom_id
    }
}

#[derive(Debug, Default)]
pub struct AIPlayerBehavior {}

#[async_trait::async_trait]
impl PlayerBehavior for AIPlayerBehavior {
    async fn assign_uuid(&self, _uuid: u64) {}

    async fn next_starting_action(&self, _player: &Player) -> Option<PlayerStartingAction> {
        None
    }

    async fn finish_starting_action(&self, _player: &Player) {}

    async fn next_action(&self, _game: &Game, _player: &Player) -> PlayerTurnAction {
        PlayerTurnAction::EndTurn
    }
}
