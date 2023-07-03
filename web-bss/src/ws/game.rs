use super::WsBiz;
use crate::rpc;
use crate::util::protobuf::pack_message;
use anyhow::Result;
use idl_gen::bss_websocket_client::*;
use idl_gen::game_backend;
use log::warn;
use protobuf::Message;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use web_db::user::{query_user, update_user_login_time, QueryUserParam, User};
use web_db::{begin_tx, create_connection, RDS};

pub struct GameBiz {
    con: Arc<super::WsCon>,
    user: Option<User>,
    room: Option<RoomKey>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct RoomKey {
    pub user_id: i64,
    pub game_type: i32,
    pub room_id: i32,
}

lazy_static::lazy_static! {
    static ref ROOMS: RwLock<HashMap<RoomKey, Arc<super::WsCon>>> = RwLock::new(HashMap::new());
}

impl GameBiz {
    pub fn create(con: super::WsCon) -> GameBiz {
        GameBiz {
            con: Arc::new(con),
            user: None,
            room: None,
        }
    }
}

#[async_trait]
impl WsBiz for GameBiz {
    async fn on_binary_message(&mut self, msg: &[u8]) {
        if let Err(e) = self.do_binary_message(msg).await {
            warn!("handle binary message error: {e}")
        }
    }

    async fn on_close(&mut self) {
        if let Some(room) = &self.room {
            let mut rooms = ROOMS.write().await;
            rooms.remove(room);
        }
    }
}

impl GameBiz {
    async fn do_binary_message(&mut self, msg: &[u8]) -> Result<()> {
        let paylod = BoxProtobufPayload::parse_from_bytes(msg)?;
        if paylod.name == ClientLoginRequest::NAME {
            let req = ClientLoginRequest::parse_from_bytes(paylod.payload.as_slice())?;
            let resp = self.client_login(req).await.unwrap_or_else(|e| {
                warn!("error when handle client_login: {e}");
                let mut resp = ClientLoginResponse::new();
                resp.code = 500;
                resp.message = "internal error".to_string();
                resp
            });
            self.con.send_binary(pack_message(resp)?)?;
        } else if paylod.name == CreateRoomRequest::NAME {
            let req = CreateRoomRequest::parse_from_bytes(paylod.payload.as_slice())?;
            let resp = self.create_room(req).await.unwrap_or_else(|e| {
                warn!("error when handle create_room: {e}");
                let mut resp = JoinRoomResponse::new();
                resp.code = 500;
                resp.message = "internal error".to_string();
                resp
            });
            self.con.send_binary(pack_message(resp)?)?;
        } else if paylod.name == JoinRoomRequest::NAME {
            let req = JoinRoomRequest::parse_from_bytes(paylod.payload.as_slice())?;
            let resp = self.join_room(req).await.unwrap_or_else(|e| {
                warn!("error when handle create_room: {e}");
                let mut resp = JoinRoomResponse::new();
                resp.code = 500;
                resp.message = "internal error".to_string();
                resp
            });
            self.con.send_binary(pack_message(resp)?)?;
        } else if paylod.name == MateRoomRequest::NAME {
            let req = MateRoomRequest::parse_from_bytes(paylod.payload.as_slice())?;
            let resp = self.mate_room(req).await.unwrap_or_else(|e| {
                warn!("error when handle create_room: {e}");
                let mut resp = JoinRoomResponse::new();
                resp.code = 500;
                resp.message = "internal error".to_string();
                resp
            });
            self.con.send_binary(pack_message(resp)?)?;
        }

        Ok(())
    }

    async fn client_login(&mut self, req: ClientLoginRequest) -> Result<ClientLoginResponse> {
        if self.user.is_some() {
            let mut resp = ClientLoginResponse::new();
            resp.code = 1001;
            resp.message = "user has login".to_string();
            return Ok(resp);
        }

        let mut conn = create_connection(RDS::User).await?;
        let mut tx = begin_tx(&mut conn).await?;

        let user = query_user(
            &mut tx,
            QueryUserParam::ByAccount {
                account: req.account.clone(),
            },
        )
        .await?;

        if user.password != req.password {
            let mut resp = ClientLoginResponse::new();
            resp.code = 1002;
            resp.message = "wrong password".to_string();
            return Ok(resp);
        }

        update_user_login_time(&mut tx, user.rowid).await?;
        tx.commit().await?;

        self.user = Some(user);
        let mut resp = ClientLoginResponse::new();
        resp.code = 0;
        resp.message = "success".to_string();
        Ok(resp)
    }

    async fn create_room(&mut self, req: CreateRoomRequest) -> Result<JoinRoomResponse> {
        let user = match &self.user {
            Some(user) => user,
            None => {
                let mut resp = JoinRoomResponse::new();
                resp.code = 401;
                resp.message = "not login".to_string();
                return Ok(resp);
            }
        };

        if self.room.is_some() {
            let mut resp = JoinRoomResponse::new();
            resp.code = 1001;
            resp.message = "has join room".to_string();
            return Ok(resp);
        }

        let mut rpc_req = game_backend::JoinRoomRequest::default();
        rpc_req.user_id = user.rowid;
        rpc_req.game_type = match game_backend::GameType::try_from(req.game_type) {
            Ok(v) => v,
            Err(_) => {
                let mut resp = JoinRoomResponse::new();
                resp.code = 1002;
                resp.message = "game not supported".to_string();
                return Ok(resp);
            }
        };
        rpc_req.strategy = game_backend::JoinRoomStrategy::Create;
        rpc_req.public = Some(req.init_public);
        rpc_req.extra_data = clone_extra_data(req.extra_data);
        let rpc_resp = rpc::game::client().join_room(rpc_req).await?.into_inner();

        let room_key = RoomKey {
            user_id: user.rowid,
            game_type: req.game_type,
            room_id: rpc_resp.room_id,
        };
        self.room = Some(room_key);
        let mut rooms = ROOMS.write().await;
        rooms.insert(room_key, self.con.clone());
        drop(rooms);

        let mut resp = JoinRoomResponse::new();
        resp.code = 0;
        resp.message = "success".to_string();
        resp.room_id = Some(rpc_resp.room_id);
        Ok(resp)
    }

    async fn join_room(&mut self, req: JoinRoomRequest) -> Result<JoinRoomResponse> {
        let user = match &self.user {
            Some(user) => user,
            None => {
                let mut resp = JoinRoomResponse::new();
                resp.code = 401;
                resp.message = "not login".to_string();
                return Ok(resp);
            }
        };

        if self.room.is_some() {
            let mut resp = JoinRoomResponse::new();
            resp.code = 1001;
            resp.message = "has join room".to_string();
            return Ok(resp);
        }

        let mut rpc_req = game_backend::JoinRoomRequest::default();
        rpc_req.user_id = user.rowid;
        rpc_req.game_type = match game_backend::GameType::try_from(req.game_type) {
            Ok(v) => v,
            Err(_) => {
                let mut resp = JoinRoomResponse::new();
                resp.code = 1002;
                resp.message = "game not supported".to_string();
                return Ok(resp);
            }
        };
        rpc_req.strategy = game_backend::JoinRoomStrategy::Join;
        rpc_req.room_id = Some(req.room_id);
        rpc_req.extra_data = clone_extra_data(req.extra_data);
        let rpc_resp = rpc::game::client().join_room(rpc_req).await?.into_inner();

        let room_key = RoomKey {
            user_id: user.rowid,
            game_type: req.game_type,
            room_id: rpc_resp.room_id,
        };
        self.room = Some(room_key);
        let mut rooms = ROOMS.write().await;
        rooms.insert(room_key, self.con.clone());
        drop(rooms);

        let mut resp = JoinRoomResponse::new();
        resp.code = 0;
        resp.message = "success".to_string();
        resp.room_id = Some(rpc_resp.room_id);
        Ok(resp)
    }

    async fn mate_room(&mut self, req: MateRoomRequest) -> Result<JoinRoomResponse> {
        let user = match &self.user {
            Some(user) => user,
            None => {
                let mut resp = JoinRoomResponse::new();
                resp.code = 401;
                resp.message = "not login".to_string();
                return Ok(resp);
            }
        };

        if self.room.is_some() {
            let mut resp = JoinRoomResponse::new();
            resp.code = 1001;
            resp.message = "has join room".to_string();
            return Ok(resp);
        }

        let mut rpc_req = game_backend::JoinRoomRequest::default();
        rpc_req.user_id = user.rowid;
        rpc_req.game_type = match game_backend::GameType::try_from(req.game_type) {
            Ok(v) => v,
            Err(_) => {
                let mut resp = JoinRoomResponse::new();
                resp.code = 1002;
                resp.message = "game not supported".to_string();
                return Ok(resp);
            }
        };
        rpc_req.strategy = game_backend::JoinRoomStrategy::Mate;
        rpc_req.extra_data = clone_extra_data(req.extra_data);
        let rpc_resp = rpc::game::client().join_room(rpc_req).await?.into_inner();

        let room_key = RoomKey {
            user_id: user.rowid,
            game_type: req.game_type,
            room_id: rpc_resp.room_id,
        };
        self.room = Some(room_key);
        let mut rooms = ROOMS.write().await;
        rooms.insert(room_key, self.con.clone());
        drop(rooms);

        let mut resp = JoinRoomResponse::new();
        resp.code = 0;
        resp.message = "success".to_string();
        Ok(resp)
    }
}

pub async fn get_room_wscon(key: &RoomKey) -> Option<Arc<super::WsCon>> {
    let rooms = ROOMS.read().await;
    match rooms.get(key) {
        Some(wscon) => Some(wscon.clone()),
        None => None,
    }
}

fn clone_extra_data(extra_data: Option<Vec<u8>>) -> Option<pilota::Bytes> {
    match extra_data {
        Some(data) => Some(pilota::Bytes::copy_from_slice(&data)),
        None => None,
    }
}
