/// 角色
#[derive(Debug)]
pub struct Character {
    /// 角色id
    pub id: i64,
    /// 主要攻击距离
    pub primary_attack_distance: Vec<u8>,
    /// 次要攻击距离
    pub secondary_attack_distance: Vec<u8>,
    pub special_effect: SpecialEffect,
    /// 关联状态
    pub status: Vec<i64>,
}

/// 卡牌
#[derive(Debug)]
pub enum Card {
    /// 攻击牌
    Attack(AttackCardData),
    /// 行动牌
    Movement(MovementCardData),
    /// 附于牌
    Delaying(DelayingCardCard),
}

#[derive(Debug)]
pub struct BaseCardData {
    /// id
    pub id: i64,
    /// 是否是全力牌
    pub all_out: bool,
    /// 是否是应对牌
    pub trap: bool,
    /// 如果是王牌的话，消耗多少气
    pub mp_cost: Option<u8>,
    pub special_effect: SpecialEffect,
}

#[derive(Debug)]
pub struct AttackCardData {
    pub base_data: BaseCardData,
    /// 范围
    pub range: Vec<u8>,
    /// 对护甲伤害
    pub damage_shield: Option<u8>,
    /// 对命伤害
    pub damage_health: Option<u8>,
    /// 是否可被应对
    pub can_trap: bool,
}

#[derive(Debug)]
pub struct MovementCardData {
    pub base_data: BaseCardData,
}

#[derive(Debug)]
pub struct DelayingCardCard {
    pub base_data: BaseCardData,
    /// 冷却时间
    pub cooldown_time: u8,
}

#[derive(Debug)]
pub struct Status {
    id: i64,
}

#[derive(Debug)]
pub struct SpecialEffect {}

#[derive(Debug, Clone, Copy)]
pub enum SpecialEffectTrigger {
    /// 卡牌使用效果
    CardEffect,
    /// 卡牌使用后效果
    AfterCardUse,
}

impl SpecialEffect {
    pub async fn trigger(
        &self,
        trigger: SpecialEffectTrigger,
        game: super::room::Room,
        trigger_user_id: i64,
    ) -> bool {
        todo!()
    }
}
