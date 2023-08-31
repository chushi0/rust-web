use super::{consts, db_cache};
use anyhow::Result;
use async_trait::async_trait;
use datastructure::macros::TwoValue;
use datastructure::TwoValueEnum;
use std::{collections::HashMap, sync::Arc};
use web_db::hearthstone::SpecialCardInfo;

// 游戏
pub struct Game {
    players: HashMap<i64, Player>,
    battlefields: HashMap<Camp, Battlefield>,
    turn: u64,
    current_turn_action: datastructure::CycleArrayVector<TurnAction>,
}

// 玩家
pub struct Player {
    user_id: i64,

    camp: Camp,
    fightline: Fightline,

    hero_hp: i32,
    hand_cards: Vec<Card>,
    deck_cards: Vec<Card>,

    mana: i32,
    max_mana: i32,
    tired_val: i32,
}

// 阵营
#[derive(Debug, PartialEq, Eq, Hash, Clone, Copy, TwoValue)]
pub enum Camp {
    A = 1,
    B = 2,
}

// 前后方
#[derive(Debug, PartialEq, Eq, Hash, Clone, Copy, TwoValue)]
pub enum Fightline {
    Front,
    Back,
}

// 卡牌（手中、战场上）
// 单独抽出一个结构体，为后期污手党、心火牧做准备
#[derive(Debug, Clone)]
pub struct Card {
    // 对应db卡牌
    card: Arc<db_cache::DbCardCache>,
}

// 随从
pub struct Minion {
    model: Arc<db_cache::DbCardCache>,
    minion_id: u64,
    atk: i32,
    hp: i32,
    maxhp: i32,
    buf_list: Vec<Buff>,
}

// 战场
pub struct Battlefield {
    minions: Vec<Minion>,
}

// 回合行动
enum TurnAction {
    PlayerAction { uid: i64 },
    SwapFrontBack,
}

// 可伤害的对象
pub trait Damageable {
    fn damage(&mut self, damage: i32);

    fn heal(&mut self, heal: i32) {
        self.damage(-heal);
    }
}

// 可加buff的对象
pub trait Buffable {
    fn buff(&mut self, buff: Buff);
}

// 加的buff
#[derive(Clone)]
pub struct Buff {
    from_model: Arc<db_cache::DbCardCache>,
    buff_type: i32,

    atk_boost: i32,
    hp_boost: i32,
}

impl Card {
    pub async fn from_cache(code: String) -> Result<Card> {
        Ok(Card {
            card: db_cache::get_cache_card(code).await?,
        })
    }

    // 法力值消耗
    pub fn get_mana_cost(&self) -> i32 {
        self.card.card.mana_cost
    }

    // 随从攻击力
    pub fn get_minion_atk(&self) -> Option<i32> {
        if let SpecialCardInfo::Minion(info) = &self.card.card_info.special_card_info {
            Some(info.attack)
        } else {
            None
        }
    }

    // 随从血量
    pub fn get_minion_maxhp(&self) -> Option<i32> {
        if let SpecialCardInfo::Minion(info) = &self.card.card_info.special_card_info {
            Some(info.health)
        } else {
            None
        }
    }

    pub fn get_model(&self) -> Arc<db_cache::DbCardCache> {
        self.card.clone()
    }
}

impl Game {
    pub fn get_minion(&mut self, camp: &Camp, minion_id: u64) -> Option<&mut Minion> {
        let Some(battlefield) = self.battlefields.get_mut(camp) else {
            return None;
        };
        for minion in &mut battlefield.minions {
            if minion.id() == minion_id {
                return Some(minion);
            }
        }
        None
    }

    pub fn get_minions(&mut self, camp: &Camp) -> Vec<&mut Minion> {
        let mut result = vec![];
        let Some(battlefield) = self.battlefields.get_mut(camp) else {
            return result;
        };
        for minion in &mut battlefield.minions {
            result.push(minion);
        }
        result
    }

    pub fn get_all_minions(&mut self) -> Vec<&mut Minion> {
        let mut result = vec![];
        for (_, battlefield) in &mut self.battlefields {
            for minion in &mut battlefield.minions {
                result.push(minion);
            }
        }
        result
    }

    pub fn get_player(&mut self, uid: i64) -> Option<&mut Player> {
        self.players.get_mut(&uid)
    }

    pub fn get_player_by_camp_pos(
        &mut self,
        camp: &Camp,
        fightline: Fightline,
    ) -> Option<&mut Player> {
        for (_, player) in &mut self.players {
            if player.camp == *camp && player.fightline == fightline {
                return Some(player);
            }
        }
        None
    }

    pub fn get_player_by_pos(&mut self, fightline: Fightline) -> Option<&mut Player> {
        for (_, player) in &mut self.players {
            if player.fightline == fightline {
                return Some(player);
            }
        }
        None
    }

    pub fn get_player_by_camp(&mut self, camp: &Camp) -> Vec<&mut Player> {
        let mut result = vec![];
        for (_, player) in &mut self.players {
            if player.camp == *camp {
                result.push(player);
            }
        }
        result
    }

    pub fn get_all_players(&mut self) -> Vec<&mut Player> {
        let mut result = vec![];
        for (_, player) in &mut self.players {
            result.push(player);
        }
        result
    }

    pub fn get_minions_and_players_by_camp(
        &mut self,
        camp: &Camp,
    ) -> (Vec<&mut Minion>, Vec<&mut Player>) {
        let mut minions = vec![];
        let mut players = vec![];

        if let Some(battlefield) = self.battlefields.get_mut(camp) {
            for minion in &mut battlefield.minions {
                minions.push(minion);
            }
        }
        for (_, player) in &mut self.players {
            if player.camp == *camp {
                players.push(player);
            }
        }

        (minions, players)
    }

    pub fn get_all_minions_and_players(&mut self) -> (Vec<&mut Minion>, Vec<&mut Player>) {
        let mut minions = vec![];
        let mut players = vec![];

        for (_, battlefield) in &mut self.battlefields {
            for minion in &mut battlefield.minions {
                minions.push(minion);
            }
        }
        for (_, player) in &mut self.players {
            players.push(player);
        }

        (minions, players)
    }
}

#[async_trait]
pub trait GameEventSender: Sized {
    async fn send_to_player(self, uid: i64) {
        self.send_to_players(vec![uid]).await;
    }

    async fn send_to_players(self, uids: Vec<i64>);
}

#[must_use = "This info should be sent to player"]
pub struct PlayerDrawCardResult {
    draw_player_uid: i64,
    draw_cards: Vec<Card>,
    from_tired_val: i32,
    to_tired_val: i32,
}

impl Player {
    pub fn draw_card(&mut self, c: i32) -> PlayerDrawCardResult {
        let from_tired_val = self.tired_val;
        let mut draw_cards = vec![];
        for _ in 0..c {
            let card = self.draw_card_internal();
            if let Some(card) = card {
                draw_cards.push(card);
            }
        }
        PlayerDrawCardResult {
            draw_player_uid: self.user_id,
            draw_cards,
            from_tired_val,
            to_tired_val: self.tired_val,
        }
    }

    fn draw_card_internal(&mut self) -> Option<Card> {
        if self.deck_cards.is_empty() {
            self.tired_val += 1;
            self.damage(self.tired_val);
            return None;
        }
        self.hand_cards.push(self.deck_cards[0].clone());
        let card = self.deck_cards.remove(0);
        Some(card)
    }
}

impl Minion {
    pub fn new(id: u64, card: &Card) -> Minion {
        let model = card.get_model().clone();
        Minion {
            model,
            minion_id: id,
            atk: card.get_minion_atk().expect("minion should has atk"),
            hp: card.get_minion_maxhp().expect("minino should has maxhp"),
            maxhp: card.get_minion_maxhp().expect("minion shuold has maxhp"),
            buf_list: Vec::new(),
        }
    }

    pub fn id(&self) -> u64 {
        self.minion_id
    }

    pub fn get_atk(&self) -> i32 {
        if self.atk > 0 {
            self.atk
        } else {
            0
        }
    }
}

impl Damageable for Minion {
    fn damage(&mut self, damage: i32) {
        self.hp -= damage;
        if self.hp > self.maxhp {
            self.hp = self.maxhp;
        }
    }
}

impl Damageable for Player {
    fn damage(&mut self, damage: i32) {
        self.hero_hp -= damage;
        if self.hero_hp > consts::MAX_HERO_HP {
            self.hero_hp = consts::MAX_HERO_HP;
        }
    }
}

impl Buffable for Minion {
    fn buff(&mut self, buff: Buff) {
        self.atk += buff.atk_boost;

        if buff.hp_boost >= 0 {
            self.hp += buff.hp_boost;
            self.maxhp += buff.hp_boost;
        } else {
            self.maxhp += buff.hp_boost;
            if self.hp > self.maxhp {
                self.hp = self.maxhp;
            }
        }

        self.buf_list.push(buff);
    }
}

impl Buff {
    pub fn new(
        model: Arc<db_cache::DbCardCache>,
        buff_type: i32,
        atk_boost: i32,
        hp_boost: i32,
    ) -> Buff {
        Buff {
            from_model: model,
            buff_type,
            atk_boost,
            hp_boost,
        }
    }
}

#[async_trait]
impl GameEventSender for PlayerDrawCardResult {
    async fn send_to_players(self, uid: Vec<i64>) {}
}
