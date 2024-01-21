use super::WsBiz;
use crate::rpc;
use crate::service;
use crate::util::protobuf::pack_message;
use anyhow::anyhow;
use anyhow::bail;
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
            info!("connection reset with leaving room: {room:?}");
            Self::release_room_connection(room).await;
            let leave_resp = Self::try_remove_from_room(room).await;
            log::info!("try remove from room: {:?}", leave_resp);
        }
    }
}

impl GameBiz {
    async fn do_binary_message(&mut self, msg: &[u8]) -> Result<()> {
        macro_rules! router {
            ($($req:ty => $func:tt $(: $resp:ty)? ,)*) => {
                let paylod = BoxProtobufPayload::parse_from_bytes(msg)?;

                match paylod.name.as_ref() {
                    $(
                        <$req>::NAME => {
                            let req = <$req>::parse_from_bytes(paylod.payload.as_slice())?;
                            let resp = self.$func(req).await;
                            if resp.is_err() {
                                warn!("error when handle {}: {resp:?}", paylod.name);
                            }
                            $(
                                let resp = resp.unwrap_or_else(|_| {
                                    let mut resp = <$resp>::new();
                                    resp.code = 500;
                                    resp.message = "internal error".to_string();
                                    resp
                                });
                                self.con.send_binary(pack_message(resp)?)?;
                            )?
                        }
                    )*
                    name => return Err(anyhow::anyhow!("Received not supported message: {name}")),
                }
            };
        }

        router! {
            ClientLoginRequest => client_login: ClientLoginResponse,
            CreateRoomRequest => create_room: JoinRoomResponse,
            JoinRoomRequest => join_room: JoinRoomResponse,
            MateRoomRequest => mate_room: JoinRoomResponse,
            LeaveRoomRequest => leave_room: LeaveRoomResponse,
            RoomPlayerAction => room_player_action,
            GameAction => game_action,
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

        let rpc_req = game_backend::JoinRoomRequest {
            user_id: user.rowid,
            game_type: match game_backend::GameType::try_from(req.game_type) {
                Ok(v) => v,
                Err(_) => {
                    let mut resp = JoinRoomResponse::new();
                    resp.code = 1002;
                    resp.message = "game not supported".to_string();
                    return Ok(resp);
                }
            },
            strategy: game_backend::JoinRoomStrategy::Create,
            public: Some(req.init_public),
            extra_data: clone_extra_data(req.extra_data),
            ..Default::default()
        };
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
        resp.players =
            service::game::pack_game_room_player(&as_client_room_players(&rpc_resp.players))
                .await?;
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

        let rpc_req = game_backend::JoinRoomRequest {
            user_id: user.rowid,
            game_type: match game_backend::GameType::try_from(req.game_type) {
                Ok(v) => v,
                Err(_) => {
                    let mut resp = JoinRoomResponse::new();
                    resp.code = 1002;
                    resp.message = "game not supported".to_string();
                    return Ok(resp);
                }
            },
            strategy: game_backend::JoinRoomStrategy::Join,
            room_id: Some(req.room_id),
            extra_data: clone_extra_data(req.extra_data),
            ..Default::default()
        };
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
        resp.players =
            service::game::pack_game_room_player(&as_client_room_players(&rpc_resp.players))
                .await?;
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

        let rpc_req = game_backend::JoinRoomRequest {
            user_id: user.rowid,
            game_type: match game_backend::GameType::try_from(req.game_type) {
                Ok(v) => v,
                Err(_) => {
                    let mut resp = JoinRoomResponse::new();
                    resp.code = 1002;
                    resp.message = "game not supported".to_string();
                    return Ok(resp);
                }
            },
            strategy: game_backend::JoinRoomStrategy::Mate,
            extra_data: clone_extra_data(req.extra_data),
            ..Default::default()
        };
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
        resp.players =
            service::game::pack_game_room_player(&as_client_room_players(&rpc_resp.players))
                .await?;
        Ok(resp)
    }

    async fn leave_room(&mut self, _req: LeaveRoomRequest) -> Result<LeaveRoomResponse> {
        let Some(room) = self.room else {
            bail!("user has not join any room");
        };

        Self::try_remove_from_room(&room).await?;
        Self::release_room_connection(&room).await;
        self.room = None;

        Ok(LeaveRoomResponse {
            code: 0,
            message: "success".to_string(),
            ..Default::default()
        })
    }

    async fn room_player_action(&mut self, req: RoomPlayerAction) -> Result<()> {
        let Some(room) = self.room else {
            return Err(anyhow!("user has not join any room"));
        };

        let user_id = room.user_id;
        let game_type = game_backend::GameType::try_from(room.game_type)?;
        let room_id = room.room_id;

        if let Some(ready) = req.ready {
            let rpc_req = game_backend::SetPlayerReadyRequest {
                user_id,
                game_type,
                room_id,
                ready,
            };
            rpc::game::client().set_player_ready(rpc_req).await?;
        }

        if let Some(chat) = req.chat {
            let rpc_req = game_backend::SendGameChatRequest {
                user_id,
                game_type,
                room_id,
                receiver_user_id: req.chat_receiver,
                content: chat.into(),
            };
            rpc::game::client().send_game_chat(rpc_req).await?;
        }

        if let Some(public) = req.make_public {
            if public {
                let rpc_req = game_backend::SetRoomPublicRequest {
                    user_id,
                    game_type,
                    room_id,
                };
                rpc::game::client().set_room_public(rpc_req).await?;
            }
        }

        Ok(())
    }

    async fn game_action(&mut self, req: GameAction) -> Result<()> {
        let Some(room) = self.room else {
            return Err(anyhow!("user has not join any room"));
        };

        let user_id = room.user_id;
        let game_type = game_backend::GameType::try_from(room.game_type)?;
        let room_id = room.room_id;

        let req = game_backend::SubmitPlayerActionRequest {
            user_id,
            game_type,
            room_id,
            action_name: req.action_type.into(),
            payload: req.payload.into(),
        };

        rpc::game::client().submit_player_action(req).await?;
        Ok(())
    }

    async fn try_remove_from_room(room: &RoomKey) -> Result<()> {
        let req = game_backend::LeaveRoomRequest {
            user_id: room.user_id,
            game_type: game_backend::GameType::try_from(room.game_type)?,
            room_id: room.room_id,
        };
        rpc::game::client().leave_room(req).await?;
        Ok(())
    }

    async fn release_room_connection(room: &RoomKey) {
        let mut rooms = ROOMS.write().await;
        rooms.remove(room);
    }
}

pub async fn get_room_wscon(key: &RoomKey) -> Option<Arc<super::WsCon>> {
    let rooms = ROOMS.read().await;
    rooms.get(key).cloned()
}

fn clone_extra_data(extra_data: Option<Vec<u8>>) -> Option<pilota::Bytes> {
    extra_data.map(|data| pilota::Bytes::copy_from_slice(&data))
}

fn as_client_room_players(
    players: &[idl_gen::game_backend::RoomPlayer],
) -> Vec<idl_gen::bss_websocket::RoomPlayer> {
    players
        .iter()
        .map(|player| idl_gen::bss_websocket::RoomPlayer {
            user_id: player.user_id,
            index: player.index,
            ready: player.ready,
            online: player.online,
            master: player.master,
        })
        .collect()
}
