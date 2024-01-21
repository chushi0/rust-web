use crate::{
    game::Game,
    model::{
        BattlefieldTrait, Buff, Camp, Card, CardModel, Fightline, HeroTrait, Minion, MinionTrait,
        Target,
    },
    player::{Player, PlayerTrait},
};
use datastructure::{SyncHandle, TwoValueEnum};
use std::sync::Arc;
use web_db::hearthstone::{
    CardEffect, MinionEffect, SpecialCardInfo, SpellEffect, Target as ModelTarget,
};

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

#[derive(Debug, Default)]
pub struct PerformResult {
    // 效果是否存在
    pub effect_exist: bool,
    // 需要死亡检查
    pub need_death_check: bool,
    // 取消通常法术效果
    pub prevent_normal_effect: bool,
}

pub fn has_event_type(model: Arc<CardModel>, event_type: EventType) -> bool {
    match query_card_effects(model, event_type) {
        Some(_data) => true,
        None => false,
    }
}

#[derive(Debug, Clone, Copy)]
pub enum Trigger {
    // 由随从触发，用于随从各种扳机（亡语等）
    Minion(u64),
    // 由英雄触发，用于法术
    Hero(u64),
    // 由随从和英雄共同触发，用于战吼
    MinionAndHero { minion: u64, hero: u64 },
}

impl Trigger {
    fn into_camp_target(self) -> Target {
        match self {
            Trigger::Minion(id) => Target::Minion(id),
            Trigger::Hero(id) => Target::Hero(id),
            Trigger::MinionAndHero { hero, .. } => Target::Hero(hero),
        }
    }
}

impl From<Target> for Trigger {
    fn from(value: Target) -> Self {
        match value {
            Target::Minion(id) => Trigger::Minion(id),
            Target::Hero(id) => Trigger::Hero(id),
        }
    }
}

/// 解释卡牌效果
/// TODO: 法术伤害、狂战
#[async_recursion::async_recursion]
pub async fn interpreter(
    game: &mut Game,         // 游戏对象，用于将解释的结果作用到游戏上
    event_type: EventType,   // 触发事件类型
    trigger: Trigger,        // 触发对象
    pointer: Option<Target>, // 触发对象所指定的对象（如果由玩家所指定）
    model: Arc<CardModel>,   // 触发对象的卡牌模型
) -> PerformResult {
    // 递归过深熔断
    if game.interpreter_depth > 20 {
        panic!("interpreter recursion too deep")
    }

    let card_effects = match query_card_effects(model, event_type) {
        Some(data) => data,
        None => return PerformResult::default(),
    };

    let mut result = PerformResult {
        effect_exist: true,
        ..Default::default()
    };
    let mut just_summon = Vec::new();
    game.interpreter_depth += 1;
    for effect in card_effects {
        match effect {
            CardEffect::DealDamage { target, damage } => {
                for target in
                    get_damageable_target(game, trigger, target, pointer, &just_summon).await
                {
                    game.deal_damage(target, damage).await;
                }
                result.need_death_check = true;
            }
            CardEffect::DrawCard { target, count } => {
                for target in get_player_target(game, trigger, target, pointer).await {
                    game.player_draw_card(target, count).await;
                }
                result.need_death_check = true;
            }
            CardEffect::Buff {
                target,
                buff_type,
                atk_boost,
                hp_boost,
            } => {
                let buff = Buff::new(buff_type, atk_boost, hp_boost);
                for target in get_minion_target(game, trigger, target, pointer, &just_summon).await
                {
                    game.buff(target, buff.clone()).await;
                }
                if hp_boost < 0 {
                    result.need_death_check = true;
                }
            }
            CardEffect::SummonMinion {
                target,
                minion_code,
                summon_side: _,
            } => {
                if let Some(player) = get_player_target(game, trigger, target, pointer)
                    .await
                    .first()
                {
                    let camp = player.camp().await;
                    let model = game
                        .query_model_by_code(&minion_code)
                        .await
                        .expect("summon undefined minion");
                    let minion = game.minion_summon(&Card::new_raw(model), camp).await;
                    just_summon.push(minion);
                }
            }
            CardEffect::SwapFrontBack { target } => {
                let swap_players = get_player_target(game, trigger, target, pointer).await;
                game.swap_fightline(swap_players).await;
            }
            CardEffect::RecoverHealth { target, hp } => {
                for target in
                    get_damageable_target(game, trigger, target, pointer, &just_summon).await
                {
                    game.deal_heal(target, hp).await;
                }
            }
            CardEffect::PreventNormalEffect => result.prevent_normal_effect = true,
        }
    }

    game.interpreter_depth -= 1;
    result
}

fn query_card_effects(model: Arc<CardModel>, event_type: EventType) -> Option<Vec<CardEffect>> {
    match &model.card_info.special_card_info {
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

async fn get_damageable_target(
    game: &Game,
    trigger: Trigger,
    target: ModelTarget,
    pointer: Option<Target>,
    just_summon: &Vec<SyncHandle<Minion>>,
) -> Vec<Target> {
    let minions = get_minion_target(game, trigger, target, pointer, just_summon).await;
    let heros = get_hero_target(game, trigger, target, pointer).await;

    minions.into_iter().chain(heros).collect()
}

async fn get_minion_target(
    game: &Game,
    trigger: Trigger,
    target: ModelTarget,
    pointer: Option<Target>,
    just_summon: &Vec<SyncHandle<Minion>>,
) -> Vec<Target> {
    match target {
        ModelTarget::SelfMinion => match trigger {
            Trigger::Minion(id) => vec![Target::Minion(id)],
            Trigger::MinionAndHero { minion, .. } => vec![Target::Minion(minion)],
            _ => vec![],
        },
        ModelTarget::SelectTargetMinion | ModelTarget::SelectTargetEntity => match pointer {
            Some(Target::Minion(id)) => vec![Target::Minion(id)],
            _ => vec![],
        },
        ModelTarget::OppositeAllMinion | ModelTarget::OppositeAllEntity => {
            let Some(camp) = game.get_target_camp(trigger.into_camp_target()).await else {
                return vec![];
            };
            let mut result = Vec::new();
            for minion in game
                .get_battlefield(camp.opposite())
                .await
                .alive_minions()
                .await
            {
                result.push(Target::Minion(minion.uuid().await));
            }

            result
        }
        ModelTarget::TeamAllMinion | ModelTarget::TeamAllEntity => {
            let Some(camp) = game.get_target_camp(trigger.into_camp_target()).await else {
                return vec![];
            };
            let mut result = Vec::new();
            for minion in game.get_battlefield(camp).await.alive_minions().await {
                result.push(Target::Minion(minion.uuid().await));
            }

            result
        }
        ModelTarget::AllMinion | ModelTarget::AllEntity => {
            let mut result = Vec::new();
            for minion in game.get_battlefield(Camp::A).await.alive_minions().await {
                result.push(Target::Minion(minion.uuid().await));
            }
            for minion in game.get_battlefield(Camp::B).await.alive_minions().await {
                result.push(Target::Minion(minion.uuid().await));
            }

            result
        }
        ModelTarget::JustSummon => {
            let mut result = Vec::new();
            for mission in just_summon {
                result.push(Target::Minion(mission.uuid().await))
            }

            result
        }
        ModelTarget::SelfHero
        | ModelTarget::SelectTargetHero
        | ModelTarget::OppositeFrontHero
        | ModelTarget::OppositeBackHero
        | ModelTarget::OppositeAllHero
        | ModelTarget::TeamFrontHero
        | ModelTarget::TeamBackHero
        | ModelTarget::TeamAllHero
        | ModelTarget::AllFrontHero
        | ModelTarget::AllBackHero
        | ModelTarget::AllHero => vec![],
    }
}

async fn get_hero_target(
    game: &Game,
    trigger: Trigger,
    target: ModelTarget,
    pointer: Option<Target>,
) -> Vec<Target> {
    let players = get_player_target(game, trigger, target, pointer).await;
    let mut result = Vec::new();
    for player in players {
        result.push(Target::Hero(player.uuid().await));
    }

    result
}

async fn get_player_target(
    game: &Game,
    trigger: Trigger,
    target: ModelTarget,
    pointer: Option<Target>,
) -> Vec<SyncHandle<Player>> {
    match target {
        ModelTarget::SelfHero => {
            let player_target = match trigger {
                Trigger::Minion(_) => None,
                Trigger::Hero(id) => Some(Target::Hero(id)),
                Trigger::MinionAndHero { hero, .. } => Some(Target::Hero(hero)),
            };
            if let Some(player_target) = player_target {
                match game.get_player_target(player_target).await {
                    Some(player) => vec![player],
                    None => vec![],
                }
            } else {
                vec![]
            }
        }
        ModelTarget::SelectTargetHero | ModelTarget::SelectTargetEntity => {
            let Some(pointer) = pointer else {
                return vec![];
            };
            match game.get_player_target(pointer).await {
                Some(player) => vec![player],
                None => vec![],
            }
        }
        ModelTarget::OppositeFrontHero => {
            let Some(camp) = game.get_target_camp(trigger.into_camp_target()).await else {
                return vec![];
            };
            let mut result = Vec::new();
            for player in game.get_players().await {
                if player.camp().await == camp.opposite()
                    && player.get_hero().await.fightline().await == Fightline::Front
                {
                    result.push(player.clone());
                }
            }
            result
        }
        ModelTarget::OppositeBackHero => {
            let Some(camp) = game.get_target_camp(trigger.into_camp_target()).await else {
                return vec![];
            };
            let mut result = Vec::new();
            for player in game.get_players().await {
                if player.camp().await == camp.opposite()
                    && player.get_hero().await.fightline().await == Fightline::Back
                {
                    result.push(player.clone());
                }
            }
            result
        }
        ModelTarget::OppositeAllHero | ModelTarget::OppositeAllEntity => {
            let Some(camp) = game.get_target_camp(trigger.into_camp_target()).await else {
                return vec![];
            };
            let mut result = Vec::new();
            for player in game.get_players().await {
                if player.camp().await == camp.opposite() {
                    result.push(player.clone());
                }
            }
            result
        }
        ModelTarget::TeamFrontHero => {
            let Some(camp) = game.get_target_camp(trigger.into_camp_target()).await else {
                return vec![];
            };
            let mut result = Vec::new();
            for player in game.get_players().await {
                if player.camp().await == camp
                    && player.get_hero().await.fightline().await == Fightline::Front
                {
                    result.push(player.clone());
                }
            }
            result
        }
        ModelTarget::TeamBackHero => {
            let Some(camp) = game.get_target_camp(trigger.into_camp_target()).await else {
                return vec![];
            };
            let mut result = Vec::new();
            for player in game.get_players().await {
                if player.camp().await == camp
                    && player.get_hero().await.fightline().await == Fightline::Back
                {
                    result.push(player.clone());
                }
            }
            result
        }
        ModelTarget::TeamAllHero | ModelTarget::TeamAllEntity => {
            let Some(camp) = game.get_target_camp(trigger.into_camp_target()).await else {
                return vec![];
            };
            let mut result = Vec::new();
            for player in game.get_players().await {
                if player.camp().await == camp {
                    result.push(player.clone());
                }
            }
            result
        }
        ModelTarget::AllFrontHero => {
            let mut result = Vec::new();
            for player in game.get_players().await {
                if player.get_hero().await.fightline().await == Fightline::Front {
                    result.push(player.clone());
                }
            }
            result
        }
        ModelTarget::AllBackHero => {
            let mut result = Vec::new();
            for player in game.get_players().await {
                if player.get_hero().await.fightline().await == Fightline::Back {
                    result.push(player.clone());
                }
            }
            result
        }
        ModelTarget::AllHero | ModelTarget::AllEntity => game.get_players().await.clone(),
        ModelTarget::SelfMinion
        | ModelTarget::SelectTargetMinion
        | ModelTarget::OppositeAllMinion
        | ModelTarget::TeamAllMinion
        | ModelTarget::AllMinion
        | ModelTarget::JustSummon => vec![],
    }
}
