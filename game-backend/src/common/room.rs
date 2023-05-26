use async_trait::async_trait;
use idl_gen::game_backend::GameType;
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

lazy_static::lazy_static! {
    static ref ROOMS: Mutex<HashMap<RoomKey, SafeRoom>> = Mutex::new(HashMap::new());
    static ref RNG: Mutex<ChaCha8Rng> = Mutex::new(ChaCha8Rng::seed_from_u64(SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs()));
}

const MAX_ROOM_ID: i32 = 1000000;
const MIN_ROOM_ID: i32 = 100000;

pub type SafeRoom = Arc<Mutex<Room>>;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct RoomKey {
    pub game_type: GameType,
    pub room_id: i32,
}

#[derive(Debug)]
pub struct Room {
    room_key: RoomKey,

    public: bool,
    join_players: Vec<RoomPlayer>,
    can_join_room: bool,
    master_user_id: i64,

    biz_room: Arc<Box<dyn BizRoom>>,
}

#[derive(Debug)]
pub struct RoomPlayer {
    user_id: i64,
    ready: bool,
    lost_connection: bool,
}

#[async_trait]
pub trait BizRoom: Send + Sync + Debug {
    async fn do_game_logic(&self, safe_room: SafeRoom);

    async fn check_start(&self, player_count: usize) -> bool;

    async fn max_player_count(&self) -> usize;
}

#[derive(Debug)]
pub enum RoomError {
    RoomNotSetJoinFlag,
    RoomFull,
    RoomHasBeenJoin,
}

pub async fn create_room(game_type: GameType) -> SafeRoom {
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

    let room = Room {
        room_key,
        public: false,
        join_players: vec![],
        can_join_room: true,
        master_user_id: 0,
        biz_room: Arc::new(create_biz_room(game_type)),
    };

    let room = Arc::new(Mutex::new(room));
    rooms.insert(room_key, room.clone());

    room
}

pub async fn get_room(game_type: GameType, room_id: i32) -> Option<SafeRoom> {
    let rooms = ROOMS.lock().await;
    let key = RoomKey { game_type, room_id };

    match rooms.get(&key) {
        Some(room) => Some(room.clone()),
        None => None,
    }
}

pub async fn join_room(safe_room: SafeRoom, user_id: i64) -> Result<(), RoomError> {
    let room = safe_room.clone();
    let mut room = room.lock().await;
    room.join_room(safe_room, user_id).await
}

pub async fn mate_room(game_type: GameType, user_id: i64) -> Result<SafeRoom, RoomError> {
    let rooms = ROOMS.lock().await;
    for entry in &*rooms {
        if entry.0.game_type != game_type {
            continue;
        }

        let room = entry.1.clone();
        let mut room = room.lock().await;

        if room.public && room.can_join_room {
            room.join_room(entry.1.clone(), user_id).await?;
            return Ok(entry.1.clone());
        }
    }
    // 需手动释放以避免 create_room 死锁
    drop(rooms);

    let safe_room = create_room(game_type).await;
    let room = safe_room.clone();
    let mut room = room.lock().await;
    room.public = true;

    room.join_room(safe_room.clone(), user_id).await?;
    Ok(safe_room)
}

fn create_biz_room(game_type: GameType) -> Box<dyn BizRoom> {
    match game_type {
        GameType::Furuyoni => Box::new(crate::biz::furuyoni::room::Room::new()),
    }
}

impl Room {
    pub fn get_room_id(&self) -> i32 {
        self.room_key.room_id
    }

    pub async fn release(&mut self) {
        if self.room_key.room_id == -1 {
            return;
        }

        let mut rooms = ROOMS.lock().await;
        rooms.remove(&self.room_key);
        self.room_key.room_id = -1;
    }

    async fn join_room(&mut self, safe_room: SafeRoom, user_id: i64) -> Result<(), RoomError> {
        if !self.can_join_room {
            return Err(RoomError::RoomNotSetJoinFlag);
        }

        for id in &self.join_players {
            if id.user_id == user_id {
                return Err(RoomError::RoomHasBeenJoin);
            }
        }

        if self.biz_room.max_player_count().await <= self.join_players.len() {
            return Err(RoomError::RoomFull);
        }

        self.join_players.push(RoomPlayer {
            user_id,
            ready: false,
            lost_connection: false,
        });

        self.broadcast_user_change().await;

        if self.join_players.iter().all(|player| player.ready)
            && self.biz_room.check_start(self.join_players.len()).await
        {
            self.can_join_room = false;
            tokio::spawn(room_runner(self.biz_room.clone(), safe_room));
        }

        Ok(())
    }

    async fn broadcast_user_change(&self) {
        // TODO: call bss rpc to notify player change
        // consume error if any error happened
        todo!()
    }

    pub fn set_public(&mut self) {
        self.public = true;
    }

    pub fn set_master_user_id(&mut self, user_id: i64) {
        self.master_user_id = user_id;
    }
}

async fn room_runner(biz_room: Arc<Box<dyn BizRoom>>, safe_room: SafeRoom) {
    biz_room.do_game_logic(safe_room.clone()).await;
    let mut room = safe_room.lock().await;
    room.can_join_room = true;

    room.join_players.retain(|player| !player.lost_connection);

    if room.join_players.is_empty() {
        room.release().await;
    }
}
