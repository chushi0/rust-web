use idl_gen::bss_hearthstone::BattlecryEvent;
use tokio::sync::MutexGuard;
use web_db::hearthstone::{
    CardEffect, CardEffects, MinionEffect, SpecialCardInfo, SpellEffect, Target,
};

use super::{
    game::{Game, Player},
    model::{Buff, Buffable, Camp, Card, Damageable, Minion},
};

pub enum EffectTarget {
    Minion { camp: Camp, minion_id: u64 },
    Hero { camp: Camp, uid: i64 },
}

impl EffectTarget {
    fn get_camp(&self) -> Camp {
        match self {
            EffectTarget::Minion { camp, minion_id: _ } => *camp,
            EffectTarget::Hero { camp, uid: _ } => *camp,
        }
    }
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
    trigger: EffectTarget,
    pointer: Option<EffectTarget>,
    card: Card,
}

#[derive(Debug, Default)]
pub struct PerformResult {
    // 需要死亡检查
    need_death_check: bool,
    // 取消通常法术效果
    prevent_normal_effect: bool,
    // 交换前后排
    my_team_swap: bool,
    oppo_team_swap: bool,
}

impl<'a> Interpreter<'a> {
    pub fn new(
        game: &'a mut Game,
        trigger: EffectTarget,
        pointer: Option<EffectTarget>,
        card: Card,
    ) -> Self {
        Self {
            game,
            trigger,
            pointer,
            card,
        }
    }

    pub async fn perform(
        &mut self,
        event_type: EventType,
        pointer: Option<EffectTarget>,
    ) -> PerformResult {
        let card_effects = match self.query_card_effects(event_type) {
            Some(data) => data,
            None => return PerformResult::default(),
        };

        let mut result = PerformResult::default();
        let mut just_summon = vec![];

        for effect in card_effects {
            match effect {
                CardEffect::DealDamage { target, damage } => self
                    .get_damageable_target(target, &mut just_summon)
                    .await
                    .damage(damage),
                CardEffect::DrawCard { target, count } => todo!(),
                CardEffect::Buff {
                    target,
                    buff_type,
                    atk_boost,
                    hp_boost,
                } => {
                    let buff = Buff::new(self.card.get_model(), buff_type, atk_boost, hp_boost);
                    self.get_buffable_target(target, &mut just_summon)
                        .await
                        .buff(buff)
                }
                CardEffect::SummonMinion {
                    target,
                    minion_code,
                    summon_side,
                } => todo!(),
                CardEffect::SwapFrontBack {
                    swap_team,
                    swap_opposite,
                } => {
                    result.my_team_swap = result.my_team_swap || swap_team;
                    result.oppo_team_swap = result.oppo_team_swap || swap_opposite;
                }
                CardEffect::RecoverHealth { target, hp } => self
                    .get_damageable_target(target, &mut just_summon)
                    .await
                    .heal(hp),
                CardEffect::PreventNormalEffect => result.prevent_normal_effect = true,
            }
        }

        result
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

    async fn get_target<'b>(
        &'b mut self,
        target: Target,
        just_summon: &'b mut Vec<Minion>,
    ) -> TargetResult<'b> {
        match target {
            Target::SelfMinion => {
                if let EffectTarget::Minion { camp, minion_id } = &self.trigger {
                    if let Some(minion) = self.game.get_minion(camp, *minion_id).await {
                        return TargetResult::Minion { minion };
                    }
                }
            }
            Target::SelfHero => {
                if let EffectTarget::Hero { camp: _, uid } = &self.trigger {
                    if let Some(player) = self.game.get_player(*uid).await {
                        return TargetResult::Player { player };
                    }
                }
            }
            Target::SelectTargetMinion => {
                if let Some(EffectTarget::Minion { camp, minion_id }) = &self.pointer {
                    if let Some(minion) = self.game.get_minion(camp, *minion_id).await {
                        return TargetResult::Minion { minion };
                    }
                }
            }
            Target::SelectTargetHero => {
                if let Some(EffectTarget::Hero { camp: _, uid }) = &self.pointer {
                    if let Some(player) = self.game.get_player(*uid).await {
                        return TargetResult::Player { player };
                    }
                }
            }
            Target::SelectTargetEntity => {
                if let Some(pointer) = &self.pointer {
                    match pointer {
                        EffectTarget::Minion { camp, minion_id } => {
                            if let Some(minion) = self.game.get_minion(camp, *minion_id).await {
                                return TargetResult::Minion { minion };
                            }
                        }
                        EffectTarget::Hero { camp: _, uid } => {
                            if let Some(player) = self.game.get_player(*uid).await {
                                return TargetResult::Player { player };
                            }
                        }
                    }
                }
            }
            Target::OppositeAllMinion => {
                let camp = self.trigger.get_camp().opposite();
                return TargetResult::List {
                    list: self
                        .game
                        .get_minions(&camp)
                        .await
                        .into_iter()
                        .map(|minion| TargetResult::Minion { minion })
                        .collect(),
                };
            }
            Target::OppositeFrontHero => {
                let camp = self.trigger.get_camp().opposite();
                let player = self
                    .game
                    .get_player_by_camp_pos(&camp, super::model::Fightline::Front)
                    .await;
                if let Some(player) = player {
                    return TargetResult::Player { player };
                }
            }
            Target::OppositeBackHero => {
                let camp = self.trigger.get_camp().opposite();
                let player = self
                    .game
                    .get_player_by_camp_pos(&camp, super::model::Fightline::Back)
                    .await;
                if let Some(player) = player {
                    return TargetResult::Player { player };
                }
            }
            Target::OppositeAllHero => {
                let camp = self.trigger.get_camp().opposite();
                return TargetResult::List {
                    list: self
                        .game
                        .get_player_by_camp(&camp)
                        .await
                        .into_iter()
                        .map(|player| TargetResult::Player { player })
                        .collect(),
                };
            }
            Target::OppositeAllEntity => {
                let camp = self.trigger.get_camp().opposite();
                let minions = self.game.get_minions(&camp).await;
                let players = self.game.get_player_by_camp(&camp).await;
                return TargetResult::List {
                    list: vec![
                        TargetResult::List {
                            list: minions
                                .into_iter()
                                .map(|minion| TargetResult::Minion { minion })
                                .collect(),
                        },
                        TargetResult::List {
                            list: players
                                .into_iter()
                                .map(|player| TargetResult::Player { player })
                                .collect(),
                        },
                    ],
                };
            }
            Target::TeamAllMinion => {
                let camp = self.trigger.get_camp();
                return TargetResult::List {
                    list: self
                        .game
                        .get_minions(&camp)
                        .await
                        .into_iter()
                        .map(|minion| TargetResult::Minion { minion })
                        .collect(),
                };
            }
            Target::TeamFrontHero => {
                let camp = self.trigger.get_camp();
                let player = self
                    .game
                    .get_player_by_camp_pos(&camp, super::model::Fightline::Front)
                    .await;
                if let Some(player) = player {
                    return TargetResult::Player { player };
                }
            }
            Target::TeamBackHero => {
                let camp = self.trigger.get_camp();
                let player = self
                    .game
                    .get_player_by_camp_pos(&camp, super::model::Fightline::Back)
                    .await;
                if let Some(player) = player {
                    return TargetResult::Player { player };
                }
            }
            Target::TeamAllHero => {
                let camp = self.trigger.get_camp();
                return TargetResult::List {
                    list: self
                        .game
                        .get_player_by_camp(&camp)
                        .await
                        .into_iter()
                        .map(|player| TargetResult::Player { player })
                        .collect(),
                };
            }
            Target::TeamAllEntity => {
                let camp = self.trigger.get_camp();
                let minions = self.game.get_minions(&camp).await;
                let players = self.game.get_player_by_camp(&camp).await;
                return TargetResult::List {
                    list: vec![
                        TargetResult::List {
                            list: minions
                                .into_iter()
                                .map(|minion| TargetResult::Minion { minion })
                                .collect(),
                        },
                        TargetResult::List {
                            list: players
                                .into_iter()
                                .map(|player| TargetResult::Player { player })
                                .collect(),
                        },
                    ],
                };
            }
            Target::AllMinion => {
                return TargetResult::List {
                    list: self
                        .game
                        .get_all_minions()
                        .await
                        .into_iter()
                        .map(|minion| TargetResult::Minion { minion })
                        .collect(),
                }
            }
            Target::AllFrontHero => {
                let player = self
                    .game
                    .get_player_by_pos(super::model::Fightline::Front)
                    .await;
                if let Some(player) = player {
                    return TargetResult::Player { player };
                }
            }
            Target::AllBackHero => {
                let player = self
                    .game
                    .get_player_by_pos(super::model::Fightline::Back)
                    .await;
                if let Some(player) = player {
                    return TargetResult::Player { player };
                }
            }
            Target::AllHero => {
                return TargetResult::List {
                    list: self
                        .game
                        .get_all_players()
                        .await
                        .into_iter()
                        .map(|player| TargetResult::Player { player })
                        .collect(),
                };
            }
            Target::AllEntity => {
                let minions: Vec<MutexGuard<'_, Minion>> = self.game.get_all_minions().await;
                let players = self.game.get_all_players().await;
                return TargetResult::List {
                    list: vec![
                        TargetResult::List {
                            list: minions
                                .into_iter()
                                .map(|minion| TargetResult::Minion { minion })
                                .collect(),
                        },
                        TargetResult::List {
                            list: players
                                .into_iter()
                                .map(|player| TargetResult::Player { player })
                                .collect(),
                        },
                    ],
                };
            }
            Target::JustSummon => {
                return TargetResult::List {
                    list: just_summon
                        .iter_mut()
                        .map(|minion| TargetResult::MutMinion { minion: minion })
                        .collect(),
                }
            }
        };

        TargetResult::None
    }

    async fn get_damageable_target<'b>(
        &'b mut self,
        target: Target,
        just_summon: &'b mut Vec<Minion>,
    ) -> DamageableResult<'b> {
        self.get_target(target, just_summon).await.into()
    }

    async fn get_buffable_target<'b>(
        &'b mut self,
        target: Target,
        just_summon: &'b mut Vec<Minion>,
    ) -> BuffableResult<'b> {
        self.get_target(target, just_summon).await.into()
    }
}

enum TargetResult<'a> {
    Minion { minion: MutexGuard<'a, Minion> },
    MutMinion { minion: &'a mut Minion },
    Player { player: MutexGuard<'a, Player> },
    List { list: Vec<TargetResult<'a>> },
    None,
}

enum DamageableResult<'a> {
    Minion { minion: MutexGuard<'a, Minion> },
    MutMinion { minion: &'a mut Minion },
    Player { player: MutexGuard<'a, Player> },
    List { list: Vec<DamageableResult<'a>> },
    None,
}

impl<'a> Damageable for DamageableResult<'a> {
    fn damage(&mut self, damage: i32) {
        match self {
            DamageableResult::Minion { minion } => minion.damage(damage),
            DamageableResult::MutMinion { minion } => minion.damage(damage),
            DamageableResult::Player { player } => player.damage(damage),
            DamageableResult::List { list } => {
                for it in list {
                    it.damage(damage)
                }
            }
            DamageableResult::None => {}
        }
    }
}

impl<'a> From<TargetResult<'a>> for DamageableResult<'a> {
    fn from(value: TargetResult<'a>) -> Self {
        match value {
            TargetResult::Minion { minion } => DamageableResult::Minion { minion },
            TargetResult::MutMinion { minion } => DamageableResult::MutMinion { minion },
            TargetResult::Player { player } => DamageableResult::Player { player },
            TargetResult::List { list } => DamageableResult::List {
                list: list.into_iter().map(|it| it.into()).collect(),
            },
            TargetResult::None => DamageableResult::None,
        }
    }
}

enum BuffableResult<'a> {
    Minion { minion: MutexGuard<'a, Minion> },
    MutMinion { minion: &'a mut Minion },
    List { list: Vec<BuffableResult<'a>> },
    None,
}

impl<'a> Buffable for BuffableResult<'a> {
    fn buff(&mut self, buff: Buff) {
        match self {
            BuffableResult::Minion { minion } => minion.buff(buff),
            BuffableResult::MutMinion { minion } => minion.buff(buff),
            BuffableResult::List { list } => {
                for it in list {
                    it.buff(buff.clone())
                }
            }
            BuffableResult::None => {}
        }
    }
}

impl<'a> From<TargetResult<'a>> for BuffableResult<'a> {
    fn from(value: TargetResult<'a>) -> Self {
        match value {
            TargetResult::Minion { minion } => BuffableResult::Minion { minion },
            TargetResult::MutMinion { minion } => BuffableResult::MutMinion { minion },
            TargetResult::Player { player: _ } => BuffableResult::None,
            TargetResult::List { list } => BuffableResult::List {
                list: list.into_iter().map(|it| it.into()).collect(),
            },
            TargetResult::None => BuffableResult::None,
        }
    }
}
