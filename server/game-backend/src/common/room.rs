use crate::rpc;
use async_trait::async_trait;
use idl_gen::bss_websocket::SendRoomChatRequest;
use idl_gen::bss_websocket::SendRoomCommonChangeRequest;
use idl_gen::bss_websocket_client::BoxProtobufPayload;
use idl_gen::game_backend::GameType;
use log::info;
use log::warn;
use pilota::FastStr;
use rand::distributions::Uniform;
use rand::prelude::Distribution;
use rand::SeedableRng;
use rand_chacha::ChaCha8Rng;
use std::collections::HashMap;
use std::fmt::Debug;
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};
use std::vec;
use tokio::sync::Mutex;
use volo_grpc::{Code, Status};

lazy_static::lazy_static! {
    static ref ROOMS: Mutex<HashMap<RoomKey, SafeRoom>> = Mutex::new(HashMap::new());
    static ref RNG: Mutex<ChaCha8Rng> = Mutex::new(ChaCha8Rng::seed_from_u64(SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs()));
}

pub const MAX_ROOM_ID: i32 = 1000000;
pub const MIN_ROOM_ID: i32 = 100000;

pub type SafeRoom = Arc<Mutex<Room>>;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct RoomKey {
    pub game_type: GameType,
    pub room_id: i32,
}

pub struct Room {
    room_key: RoomKey,

    /// 是否公开。
    /// 如果一个房间被标记为公开，则可以被匹配到
    public: bool,
    /// 房间内的玩家
    join_players: Vec<RoomPlayer>,
    /// 标注是否允许玩家加入或退出。
    /// 会在开始游戏时设置为true，并在游戏结束时设置为false
    player_lock: bool,

    /// 具体的游戏房间（根据游戏不同而有不同实现）
    biz_room: Arc<Box<dyn BizRoom>>,

    /// 随机数生成器
    rng: ChaCha8Rng,
}

#[derive(Debug)]
pub struct RoomPlayer {
    /// 用户id
    user_id: i64,
    // 是否已准备
    ready: bool,
    // 是否断线（会在游戏结束时移出房间）
    lost_connection: bool,
    // 进入房间的附加信息（由具体游戏解析）
    extra_data: Option<Vec<u8>>,
}

#[async_trait]
pub trait BizRoom: Send + Sync {
    /// 游戏主逻辑
    async fn do_game_logic(&self, safe_room: SafeRoom);

    /// 检查游戏人数是否满足开始条件
    async fn check_start(&self, player_count: usize) -> bool;

    /// 游戏最大支持同时加入人数
    async fn max_player_count(&self) -> usize;

    async fn player_input(&self, user_id: i64, data: BoxProtobufPayload);
}

#[derive(Debug, Clone, Copy)]
pub enum RoomError {
    RoomPlayerLock,
    RoomFull,
    RoomHasBeenJoin,
    PlayerNotInRoom,
    InternalError,
}

pub async fn create_room(game_type: GameType) -> SafeRoom {
    info!("start to create room...");
    let range = Uniform::new(MIN_ROOM_ID, MAX_ROOM_ID);

    let mut rooms = ROOMS.lock().await;
    let mut rng = RNG.lock().await;

    let room_key = (|| loop {
        let id = range.sample(&mut *rng);
        let key = RoomKey {
            game_type,
            room_id: id,
        };

        if !rooms.contains_key(&key) {
            return key;
        }
    })();

    let seed = Uniform::new(u64::MIN, u64::MAX).sample(&mut *rng);
    let room_rng = ChaCha8Rng::seed_from_u64(seed);

    let room = Room {
        room_key,
        public: false,
        join_players: vec![],
        player_lock: false,
        biz_room: Arc::new(create_biz_room(game_type)),
        rng: room_rng,
    };

    let room = Arc::new(Mutex::new(room));
    rooms.insert(room_key, room.clone());
    info!("room created: {room_key:?}");

    room
}

pub async fn get_room(game_type: GameType, room_id: i32) -> Option<SafeRoom> {
    let rooms = ROOMS.lock().await;
    let key = RoomKey { game_type, room_id };

    rooms.get(&key).cloned()
}

pub async fn join_room(
    safe_room: SafeRoom,
    user_id: i64,
    extra_data: Option<Vec<u8>>,
) -> Result<(), RoomError> {
    let room = safe_room.clone();
    let mut room = room.lock().await;
    room.join_room(safe_room, user_id, extra_data).await
}

pub async fn mate_room(
    game_type: GameType,
    user_id: i64,
    extra_data: Option<Vec<u8>>,
) -> Result<SafeRoom, RoomError> {
    {
        let rooms = ROOMS.lock().await;
        for entry in &*rooms {
            if entry.0.game_type != game_type {
                continue;
            }

            let room = entry.1.clone();
            let mut room = room.lock().await;

            if room.public && room.can_join(user_id).await {
                room.join_room_unchecked(entry.1.clone(), user_id, extra_data)
                    .await;
                return Ok(entry.1.clone());
            }
        }
    }

    let safe_room = create_room(game_type).await;
    let room = safe_room.clone();
    let mut room = room.lock().await;
    room.public = true;

    room.join_room(safe_room.clone(), user_id, extra_data)
        .await?;
    Ok(safe_room)
}

pub async fn leave_room(safe_room: SafeRoom, user_id: i64) -> Result<(), RoomError> {
    let mut room = safe_room.lock().await;
    room.leave_room(user_id).await
}

pub async fn set_player_ready(
    safe_room: SafeRoom,
    user_id: i64,
    ready: bool,
) -> Result<(), RoomError> {
    let room = safe_room.clone();
    let mut room = room.lock().await;
    room.set_player_ready(safe_room, user_id, ready).await
}

pub async fn room_chat(
    safe_room: SafeRoom,
    sender_user_id: i64,
    receiver_user_indexes: &[i32],
    content: FastStr,
) -> Result<(), RoomError> {
    let room = safe_room.lock().await;
    let room_id = room.room_key.room_id;
    let game_type = room.room_key.game_type as i32;
    let players = room.players();

    let sender_user_index = players
        .iter()
        .position(|player| player.user_id == sender_user_id)
        .ok_or(RoomError::PlayerNotInRoom)? as i32;

    let receiver_user_ids = receiver_user_indexes
        .iter()
        .filter(|index| **index >= 0 && (**index as usize) < players.len())
        .map(|index| players[*index as usize].user_id)
        .collect();

    let req = SendRoomChatRequest {
        room_id,
        game_type,
        sender_user_id,
        sender_user_index,
        receiver_user_indexes: receiver_user_indexes.to_vec(),
        receiver_user_ids,
        content,
    };

    rpc::bss::client().send_room_chat(req).await.map_err(|e| {
        warn!("send room chat error: {e:?}");
        RoomError::InternalError
    })?;

    Ok(())
}

fn create_biz_room(game_type: GameType) -> Box<dyn BizRoom> {
    match game_type {
        GameType::Furuyoni => unimplemented!(),
        GameType::Hearthstone => Box::new(crate::biz::hearthstone::room::Room::new()),
    }
}

impl Room {
    pub fn get_room_id(&self) -> i32 {
        self.room_key.room_id
    }

    /// 释放房间，从全局房间中删除。
    /// 在游戏结束后或所有玩家退出房间后，必须调用此函数释放，否则会造成内存泄漏。
    /// 释放后，对象不应再次使用。
    pub async fn release(&mut self) {
        if self.room_key.room_id == -1 {
            return;
        }

        info!("start to release room...");
        let mut rooms = ROOMS.lock().await;
        rooms.remove(&self.room_key);
        info!("room released: {:?}", self.room_key);
        self.room_key.room_id = -1;
    }

    /// 判断玩家是否可以加入当前房间
    async fn can_join(&self, user_id: i64) -> bool {
        if self.player_lock {
            return false;
        }

        if self
            .join_players
            .iter()
            .any(|player| player.user_id == user_id)
        {
            return false;
        }

        if self.biz_room.max_player_count().await <= self.join_players.len() {
            return false;
        }

        true
    }

    /// 玩家加入房间，如果无法加入房间则返回错误
    async fn join_room(
        &mut self,
        safe_room: SafeRoom,
        user_id: i64,
        extra_data: Option<Vec<u8>>,
    ) -> Result<(), RoomError> {
        if self.player_lock {
            return Err(RoomError::RoomPlayerLock);
        }

        if self
            .join_players
            .iter()
            .any(|player| player.user_id == user_id)
        {
            return Err(RoomError::RoomHasBeenJoin);
        }

        if self.biz_room.max_player_count().await <= self.join_players.len() {
            return Err(RoomError::RoomFull);
        }

        self.join_room_unchecked(safe_room, user_id, extra_data)
            .await;

        Ok(())
    }

    /// 离开房间。如果当前不能离开，或玩家不在房间内，返回错误。
    async fn leave_room(&mut self, user_id: i64) -> Result<(), RoomError> {
        if self.player_lock {
            return Err(RoomError::RoomPlayerLock);
        }

        let player_count = self.join_players.len();
        self.join_players.retain(|player| player.user_id != user_id);
        // 没有删除任何玩家
        if player_count == self.join_players.len() {
            return Err(RoomError::PlayerNotInRoom);
        }

        self.broadcast_user_change().await;

        // 没有玩家了，释放房间
        if self.join_players.is_empty() {
            self.release().await;
        }

        Ok(())
    }

    /// 玩家加入房间，不检查是否满足加入条件。
    /// 务必在调用此函数前调用 [`can_join`] 函数判断是否可以加入
    async fn join_room_unchecked(
        &mut self,
        safe_room: SafeRoom,
        user_id: i64,
        extra_data: Option<Vec<u8>>,
    ) {
        debug_assert!(self.can_join(user_id).await);

        self.join_players.push(RoomPlayer {
            user_id,
            ready: false,
            lost_connection: false,
            extra_data,
        });

        self.broadcast_user_change().await;

        self.start_game_if_satisfy(safe_room).await;
    }

    pub fn pack_room_players(&self) -> Vec<idl_gen::bss_websocket::RoomPlayer> {
        let mut players = Vec::new();
        for i in 0..self.join_players.len() {
            let player = &self.join_players[i];

            players.push(idl_gen::bss_websocket::RoomPlayer {
                user_id: player.user_id,
                index: i as i32,
                ready: player.ready,
                online: !player.lost_connection,
                master: false,
            });
        }

        players
    }

    async fn broadcast_user_change(&self) {
        let user_ids: Vec<_> = self
            .join_players
            .iter()
            .map(|player| player.user_id)
            .collect();
        let room_players = self.pack_room_players();

        if user_ids.is_empty() || room_players.is_empty() {
            return;
        }

        let request = SendRoomCommonChangeRequest {
            user_ids,
            game_type: self.room_key.game_type as i32,
            room_id: self.room_key.room_id,
            room_players,
            public: self.public,
        };

        let resp = rpc::bss::client().send_room_common_change(request).await;
        match resp {
            Ok(resp) => {
                let failed_user_ids = &resp.get_ref().failed_user_ids;
                if !failed_user_ids.is_empty() {
                    log::warn!("send room common change error for user ids: {failed_user_ids:?}")
                }
            }
            Err(err) => log::warn!("send room common change error: {err:?}"),
        }
    }

    fn get_player(&mut self, user_id: i64) -> Option<&mut RoomPlayer> {
        self.join_players
            .iter_mut()
            .find(|player| player.user_id == user_id)
    }

    pub fn players(&self) -> &Vec<RoomPlayer> {
        &self.join_players
    }

    /// 设置玩家是否准备
    async fn set_player_ready(
        &mut self,
        safe_room: SafeRoom,
        user_id: i64,
        ready: bool,
    ) -> Result<(), RoomError> {
        if self.player_lock {
            return Err(RoomError::RoomPlayerLock);
        }

        let player = self.get_player(user_id);
        let player = match player {
            Some(player) => player,
            None => return Err(RoomError::PlayerNotInRoom),
        };

        if player.ready == ready {
            return Ok(());
        }

        player.ready = ready;
        self.broadcast_user_change().await;

        if ready {
            self.start_game_if_satisfy(safe_room).await;
        }

        Ok(())
    }

    async fn start_game_if_satisfy(&mut self, safe_room: SafeRoom) {
        log::debug!("room {:?} checking start condition...", self.room_key);
        if self.join_players.iter().all(|player| player.ready)
            && self.biz_room.check_start(self.join_players.len()).await
        {
            self.player_lock = true;
            log::debug!("room {:?} has satisfy the start condition", self.room_key);
            tokio::spawn(room_runner(self.biz_room.clone(), safe_room));
        }
        log::debug!("room {:?} checked start condition", self.room_key);
    }

    #[cfg(test)]
    pub fn biz_room(&self) -> Arc<Box<dyn BizRoom>> {
        self.biz_room.clone()
    }

    /// 设置房间公开
    pub async fn set_public(&mut self) {
        if self.public {
            return;
        }
        self.public = true;

        self.broadcast_user_change().await;
    }

    /// 房主user_id
    ///
    /// 房主是房间内加入房间最早的玩家
    pub fn master_user_id(&self) -> i64 {
        match self.join_players.first() {
            Some(player) => player.user_id,
            None => 0,
        }
    }

    pub fn random(&mut self, min: i32, max: i32) -> i32 {
        Uniform::new(min, max).sample(&mut self.rng)
    }

    pub fn new_rng(&mut self) -> ChaCha8Rng {
        let seed = Uniform::new(u64::MIN, u64::MAX).sample(&mut self.rng);
        ChaCha8Rng::seed_from_u64(seed)
    }
}

impl RoomPlayer {
    pub fn get_user_id(&self) -> i64 {
        self.user_id
    }

    pub fn get_extra_data(&self) -> &Option<Vec<u8>> {
        &self.extra_data
    }
}

impl From<RoomError> for Status {
    fn from(value: RoomError) -> Self {
        let code = match value {
            RoomError::RoomPlayerLock => Code::FailedPrecondition,
            RoomError::RoomFull => Code::FailedPrecondition,
            RoomError::RoomHasBeenJoin => Code::AlreadyExists,
            RoomError::PlayerNotInRoom => Code::NotFound,
            RoomError::InternalError => Code::Internal,
        };
        let msg = format!("{value:?}");
        Status::new(code, msg)
    }
}

/// 房间主逻辑处理
async fn room_runner(biz_room: Arc<Box<dyn BizRoom>>, safe_room: SafeRoom) {
    biz_room.do_game_logic(safe_room.clone()).await;
    let mut room = safe_room.lock().await;
    room.player_lock = false;

    room.join_players.retain(|player| !player.lost_connection);
    room.join_players
        .iter_mut()
        .for_each(|player| player.ready = false);

    if room.join_players.is_empty() {
        room.release().await;
    }
}

#[cfg(test)]
pub async fn force_start_game(safe_room: SafeRoom) {
    let biz_room = safe_room.lock().await.biz_room().clone();
    room_runner(biz_room, safe_room).await;
}
