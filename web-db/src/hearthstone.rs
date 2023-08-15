use anyhow::Result;
use futures::TryStreamExt;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct Card {
    pub rowid: i64,
    pub code: String,
    pub name: String,
    pub card_type: i32,
    pub mana_cost: i32,
    pub derive: bool,
    pub need_select_target: bool,
    pub card_info: String,
    pub create_info: i64,
    pub update_time: i64,
    pub enable: bool,
}

#[derive(Debug, Clone, Copy)]
pub enum CardType {
    Minion,
    Spell,
}

impl TryFrom<i32> for CardType {
    type Error = anyhow::Error;

    fn try_from(value: i32) -> std::result::Result<Self, Self::Error> {
        match value {
            1 => Ok(Self::Minion),
            2 => Ok(Self::Spell),

            _ => Err(anyhow::anyhow!("unknown enum value ({value})")),
        }
    }
}

impl From<CardType> for i32 {
    fn from(value: CardType) -> Self {
        match value {
            CardType::Minion => 1,
            CardType::Spell => 2,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CardInfo {
    #[serde(flatten)]
    pub common_card_info: CommonCardInfo,
    #[serde(flatten)]
    pub special_card_info: SpecialCardInfo,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommonCardInfo {}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SpecialCardInfo {
    Minion(MinionCardInfo),
    Spell(SpellCardInfo),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MinionCardInfo {
    pub attack: i32,
    pub health: i32,
    pub effects: Vec<MinionEffect>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpellCardInfo {
    pub effects: Vec<SpellEffect>,
}

pub type CardEffects = Vec<CardEffect>;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MinionEffect {
    // 战吼
    Battlecry {
        effects: CardEffects,
    },
    // 亡语
    Deathrattle {
        effects: CardEffects,
    },
    // 狂战（同时攻击目标随从和相邻随从）
    Berserk,
    // 法术伤害+X
    SpellDamage {
        target: Target,
        ampilfy: i32,
    },
    // 切换前后排
    SwapFrontBackHook {
        apply_when_team_swap: bool,
        apply_when_opposite_swap: bool,
        effects: CardEffects,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SpellEffect {
    // 正常使用
    Normal { effects: CardEffects },
    // 前置
    FrontUse { effects: CardEffects },
    // 后置
    BackUse { effects: CardEffects },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Target {
    SelfMinion,
    SelfHero,
    SelectTargetMinion,
    SelectTargetHero,
    SelectTargetEntity,
    OppositeAllMinion,
    OppositeFrontHero,
    OppositeBackHero,
    OppositeAllHero,
    OppositeAllEntity,
    TeamAllMinion,
    TeamFrontHero,
    TeamBackHero,
    TeamAllHero,
    TeamAllEntity,
    AllMinion,
    AllFrontHero,
    AllBackHero,
    AllHero,
    AllEntity,
    JustSummon,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum CardEffect {
    // 造成伤害
    DealDamage {
        target: Target,
        damage: i32,
    },
    // 抽牌
    DrawCard {
        target: Target,
        count: i32,
    },
    // 获得buff
    Buff {
        target: Target,
        buff_type: i32,
        atk_boost: i32,
        hp_boost: i32,
    },
    // 召唤随从
    SummonMinion {
        target: Target,
        minion_code: String,
        summon_side: Side,
    },
    // 切换前后排
    SwapFrontBack {
        swap_team: bool,
        swap_opposite: bool,
    },
    // 恢复生命值
    RecoverHealth {
        target: Target,
        hp: i32,
    },
    // 取消通常法术效果
    PreventNormalEffect,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Side {
    Left,
    Right,
}

pub async fn get_all_cards(db: &mut super::Transaction<'_>) -> Result<Vec<Card>> {
    let mut iter = sqlx::query_as("select rowid, * from card").fetch(&mut db.tx);

    let mut res = Vec::new();
    while let Some(row) = iter.try_next().await? {
        res.push(row);
    }

    Ok(res)
}

pub async fn get_card_by_code(db: &mut super::Transaction<'_>, code: &str) -> Result<Card> {
    Ok(sqlx::query_as("select rowid, * from card where code = ?")
        .bind(code)
        .fetch_one(&mut db.tx)
        .await?)
}
