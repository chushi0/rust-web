use crate::{
    api::{self, GameNotifier, NopGameNotifier, PlayerDrawCard},
    interpreter::{has_event_type, interpreter, EventType},
    model::{
        Battlefield, BattlefieldTrait, Buff, Buffable, Camp, Card, CardModel, CardPool, Damageable,
        Fightline, HeroTrait, Minion, MinionTrait, Target, UuidGenerator,
    },
    player::{AIPlayerBehavior, Player, PlayerBehavior, PlayerTrait},
};
use datastructure::{CycleArrayVector, SyncHandle};
use rand::{rngs::StdRng, seq::SliceRandom, Rng, SeedableRng};
use std::{collections::HashMap, sync::Arc};
use web_db::hearthstone::CardType;

#[derive(Debug)]
pub struct Config {
    pub card_pool: CardPool,
    pub seed: [u8; 32],
    pub game_notifier: Arc<dyn GameNotifier>,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            card_pool: HashMap::new(),
            seed: rand::thread_rng().gen(),
            game_notifier: Arc::new(NopGameNotifier),
        }
    }
}

#[derive(Debug)]
pub struct PlayerConfig {
    pub max_hero_hp: u32,
    pub behavior: Arc<dyn PlayerBehavior>,
    pub camp: Option<Camp>,
    pub deck: HashMap<i64, u32>, // key: card_id, value: count
}

impl Default for PlayerConfig {
    fn default() -> Self {
        Self {
            max_hero_hp: 30,
            behavior: Arc::new(AIPlayerBehavior::default()),
            camp: Default::default(),
            deck: Default::default(),
        }
    }
}

#[derive(Debug)]
pub struct Game {
    game_notifier: Arc<dyn GameNotifier>,
    card_pool: CardPool,
    _rng: StdRng,
    uuid_generator: UuidGenerator,

    players: Vec<SyncHandle<Player>>,
    battlefield: HashMap<Camp, SyncHandle<Battlefield>>,
    turn: u32,
    turn_actions: CycleArrayVector<TurnAction>,

    pub(crate) interpreter_depth: u8,
}

#[derive(Debug)]
pub struct GameResult {}

#[derive(Debug, Clone)]
pub enum TurnAction {
    PlayerTurn(SyncHandle<Player>),
    SwapFightline,
}

impl Game {
    pub async fn new(config: Config, players: Vec<PlayerConfig>) -> Self {
        assert!(
            players.len() == 4,
            "player count should be 4, but it's {}",
            players.len()
        );
        let game_notifier = config.game_notifier;
        let mut rng = StdRng::from_seed(config.seed);
        let card_pool = config.card_pool;
        let mut uuid_increase = UuidGenerator::new();

        let players = {
            // 分队
            let mut camp_a = Vec::new();
            let mut camp_b = Vec::new();
            let mut camp_undefined = Vec::new();
            for player in players {
                match player.camp {
                    Some(Camp::A) => camp_a.push(player),
                    Some(Camp::B) => camp_b.push(player),
                    None => camp_undefined.push(player),
                }
            }

            assert!(
                camp_a.len() <= 2 || camp_b.len() <= 2,
                "each team can only have a maximum of two players"
            );

            camp_undefined.shuffle(&mut rng);
            while camp_a.len() < 2 {
                camp_a.push(camp_undefined.remove(0));
            }
            while camp_b.len() < 2 {
                camp_b.push(camp_undefined.remove(0));
            }

            // 初始前后排确定
            camp_a.shuffle(&mut rng);
            camp_b.shuffle(&mut rng);

            // 生成玩家对象
            // 注意：此处按照玩家行动顺序生成对象，以便后面生成行动顺序时直接使用
            vec![
                Player::new(
                    uuid_increase.gen(),
                    &card_pool,
                    camp_a.remove(0),
                    Camp::A,
                    Fightline::Back,
                    &mut rng,
                )
                .await,
                Player::new(
                    uuid_increase.gen(),
                    &card_pool,
                    camp_b.remove(0),
                    Camp::B,
                    Fightline::Back,
                    &mut rng,
                )
                .await,
                Player::new(
                    uuid_increase.gen(),
                    &card_pool,
                    camp_a.remove(0),
                    Camp::A,
                    Fightline::Front,
                    &mut rng,
                )
                .await,
                Player::new(
                    uuid_increase.gen(),
                    &card_pool,
                    camp_b.remove(0),
                    Camp::B,
                    Fightline::Front,
                    &mut rng,
                )
                .await,
            ]
        };

        let battlefield = [(Camp::A, Battlefield::new()), (Camp::B, Battlefield::new())]
            .into_iter()
            .collect();
        let turn = 0;
        let turn_actions = CycleArrayVector::new(
            players
                .iter()
                .map(|player| TurnAction::PlayerTurn(player.clone()))
                .chain([TurnAction::SwapFightline].into_iter())
                .collect(),
        );

        Game {
            game_notifier,
            card_pool,
            _rng: rng,
            uuid_generator: uuid_increase,
            players,
            battlefield,
            turn,
            turn_actions,
            interpreter_depth: 0,
        }
    }

    pub async fn run(mut self) -> GameResult {
        loop {
            self.run_turn().await;

            if self.is_game_end().await {
                break;
            }
        }

        GameResult {}
    }

    async fn is_game_end(&self) -> bool {
        for player in &self.players {
            if player.get_hero().await.hp().await <= 0 {
                return true;
            }
        }
        return false;
    }

    async fn run_turn(&mut self) {
        if self.turn > 1000 {
            panic!("too many rounds that no winner is determined");
        }
        self.turn += 1;
        let current_turn: &TurnAction = &self.turn_actions;
        let current_turn = current_turn.clone();
        self.game_notifier.new_turn(match &current_turn {
            TurnAction::PlayerTurn(player) => {
                api::TurnAction::PlayerTurn(player.get_hero().await.uuid().await)
            }
            TurnAction::SwapFightline => api::TurnAction::SwapFightline,
        });
        log::info!("run turn: #{} {current_turn:?}", self.turn);
        match current_turn {
            TurnAction::PlayerTurn(player) => self.player_turn(player).await,
            TurnAction::SwapFightline => self.swap_fightline_turn().await,
        }
        self.turn_actions.move_to_next();
    }

    async fn player_turn(&mut self, mut player: SyncHandle<Player>) {
        log::info!("start player turn: {player:?}");

        player.turn_reset_mana().await;
        self.game_notifier
            .player_mana_change(player.get_hero().await.uuid().await, player.mana().await);
        self.player_draw_card(player.clone(), 1).await;
        self.game_notifier.flush(self).await;
        loop {
            if self.is_game_end().await {
                return;
            }
            let player_action = player.next_action(self).await;
            log::info!("player action: {player_action:?}");
            match player_action {
                crate::player::PlayerTurnAction::PlayCard { hand_index, target } => {
                    self.player_use_card(player.clone(), hand_index, target)
                        .await;
                }
                crate::player::PlayerTurnAction::MinionAttack { attacker, target } => {
                    self.minion_attack(attacker, target).await;
                }
                crate::player::PlayerTurnAction::EndTurn => break,
            }
            self.game_notifier.flush(self).await;
        }

        log::info!("end player turn: {player:?}");
    }

    async fn swap_fightline_turn(&mut self) {
        log::info!("start swap fightline turn");

        self.swap_fightline(self.players.clone()).await;
        self.game_notifier.flush(self).await;

        log::info!("end swap fightline turn");
    }
}

impl Game {
    pub(crate) async fn query_model_by_code(&self, code: &str) -> Option<Arc<CardModel>> {
        let models: Vec<_> = self
            .card_pool
            .iter()
            .map(|(_, model)| model)
            .filter(|model| model.card.code == code)
            .collect();

        models.first().map(|model| (*model).clone())
    }

    pub(crate) async fn get_target_camp(&self, target: Target) -> Option<Camp> {
        match target {
            Target::Minion(id) => {
                for camp in [Camp::A, Camp::B] {
                    for minion in self
                        .battlefield
                        .get(&camp)
                        .expect("battlefield camp_a not found")
                        .minions()
                        .await
                    {
                        if minion.uuid().await == id {
                            return Some(camp);
                        }
                    }
                }
            }
            Target::Hero(id) => {
                for player in &self.players {
                    let hero_uuid = player.get_hero().await.uuid().await;
                    if hero_uuid == id {
                        return Some(player.camp().await);
                    }
                }
            }
        }

        None
    }

    pub(crate) async fn get_minion_target(&self, target: Target) -> Option<SyncHandle<Minion>> {
        match target {
            Target::Minion(id) => {
                for camp in [Camp::A, Camp::B] {
                    for minion in self
                        .battlefield
                        .get(&camp)
                        .expect("battlefield camp_a not found")
                        .minions()
                        .await
                    {
                        if minion.uuid().await == id {
                            return Some(minion.clone());
                        }
                    }
                }
            }
            Target::Hero(_) => (),
        }

        None
    }

    pub(crate) async fn get_player_target(&self, target: Target) -> Option<SyncHandle<Player>> {
        match target {
            Target::Minion(_) => (),
            Target::Hero(id) => {
                for player in &self.players {
                    let hero_uuid = player.get_hero().await.uuid().await;
                    if hero_uuid == id {
                        return Some(player.clone());
                    }
                }
            }
        }

        None
    }

    pub(crate) async fn get_battlefield(&self, camp: Camp) -> SyncHandle<Battlefield> {
        self.battlefield
            .get(&camp)
            .expect("get battlefield should not be none")
            .clone()
    }

    pub(crate) async fn get_players(&self) -> &Vec<SyncHandle<Player>> {
        &self.players
    }

    pub(crate) async fn player_draw_card(&mut self, mut player: SyncHandle<Player>, count: u32) {
        for _i in 0..count {
            match player.draw_card().await {
                crate::player::DrawCardResult::Draw(card) => self.game_notifier.player_draw_card(
                    player.get_hero().await.uuid().await,
                    PlayerDrawCard::Draw(card.get().await.clone()),
                ),
                crate::player::DrawCardResult::Fire(card) => self.game_notifier.player_draw_card(
                    player.get_hero().await.uuid().await,
                    PlayerDrawCard::Fire(card.get().await.clone()),
                ),
                crate::player::DrawCardResult::Tired(tired) => {
                    self.game_notifier.player_draw_card(
                        player.get_hero().await.uuid().await,
                        PlayerDrawCard::Tired(tired),
                    );
                    self.deal_damage(
                        Target::Hero(player.get_hero().await.uuid().await),
                        tired.into(),
                    )
                    .await
                }
            }
        }
    }

    pub(crate) async fn player_use_card(
        &mut self,
        mut player: SyncHandle<Player>,
        index: usize,
        target: Option<Target>,
    ) {
        let Some(card) = player.remove_hand_card(index).await else {
            return;
        };
        let card = card.get().await;
        let model = card.model();

        let cost_mana = model.card.mana_cost;
        player.cost_mana(cost_mana).await;
        self.game_notifier.player_use_card(
            player.get_hero().await.uuid().await,
            card.clone(),
            cost_mana,
        );

        match model.card_type() {
            CardType::Minion => {
                self.minion_summon_with_battlecry(&*card, player.camp().await, target)
                    .await;
            }
            CardType::Spell => {
                let trigger = Target::Hero(player.get_hero().await.uuid().await);
                let result = match player.get_hero().await.fightline().await {
                    Fightline::Front => {
                        interpreter(self, EventType::FrontUse, trigger, target, model.clone()).await
                    }
                    Fightline::Back => {
                        interpreter(self, EventType::BackUse, trigger, target, model.clone()).await
                    }
                };
                if !result.prevent_normal_effect {
                    interpreter(self, EventType::NormalSpell, trigger, target, model).await;
                }
            }
        };

        self.game_notifier.player_card_effect_end();
    }

    pub(crate) async fn swap_fightline(&mut self, players: Vec<SyncHandle<Player>>) {
        let mut camp_a_swap = false;
        let mut camp_b_swap = false;
        for player in &players {
            player.get_hero().await.swap_fightline().await;
            self.game_notifier.player_swap_fightline(
                player.get_hero().await.uuid().await,
                player.get_hero().await.fightline().await,
            );

            match player.camp().await {
                Camp::A => camp_a_swap = true,
                Camp::B => camp_b_swap = true,
            }
        }

        let mut need_death_check = false;

        for minion in self.get_battlefield(Camp::A).await.minions().await {
            let result = interpreter(
                self,
                EventType::SwapFrontBack {
                    team: camp_a_swap,
                    opposite: camp_b_swap,
                },
                Target::Minion(minion.uuid().await),
                None,
                minion.model().await,
            )
            .await;

            if result.need_death_check {
                need_death_check = true;
            }
        }

        for minion in self.get_battlefield(Camp::B).await.minions().await {
            let result = interpreter(
                self,
                EventType::SwapFrontBack {
                    team: camp_b_swap,
                    opposite: camp_a_swap,
                },
                Target::Minion(minion.uuid().await),
                None,
                minion.model().await,
            )
            .await;

            if result.need_death_check {
                need_death_check = true;
            }
        }

        if need_death_check {
            self.minion_death_check().await;
        }
    }

    pub(crate) async fn minion_summon(&mut self, card: &Card, camp: Camp) -> SyncHandle<Minion> {
        let uuid = self.uuid_generator.gen();
        let minion = Minion::new(card.model(), uuid);

        self.get_battlefield(camp)
            .await
            .summon_minion(minion.clone())
            .await;

        self.game_notifier
            .minion_summon(minion.get().await.clone(), camp);

        minion
    }

    pub(crate) async fn minion_summon_with_battlecry(
        &mut self,
        card: &Card,
        camp: Camp,
        target: Option<Target>,
    ) -> SyncHandle<Minion> {
        let minion = self.minion_summon(card, camp).await;

        if has_event_type(card.model().clone(), EventType::Battlecry) {
            self.game_notifier
                .minion_battlecry(minion.get().await.clone());

            let result = interpreter(
                self,
                EventType::Battlecry,
                Target::Minion(minion.uuid().await),
                target,
                card.model(),
            )
            .await;

            if result.need_death_check {
                self.minion_death_check().await;
            }
        }

        minion
    }

    pub(crate) async fn minion_attack(&mut self, attacker: u64, target: Target) {
        let Some(attacker_mission) = self.get_minion_target(Target::Minion(attacker)).await else {
            return;
        };

        self.game_notifier
            .minion_attack(attacker_mission.get().await.clone(), target);

        match self.get_minion_target(target).await {
            Some(minion) => {
                self.deal_damage(target, attacker_mission.atk().await.into())
                    .await;
                self.deal_damage(Target::Minion(attacker), minion.atk().await.into())
                    .await;
                self.minion_death_check().await;
            }
            None => {}
        };

        match self.get_player_target(target).await {
            Some(_) => {
                self.deal_damage(target, attacker_mission.atk().await.into())
                    .await;
            }
            None => {}
        };
    }

    pub(crate) async fn minion_death_check(&mut self) {
        loop {
            let mut death_minions = Vec::new();

            for minion in self
                .get_battlefield(Camp::A)
                .await
                .remove_death_minions()
                .await
            {
                death_minions.push(minion);
            }

            for minion in self
                .get_battlefield(Camp::B)
                .await
                .remove_death_minions()
                .await
            {
                death_minions.push(minion);
            }

            let mut finish = true;
            for minion in death_minions {
                if has_event_type(minion.model().await.clone(), EventType::Deathrattle) {
                    self.game_notifier
                        .minion_deathrattle(minion.get().await.clone());

                    let result = interpreter(
                        self,
                        EventType::Deathrattle,
                        Target::Minion(minion.uuid().await),
                        None,
                        minion.model().await,
                    )
                    .await;

                    if result.need_death_check {
                        finish = false;
                    }
                }
            }

            if finish {
                break;
            }
        }
    }

    pub(crate) async fn deal_damage(&mut self, target: Target, damage: i64) {
        self.game_notifier.deal_damage(target, damage);
        match self.get_minion_target(target).await {
            Some(mut minion) => {
                minion.damage(damage).await;
            }
            None => {}
        };

        match self.get_player_target(target).await {
            Some(player) => {
                player.get_hero().await.damage(damage).await;
            }
            None => {}
        };
    }

    pub(crate) async fn deal_heal(&mut self, target: Target, heal: i64) {
        self.deal_damage(target, -heal).await;
    }

    pub(crate) async fn buff(&mut self, target: Target, buff: Buff) {
        self.game_notifier.buff(target, buff.clone());
        match self.get_minion_target(target).await {
            Some(mut minion) => {
                minion.buff(buff).await;
            }
            None => {}
        };
    }
}

impl Game {
    pub fn players(&self) -> Vec<SyncHandle<Player>> {
        self.players.clone()
    }

    pub async fn battlefield_minions(&self, camp: Camp) -> Vec<SyncHandle<Minion>> {
        let battlefield = self.get_battlefield(camp).await;
        battlefield.minions().await
    }
}
