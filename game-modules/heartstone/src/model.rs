use datastructure::macros::TwoValue;
use datastructure::{SyncHandle, TwoValueEnum};
use rand::seq::SliceRandom;
use std::collections::HashMap;
use std::sync::Arc;
use web_db::hearthstone::{CardType, SpecialCardInfo};

#[derive(Debug)]
pub(crate) struct UuidGenerator(u64);

impl UuidGenerator {
    #[inline]
    pub fn new() -> Self {
        Self(0)
    }

    #[inline]
    pub fn gen(&mut self) -> u64 {
        let val = self.0;
        self.0 += 1;
        val
    }
}

/// 数据库中定义的卡牌模型
#[derive(Debug)]
pub struct CardModel {
    pub card: web_db::hearthstone::Card,
    pub card_info: web_db::hearthstone::CardInfo,
}

impl CardModel {
    pub fn card_type(&self) -> CardType {
        self.card
            .card_type
            .try_into()
            .expect("card type is not valid")
    }
}

/// 阵营
///
/// 在各种结算时，A阵营会比B阵营先进行结算
#[derive(Debug, PartialEq, Eq, Hash, Clone, Copy, TwoValue)]
pub enum Camp {
    A = 1,
    B = 2,
}

/// 前后排
///
/// 对于前排：可以操作随从行动，但不能召唤随从。
/// 对于后排：可以召唤随从，但不能操作随从行动。随从被召唤后不可攻击，需要在下一回合才可以攻击。
#[derive(Debug, PartialEq, Eq, Hash, Clone, Copy, TwoValue)]
pub enum Fightline {
    Front = 1,
    Back = 2,
}

/// 可造成伤害的单位
#[async_trait::async_trait]
pub trait Damageable {
    /// 获取单位的生命值
    async fn hp(&self) -> i64;

    /// 获取单位的最大生命值
    async fn max_hp(&self) -> u32;

    /// 造成伤害，允许产生负数生命值
    async fn damage(&mut self, damage: i64);

    /// 回复生命，生命值最高不能超过 max_hp
    #[inline]
    async fn heal(&mut self, hp: i64) {
        self.damage(-hp).await
    }

    /// 消灭
    ///
    /// 消灭是与生命值独立的另一种击杀方式
    async fn kill(&mut self);

    /// 判断当前是否存活
    async fn is_alive(&self) -> bool;
}

/// 英雄
///
/// 用于控制英雄基本信息
#[derive(Debug)]
pub struct Hero {
    uuid: u64,

    /// 生命值
    hp: i64,
    /// 最大生命值
    max_hp: u32,
    /// 消灭标记
    killed: bool,

    /// 当前位置
    fightline: Fightline,
}

#[async_trait::async_trait]
pub trait HeroTrait {
    async fn fightline(&self) -> Fightline;

    async fn swap_fightline(&mut self);

    async fn uuid(&self) -> u64;
}

#[async_trait::async_trait]
impl HeroTrait for SyncHandle<Hero> {
    async fn fightline(&self) -> Fightline {
        self.get().await.fightline
    }

    async fn swap_fightline(&mut self) {
        let mut hero = self.get_mut().await;
        hero.fightline = hero.fightline.opposite();
    }

    async fn uuid(&self) -> u64 {
        self.get().await.uuid
    }
}

impl Hero {
    pub fn new(uuid: u64, max_hp: u32, fightline: Fightline) -> SyncHandle<Hero> {
        let hero = Hero {
            uuid,
            hp: max_hp as i64,
            max_hp,
            fightline,
            killed: false,
        };

        SyncHandle::new(hero)
    }
}

#[async_trait::async_trait]
impl Damageable for SyncHandle<Hero> {
    async fn hp(&self) -> i64 {
        self.get().await.hp
    }

    async fn max_hp(&self) -> u32 {
        self.get().await.max_hp
    }

    async fn damage(&mut self, damage: i64) {
        let mut hero = self.get_mut().await;
        #[allow(clippy::collapsible_else_if)]
        if damage > 0 {
            hero.hp -= damage;
        } else {
            if hero.hp - damage >= hero.max_hp.into() {
                hero.hp = hero.max_hp.into();
            } else {
                hero.hp -= damage;
            }
        }
    }

    async fn kill(&mut self) {
        self.get_mut().await.killed = true
    }

    async fn is_alive(&self) -> bool {
        let hero = self.get().await;
        !hero.killed && hero.hp > 0
    }
}

#[derive(Debug, Clone)]
pub struct Buff {
    buff_type: i32,
    atk_boost: i32,
    hp_boost: i32,
}
impl Buff {
    pub fn new(buff_type: i32, atk_boost: i32, hp_boost: i32) -> Self {
        Self {
            buff_type,
            atk_boost,
            hp_boost,
        }
    }

    pub fn atk_boost(&self) -> i32 {
        self.atk_boost
    }

    pub fn hp_boost(&self) -> i32 {
        self.hp_boost
    }

    pub fn buff_type(&self) -> i32 {
        self.buff_type
    }
}

#[async_trait::async_trait]
pub trait Buffable {
    async fn buff_list(&self) -> Vec<Buff>;
    async fn buff(&mut self, buff: Buff);
}

/// 随从
#[derive(Debug, Clone)]
pub struct Minion {
    model: Arc<CardModel>,
    uuid: u64,
    atk: i32,
    hp: i64,
    max_hp: u32,
    killed: bool,

    buff_list: Vec<Buff>,
}

impl Minion {
    pub fn new(model: Arc<CardModel>, uuid: u64) -> SyncHandle<Minion> {
        let SpecialCardInfo::Minion(info) = &model.card_info.special_card_info else {
            panic!("card_info is not minion info")
        };

        let minion = Minion {
            uuid,
            atk: info.attack,
            hp: info.health.into(),
            max_hp: info.health as u32,
            killed: false,
            model,
            buff_list: Vec::new(),
        };

        SyncHandle::new(minion)
    }

    pub fn model(&self) -> Arc<CardModel> {
        self.model.clone()
    }

    pub fn atk(&self) -> i32 {
        self.atk
    }

    pub fn hp(&self) -> i64 {
        self.hp
    }

    pub fn uuid(&self) -> u64 {
        self.uuid
    }
}

#[async_trait::async_trait]
pub trait MinionTrait {
    async fn model(&self) -> Arc<CardModel>;

    async fn atk(&self) -> i32;

    async fn uuid(&self) -> u64;
}

#[async_trait::async_trait]
impl MinionTrait for SyncHandle<Minion> {
    async fn model(&self) -> Arc<CardModel> {
        self.get().await.model.clone()
    }

    async fn atk(&self) -> i32 {
        self.get().await.atk
    }

    async fn uuid(&self) -> u64 {
        self.get().await.uuid
    }
}

#[async_trait::async_trait]
impl Damageable for SyncHandle<Minion> {
    async fn hp(&self) -> i64 {
        self.get().await.hp
    }

    async fn max_hp(&self) -> u32 {
        self.get().await.max_hp
    }

    async fn damage(&mut self, damage: i64) {
        let mut minion = self.get_mut().await;
        #[allow(clippy::collapsible_else_if)]
        if damage > 0 {
            minion.hp -= damage;
        } else {
            if minion.hp - damage >= minion.max_hp.into() {
                minion.hp = minion.max_hp.into();
            } else {
                minion.hp -= damage;
            }
        }
    }

    async fn kill(&mut self) {
        self.get_mut().await.killed = true;
    }

    async fn is_alive(&self) -> bool {
        let minion = self.get().await;
        !minion.killed && minion.hp > 0
    }
}

#[async_trait::async_trait]
impl Buffable for SyncHandle<Minion> {
    async fn buff_list(&self) -> Vec<Buff> {
        self.get().await.buff_list.clone()
    }

    async fn buff(&mut self, buff: Buff) {
        let mut minion = self.get_mut().await;
        minion.atk += buff.atk_boost;
        minion.max_hp = (minion.max_hp as i64 + buff.hp_boost as i64) as u32;

        if buff.hp_boost > 0 {
            minion.hp += buff.hp_boost as i64;
        }
        if minion.hp > minion.max_hp.into() {
            minion.hp = minion.max_hp.into();
        }

        minion.buff_list.push(buff);
    }
}

/// （手牌、牌库中的）卡牌
#[derive(Debug, Clone)]
pub struct Card {
    model: Arc<CardModel>,
}

impl Card {
    pub fn new(model: Arc<CardModel>) -> SyncHandle<Card> {
        SyncHandle::new(Self::new_raw(model))
    }

    pub fn new_raw(model: Arc<CardModel>) -> Card {
        Card { model }
    }

    pub fn model(&self) -> Arc<CardModel> {
        self.model.clone()
    }
}

/// 对局中使用的所有牌定义
///
/// key: card_id, value: card_model
pub type CardPool = HashMap<i64, Arc<CardModel>>;

/// 牌库
#[derive(Debug)]
pub struct Deck {
    cards: Vec<SyncHandle<Card>>,
}

#[async_trait::async_trait]
pub trait DeckTrait {
    async fn draw(&mut self) -> Option<SyncHandle<Card>>;
}

#[async_trait::async_trait]
impl DeckTrait for SyncHandle<Deck> {
    async fn draw(&mut self) -> Option<SyncHandle<Card>> {
        let cards = &mut self.get_mut().await.cards;
        if cards.is_empty() {
            None
        } else {
            Some(cards.remove(0))
        }
    }
}

impl Deck {
    pub fn new<Rng: rand::Rng>(
        card_pool: &CardPool,
        init_cards: HashMap<i64, u32>,
        rng: &mut Rng,
    ) -> SyncHandle<Deck> {
        let mut cards = Vec::new();
        for (card_id, count) in init_cards {
            let model = card_pool
                .get(&card_id)
                .expect("init deck with undefined card model");
            for _ in 0..count {
                cards.push(Card::new(model.clone()));
            }
        }

        cards.shuffle(rng);

        SyncHandle::new(Deck { cards })
    }
}

/// 手牌
#[derive(Debug)]
pub struct Hand {
    cards: Vec<SyncHandle<Card>>,
}

#[async_trait::async_trait]
pub trait HandTrait {
    async fn remove(&mut self, index: usize) -> Option<SyncHandle<Card>>;

    async fn gain_card(&mut self, card: SyncHandle<Card>) -> bool;

    async fn cards(&self) -> Vec<SyncHandle<Card>>;
}

#[async_trait::async_trait]
impl HandTrait for SyncHandle<Hand> {
    async fn remove(&mut self, index: usize) -> Option<SyncHandle<Card>> {
        let mut hand = self.get_mut().await;
        if hand.cards.len() > index {
            Some(hand.cards.remove(index))
        } else {
            None
        }
    }

    async fn gain_card(&mut self, card: SyncHandle<Card>) -> bool {
        let mut hand = self.get_mut().await;
        if hand.cards.len() < 10 {
            hand.cards.push(card);
            true
        } else {
            false
        }
    }

    async fn cards(&self) -> Vec<SyncHandle<Card>> {
        self.get().await.cards.clone()
    }
}

impl Hand {
    pub fn new() -> SyncHandle<Hand> {
        SyncHandle::new(Hand { cards: Vec::new() })
    }
}

/// 战场
#[derive(Debug)]
pub struct Battlefield {
    minions: Vec<(SyncHandle<Minion>, bool /* death checking */)>,
}

#[async_trait::async_trait]
pub trait BattlefieldTrait {
    /// 获取战场上存活的随从
    async fn alive_minions(&self) -> Vec<SyncHandle<Minion>>;

    /// 获取战场上的所有随从（活着的随从、濒死的随从、正在进行死亡结算的随从）
    async fn all_minions(&self) -> Vec<SyncHandle<Minion>>;

    /// 存活检查（将濒死的随从标记为正在进行死亡结算的随从，然后返回标记的列表）
    async fn alive_check(&mut self) -> Vec<SyncHandle<Minion>>;

    /// 移除正在进行死亡结算的随从（他们已经完成了死亡结算）
    async fn remove_death_minions(&mut self);

    async fn summon_minion(&mut self, minion: SyncHandle<Minion>);
}

#[async_trait::async_trait]
impl BattlefieldTrait for SyncHandle<Battlefield> {
    async fn alive_minions(&self) -> Vec<SyncHandle<Minion>> {
        self.get()
            .await
            .minions
            .iter()
            .map(|(minion, _)| minion.clone())
            .collect()
    }

    async fn all_minions(&self) -> Vec<SyncHandle<Minion>> {
        let mut result = Vec::new();
        for (minion, death_checking) in &self.get().await.minions {
            if !death_checking && minion.is_alive().await {
                result.push(minion.clone());
            }
        }
        result
    }

    async fn summon_minion(&mut self, minion: SyncHandle<Minion>) {
        self.get_mut().await.minions.push((minion, false));
    }

    async fn alive_check(&mut self) -> Vec<SyncHandle<Minion>> {
        let mut battlefield = self.get_mut().await;

        let mut death_minions = Vec::new();

        for (minion, death_checking) in &mut battlefield.minions {
            if !minion.is_alive().await {
                *death_checking = true;
                death_minions.push(minion.clone());
            }
        }

        death_minions
    }

    async fn remove_death_minions(&mut self) {
        let mut battlefield = self.get_mut().await;

        let mut alive_minions = Vec::new();

        for (minion, death_checking) in &battlefield.minions {
            if !*death_checking {
                alive_minions.push((minion.clone(), false));
            }
        }

        battlefield.minions = alive_minions;
    }
}

impl Battlefield {
    pub fn new() -> SyncHandle<Battlefield> {
        SyncHandle::new(Battlefield {
            minions: Vec::new(),
        })
    }
}

#[derive(Debug, Clone, Copy)]
pub enum Target {
    Minion(u64),
    Hero(u64),
}
