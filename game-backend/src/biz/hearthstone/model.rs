use super::db_cache;
use anyhow::Result;
use std::sync::Arc;
use web_db::hearthstone::SpecialCardInfo;

// 阵营
#[derive(Debug, PartialEq, Eq, Hash)]
pub enum Camp {
    A,
    B,
}

// 前后方
#[derive(Debug, Clone, Copy)]
pub enum Fightline {
    Front,
    Back,
}

impl Camp {
    pub fn opposite(&self) -> Camp {
        match self {
            Camp::A => Camp::B,
            Camp::B => Camp::A,
        }
    }
}

impl Fightline {
    pub fn swap(&self) -> Fightline {
        match self {
            Fightline::Front => Fightline::Back,
            Fightline::Back => Fightline::Front,
        }
    }
}

// 卡牌（手中、战场上）
// 单独抽出一个结构体，为后期污手党、心火牧做准备
pub struct Card {
    // 对应db卡牌
    card: Arc<db_cache::DbCardCache>,
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
}

pub struct Minion {
    model: Arc<db_cache::DbCardCache>,
    atk: i32,
    hp: i32,
    maxhp: i32,
    buf_list: Vec<Buff>,
}

pub trait Damageable {
    fn damage(&mut self, damage: i32);

    fn heal(&mut self, heal: i32) {
        self.damage(-heal);
    }
}

impl Minion {
    pub fn get_atk(&self) -> i32 {
        if self.atk > 0 {
            self.atk
        } else {
            0
        }
    }

    pub fn buff(&mut self, buff: Buff) {
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

impl Damageable for Minion {
    fn damage(&mut self, damage: i32) {
        self.hp -= damage;
        if self.hp > self.maxhp {
            self.hp = self.maxhp;
        }
    }
}

pub struct Buff {
    from_model: Arc<db_cache::DbCardCache>,
    buff_name: String,

    atk_boost: i32,
    hp_boost: i32,
}
