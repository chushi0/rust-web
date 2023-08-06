use idl_gen::bss_hearthstone::BattlecryEvent;
use web_db::hearthstone::{
    CardEffect, CardEffects, MinionEffect, SpecialCardInfo, SpellEffect, Target,
};

use super::{
    game::Game,
    model::{Camp, Card},
};

pub enum Trigger {
    Minion { camp: Camp, minion_id: u64 },
    Hero { camp: Camp, uid: i64 },
}

pub enum EventType {
    // 随从
    Battlecry,                                    // 战吼
    Deathrattle,                                  // 亡语
    SwapFrontBack { team: bool, opposite: bool }, // 切换前后排
    // 法术
    NormalSpell, // 正常施法
    FrontUse,    // 前置
    BackUse,     // 后置
}

pub struct Interpreter<'a> {
    game: &'a mut Game,
    trigger: Trigger,
    card: Card,
}

#[derive(Debug, Default)]
pub struct PerformResult {
    // 需要死亡检查
    need_death_check: bool,
    // 取消通常法术效果
    prevent_normal_effect: bool,
    // 交换前后排
    my_team_swap: i32,
    oppo_team_swap: i32,
}

impl<'a> Interpreter<'a> {
    pub fn new(game: &'a mut Game, trigger: Trigger, card: Card) -> Self {
        Self {
            game,
            trigger,
            card,
        }
    }

    pub fn perform(&mut self, event_type: EventType, target: Option<Trigger>) -> PerformResult {
        let card_effects = match self.query_card_effects(event_type) {
            Some(data) => data,
            None => return PerformResult::default(),
        };

        for effect in card_effects {
            match effect {
                CardEffect::DealDamage { target, damage } => todo!(),
                CardEffect::DrawCard { target, count } => todo!(),
                CardEffect::Buff {
                    target,
                    atk_burst,
                    hp_burst,
                } => todo!(),
                CardEffect::SummonMinion {
                    target,
                    minion_code,
                    summon_side,
                } => todo!(),
                CardEffect::SwapFrontBack {
                    swap_team,
                    swap_opposite,
                } => todo!(),
                CardEffect::RecoverHealth { target, hp } => todo!(),
                CardEffect::PreventNormalEffect => todo!(),
            }
        }

        PerformResult::default()
    }

    fn query_card_effects(&self, event_type: EventType) -> Option<Vec<CardEffect>> {
        match &self.card.get_model().card_info.special_card_info {
            SpecialCardInfo::Minion(info) => {
                for effect in &info.effects {
                    match effect {
                        MinionEffect::Battlecry { effects } => {
                            if let EventType::Battlecry = event_type {
                                return Some(effects.clone());
                            }
                        }
                        MinionEffect::Deathrattle { effects } => {
                            if let EventType::Deathrattle = event_type {
                                return Some(effects.clone());
                            }
                        }
                        MinionEffect::SwapFrontBackHook {
                            apply_when_team_swap,
                            apply_when_opposite_swap,
                            effects,
                        } => {
                            if let EventType::SwapFrontBack { team, opposite } = event_type {
                                if (*apply_when_team_swap && team)
                                    || (*apply_when_opposite_swap && opposite)
                                {
                                    return Some(effects.clone());
                                }
                            }
                        }

                        _ => {}
                    }
                }
            }
            SpecialCardInfo::Spell(info) => {
                for effect in &info.effects {
                    match effect {
                        SpellEffect::Normal { effects } => {
                            if let EventType::NormalSpell = event_type {
                                return Some(effects.clone());
                            }
                        }
                        SpellEffect::FrontUse { effects } => {
                            if let EventType::FrontUse = event_type {
                                return Some(effects.clone());
                            }
                        }
                        SpellEffect::BackUse { effects } => {
                            if let EventType::BackUse = event_type {
                                return Some(effects.clone());
                            }
                        }
                    }
                }
            }
        }
        None
    }
}
