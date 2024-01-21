use crate::{
    api::{self, GameNotifier, NopGameNotifier, PlayerDrawCard},
    interpreter::{has_event_type, interpreter, EventType, Trigger},
    model::{
        Battlefield, BattlefieldTrait, Buff, Buffable, Camp, Card, CardModel, CardPool, Damageable,
        Fightline, HeroTrait, Minion, MinionTrait, Target, UuidGenerator,
    },
    player::{AIPlayerBehavior, Player, PlayerBehavior, PlayerStartingAction, PlayerTrait},
};
use datastructure::{AsyncIter, Concurrency, CycleArrayVector, SyncHandle, TwoValueEnum};
use futures_util::stream::StreamExt;
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
    pub custom_id: i64,
    pub max_hero_hp: u32,
    pub behavior: Arc<dyn PlayerBehavior>,
    pub camp: Option<Camp>,
    pub deck: HashMap<i64, u32>, // key: card_id, value: count
}

impl Default for PlayerConfig {
    fn default() -> Self {
        Self {
            custom_id: 0,
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

        let players: Vec<_> = {
            // 先为这些玩家分配uuid
            let players: Vec<_> = players
                .into_iter()
                .map(|player| (uuid_increase.gen(), player))
                .collect();

            // 玩家uuid通知
            for (uuid, config) in &players {
                game_notifier.player_uuid(*uuid, config.custom_id);
            }

            // 分队
            let mut camp_a = Vec::new();
            let mut camp_b = Vec::new();
            let mut camp_undefined = Vec::new();
            for player in players {
                match player.1.camp {
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
            assert!(camp_undefined.is_empty(), "all player should has camp");

            let player_with_camp: Vec<_> = camp_a
                .into_iter()
                .map(|(player_uuid, config)| (player_uuid, config, Camp::A))
                .chain(
                    camp_b
                        .into_iter()
                        .map(|(player_uuid, config)| (player_uuid, config, Camp::B)),
                )
                .collect();

            // 通知
            for (player_uuid, _, camp) in &player_with_camp {
                game_notifier.camp_decide(*player_uuid, *camp)
            }
            game_notifier.flush_at_starting().await;

            // 生成玩家对象
            // 我们需要在此处先生成玩家对象，以便管理手牌、牌库资源
            // 对于前后排，由于尚未确定，暂时将其置为 前排，会在稍后将其修改为正确的值
            let mut players = Vec::with_capacity(4);
            for (uuid, config, camp) in player_with_camp {
                players.push(
                    Player::new(uuid, &card_pool, config, camp, Fightline::Front, &mut rng).await,
                );
            }

            // 准备阶段玩家初始化
            Self::player_starting_init(game_notifier.clone(), &mut players, &mut rng).await;

            players
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

    async fn player_starting_init<Rng: rand::Rng + Send>(
        game_notifier: Arc<dyn GameNotifier>,
        players: &mut [SyncHandle<Player>],
        rng: &mut Rng,
    ) {
        // 抽取起始卡牌
        for player in &mut *players {
            player.draw_starting_card().await;
            let cards = player.hand_cards().await;
            let mut card_refs = Vec::new();
            for card in &cards {
                card_refs.push(card.get().await.clone());
            }
            game_notifier.starting_card(player.uuid().await, card_refs);
        }

        // 通知
        game_notifier.flush_at_starting().await;

        // 前后排决定 & 更换手牌
        {
            struct StartingPlayer {
                index: usize,
                camp: Camp,
                replace_cards: Option<Vec<usize>>,
                position: Position,
            }
            #[derive(Debug, Clone, Copy)]
            enum Position {
                None,
                Perfer(Fightline),
                Lock(Fightline),
            }
            enum FutureResult {
                Timeout,
                PlayerStartingAction {
                    index: usize,
                    action: Option<PlayerStartingAction>,
                },
            }

            let mut starting_players: HashMap<_, _> = (0..4)
                .map(|index| {
                    (
                        index,
                        players.get(index).expect("player should exist").clone(),
                    )
                })
                .async_map(|(index, player)| async move {
                    (
                        player.uuid().await,
                        StartingPlayer {
                            index,
                            camp: player.camp().await,
                            replace_cards: None,
                            position: Position::None,
                        },
                    )
                })
                .await
                .collect();

            // 超时时间设定
            let prepare_timeout = async {
                tokio::time::sleep(std::time::Duration::from_secs(60)).await;
                FutureResult::Timeout
            };
            // 玩家输入
            let player_action_spawner = |index: usize| {
                let player = players
                    .get(index)
                    .expect("starting action should have player")
                    .clone();
                async move {
                    let action = player.next_starting_action().await;
                    FutureResult::PlayerStartingAction { index, action }
                }
            };

            let mut concurrency = Concurrency::new();
            concurrency.submit_task(prepare_timeout);
            for index in 0..4 {
                concurrency.submit_task(player_action_spawner(index));
            }

            // 轮询检查输入
            while let Some(result) = concurrency.next().await {
                match result {
                    FutureResult::Timeout => break,
                    FutureResult::PlayerStartingAction {
                        index: _,
                        action: None, // 明确表示不会有后续输入
                    } => {
                        // 只剩下超时任务了，那就不用轮询了
                        if concurrency.remain_task() == 1 {
                            break;
                        }
                    }
                    FutureResult::PlayerStartingAction {
                        index,
                        action: Some(action),
                    } => {
                        let mut player = players
                            .get(index)
                            .expect("starting player should exist")
                            .clone();
                        let uuid = player.uuid().await;
                        let starting_player = starting_players
                            .get_mut(&uuid)
                            .expect("uuid player should exist");
                        match action {
                            PlayerStartingAction::SwapStartingCards { cards_index } => {
                                if starting_player.replace_cards.is_none() {
                                    player.swap_starting_card(&cards_index, rng).await;
                                    let cards = player.hand_cards().await;
                                    let mut card_refs = Vec::new();
                                    for card in &cards {
                                        card_refs.push(card.get().await.clone());
                                    }
                                    game_notifier.change_starting_card(
                                        player.uuid().await,
                                        &cards_index,
                                        card_refs,
                                    );
                                    starting_player.replace_cards = Some(cards_index);
                                }
                            }
                            PlayerStartingAction::ChooseFightline { fightline } => {
                                if !matches!(starting_player.position, Position::Lock(_)) {
                                    starting_player.position = match fightline {
                                        Some(fightline) => Position::Perfer(fightline),
                                        None => Position::None,
                                    };
                                    game_notifier.fightline_choose(uuid, fightline);
                                }
                            }
                            PlayerStartingAction::LockFightline => {
                                // 同队其他玩家不能锁定
                                // 为了满足借用检查器，我们先拿到我们需要的参数，丢弃starting_player。在完成检查后，再重新获取starting_player
                                let camp = starting_player.camp;
                                if !starting_players.iter().any(|(player_uuid, player)| {
                                    uuid != *player_uuid
                                        && player.camp == camp
                                        && matches!(player.position, Position::Lock(_))
                                }) {
                                    let starting_player = starting_players.get_mut(&uuid).expect(
                                        "get player should success because we just used it before",
                                    );
                                    if let Position::Perfer(fightline) = starting_player.position {
                                        starting_player.position = Position::Lock(fightline);
                                        game_notifier.fightline_lock(uuid, fightline);
                                    }
                                }
                            }
                            PlayerStartingAction::UnlockFightline => {
                                if let Position::Lock(fightline) = starting_player.position {
                                    starting_player.position = Position::Perfer(fightline);
                                    game_notifier.fightline_unlock(uuid);
                                }
                            }
                        };
                        game_notifier.flush_at_starting().await;
                        // 玩家可能仍然有输入操作，将任务重新置入stream中，等待玩家输入
                        concurrency.submit_task(player_action_spawner(index));
                    }
                }
            }

            // 清理
            for player in &mut *players {
                player.finish_starting_action().await;
            }

            // 该决定位置了
            // 同一队中，优先级: Lock > Perfer > None
            // 如果都处于Perfer，则随机挑选一个
            // 如果都处于None，则随机分配
            for camp in [Camp::A, Camp::B] {
                let mut starting_players = starting_players
                    .iter()
                    .filter(|(_, player)| player.camp == camp);
                let (_, player1) = starting_players.next().expect("player should exist");
                let (_, player2) = starting_players.next().expect("player should exist");
                assert!(
                    starting_players.next().is_none(),
                    "only two players should in one camp"
                );

                match (
                    (player1.index, player1.position),
                    (player2.index, player2.position),
                ) {
                    // double lock
                    ((_, Position::Lock(_)), (_, Position::Lock(_))) => {
                        unreachable!("double lock has been filtered when input");
                    }

                    // lock
                    ((id1, Position::Lock(f)), (id2, _)) | ((id2, _), (id1, Position::Lock(f))) => {
                        players[id1].change_fightline_to(f).await;
                        players[id2].change_fightline_to(f.opposite()).await;
                    }

                    // double perfer
                    ((id1, Position::Perfer(f1)), (id2, Position::Perfer(f2))) => {
                        let random_choice = if rng.gen_bool(0.5) {
                            [(id1, f1), (id2, f1.opposite())]
                        } else {
                            [(id2, f2), (id1, f2.opposite())]
                        };

                        for (id, f) in random_choice {
                            players[id].change_fightline_to(f).await;
                        }
                    }

                    // perfer
                    ((id1, Position::Perfer(f)), (id2, _))
                    | ((id2, _), (id1, Position::Perfer(f))) => {
                        players[id1].change_fightline_to(f).await;
                        players[id2].change_fightline_to(f.opposite()).await;
                    }

                    // double none
                    ((id1, Position::None), (id2, Position::None)) => {
                        let f = if rng.gen_bool(0.5) {
                            Fightline::Front
                        } else {
                            Fightline::Back
                        };

                        players[id1].change_fightline_to(f).await;
                        players[id2].change_fightline_to(f.opposite()).await;
                    }
                }
            }

            // 位置信息发送给客户端
            for player in &*players {
                game_notifier.fightline_decide(
                    player.uuid().await,
                    player.get_hero().await.fightline().await,
                );
            }
            game_notifier.flush_at_starting().await;
        }

        // 按照行动顺序排序
        let mut sort_keys: Vec<_> = players
            .iter()
            .enumerate()
            .async_map(|(index, player)| async move {
                (
                    index,
                    player.camp().await,
                    player.get_hero().await.fightline().await,
                )
            })
            .await
            .collect();

        sort_keys.sort_by(|(_, c1, f1), (_, c2, f2)| {
            match (f1, f2) {
                (Fightline::Front, Fightline::Back) => return std::cmp::Ordering::Greater,
                (Fightline::Back, Fightline::Front) => return std::cmp::Ordering::Less,
                _ => (),
            };
            match (c1, c2) {
                (Camp::A, Camp::B) => return std::cmp::Ordering::Less,
                (Camp::B, Camp::A) => return std::cmp::Ordering::Greater,
                _ => (),
            };

            unreachable!()
        });

        let sort_keys: Vec<_> = sort_keys.into_iter().map(|(index, _, _)| index).collect();
        let mut new_players = Vec::with_capacity(4);
        for i in 0..4 {
            new_players.push(players[sort_keys[i]].clone())
        }
        for (i, player) in new_players.iter_mut().enumerate().take(4) {
            players[i] = player.clone();
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
        false
    }

    async fn run_turn(&mut self) {
        if self.turn > 1000 {
            panic!("too many rounds that no winner is determined");
        }
        self.turn += 1;
        let current_turn: &TurnAction = &self.turn_actions;
        let current_turn = current_turn.clone();
        self.game_notifier.new_turn(match &current_turn {
            TurnAction::PlayerTurn(player) => api::TurnAction::PlayerTurn(player.uuid().await),
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
            .player_mana_change(player.uuid().await, player.mana().await);
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
            .values()
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
                        .all_minions()
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
                    let hero_uuid = player.uuid().await;
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
                        .alive_minions()
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
                    let hero_uuid = player.uuid().await;
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
                    player.uuid().await,
                    PlayerDrawCard::Draw(card.get().await.clone()),
                ),
                crate::player::DrawCardResult::Fire(card) => self.game_notifier.player_draw_card(
                    player.uuid().await,
                    PlayerDrawCard::Fire(card.get().await.clone()),
                ),
                crate::player::DrawCardResult::Tired(tired) => {
                    self.game_notifier
                        .player_draw_card(player.uuid().await, PlayerDrawCard::Tired(tired));
                    self.deal_damage(Target::Hero(player.uuid().await), tired.into())
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
        self.game_notifier
            .player_use_card(player.uuid().await, card.clone(), cost_mana);

        match model.card_type() {
            CardType::Minion => {
                self.minion_summon_with_battlecry(&card, player.camp().await, player, target)
                    .await;
            }
            CardType::Spell => {
                let trigger = Target::Hero(player.uuid().await).into();
                let mut need_death_check = false;
                let result = match player.get_hero().await.fightline().await {
                    Fightline::Front => {
                        interpreter(self, EventType::FrontUse, trigger, target, model.clone()).await
                    }
                    Fightline::Back => {
                        interpreter(self, EventType::BackUse, trigger, target, model.clone()).await
                    }
                };
                if result.need_death_check {
                    need_death_check = true;
                }
                if !result.prevent_normal_effect {
                    let result =
                        interpreter(self, EventType::NormalSpell, trigger, target, model).await;
                    if result.need_death_check {
                        need_death_check = true;
                    }
                }

                if need_death_check {
                    self.minion_death_check().await;
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
                player.uuid().await,
                player.get_hero().await.fightline().await,
            );

            match player.camp().await {
                Camp::A => camp_a_swap = true,
                Camp::B => camp_b_swap = true,
            }
        }

        let mut need_death_check = false;

        for minion in self.get_battlefield(Camp::A).await.alive_minions().await {
            let result = interpreter(
                self,
                EventType::SwapFrontBack {
                    team: camp_a_swap,
                    opposite: camp_b_swap,
                },
                Trigger::Minion(minion.uuid().await),
                None,
                minion.model().await,
            )
            .await;

            if result.need_death_check {
                need_death_check = true;
            }
        }

        for minion in self.get_battlefield(Camp::B).await.alive_minions().await {
            let result = interpreter(
                self,
                EventType::SwapFrontBack {
                    team: camp_b_swap,
                    opposite: camp_a_swap,
                },
                Trigger::Minion(minion.uuid().await),
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
        player: SyncHandle<Player>,
        target: Option<Target>,
    ) -> SyncHandle<Minion> {
        let minion = self.minion_summon(card, camp).await;

        if has_event_type(card.model().clone(), EventType::Battlecry) {
            self.game_notifier
                .minion_battlecry(minion.get().await.clone());

            let result = interpreter(
                self,
                EventType::Battlecry,
                Trigger::MinionAndHero {
                    minion: minion.uuid().await,
                    hero: player.uuid().await,
                },
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

        if let Some(minion) = self.get_minion_target(target).await {
            self.deal_damage(target, attacker_mission.atk().await.into())
                .await;
            self.deal_damage(Target::Minion(attacker), minion.atk().await.into())
                .await;
            self.minion_death_check().await;
        };

        if self.get_player_target(target).await.is_some() {
            self.deal_damage(target, attacker_mission.atk().await.into())
                .await;
        };
    }

    pub(crate) async fn minion_death_check(&mut self) {
        loop {
            let mut death_minions = Vec::new();

            for minion in self.get_battlefield(Camp::A).await.alive_check().await {
                let uuid = minion.uuid().await;
                death_minions.push((minion, uuid));
            }
            for minion in self.get_battlefield(Camp::B).await.alive_check().await {
                let uuid = minion.uuid().await;
                death_minions.push((minion, uuid));
            }

            // 排序，按照uuid顺序（入场顺序）结算亡语
            death_minions.sort_by(|(_, id1), (_, id2)| id1.cmp(id2));
            let death_minions: Vec<SyncHandle<Minion>> = death_minions
                .into_iter()
                .map(|(minion, _)| minion)
                .collect();

            for minion in &death_minions {
                self.game_notifier.minion_death(minion.get().await.clone());
            }

            let mut finish = true;
            for minion in death_minions {
                if has_event_type(minion.model().await.clone(), EventType::Deathrattle) {
                    self.game_notifier
                        .minion_deathrattle(minion.get().await.clone());

                    let result = interpreter(
                        self,
                        EventType::Deathrattle,
                        Trigger::Minion(minion.uuid().await),
                        None,
                        minion.model().await,
                    )
                    .await;

                    if result.need_death_check {
                        finish = false;
                    }
                }
            }

            self.get_battlefield(Camp::A)
                .await
                .remove_death_minions()
                .await;
            self.get_battlefield(Camp::B)
                .await
                .remove_death_minions()
                .await;

            if finish {
                break;
            }
        }
    }

    pub(crate) async fn deal_damage(&mut self, target: Target, damage: i64) {
        self.game_notifier.deal_damage(target, damage);
        if let Some(mut minion) = self.get_minion_target(target).await {
            minion.damage(damage).await;
        };

        if let Some(player) = self.get_player_target(target).await {
            player.get_hero().await.damage(damage).await;
        };
    }

    pub(crate) async fn deal_heal(&mut self, target: Target, heal: i64) {
        self.deal_damage(target, -heal).await;
    }

    pub(crate) async fn buff(&mut self, target: Target, buff: Buff) {
        self.game_notifier.buff(target, buff.clone());
        if let Some(mut minion) = self.get_minion_target(target).await {
            minion.buff(buff).await;
        };
    }
}

impl Game {
    pub fn players(&self) -> Vec<SyncHandle<Player>> {
        self.players.clone()
    }

    pub async fn battlefield_minions(&self, camp: Camp) -> Vec<SyncHandle<Minion>> {
        let battlefield = self.get_battlefield(camp).await;
        battlefield.alive_minions().await
    }
}
