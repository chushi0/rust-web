use anyhow::{anyhow, bail, Result};
use futures_util::{stream::StreamExt, SinkExt};
use heartstone::model::CardModel;
use idl_gen::{
    bff_heartstone::{
        BuffEvent, DamageEvent, DrawCardEvent, JoinRoomExtraData, MinionAttackEvent,
        MinionEffectEvent, MinionEnterEvent, MinionRemoveEvent, MyTurnEndEvent, MyTurnStartEvent,
        NewTurnEvent, PlayerManaChange, PlayerTurnAction, PlayerUseCardEndEvent,
        PlayerUseCardEvent, SwapFrontBackEvent, SyncGameStatus,
    },
    bff_websocket_client::{
        BoxProtobufPayload, ClientLoginRequest, ClientLoginResponse, CreateRoomRequest, GameAction,
        GameEventList, JoinRoomRequest, JoinRoomResponse, MateRoomRequest, PlayerChatEvent,
        RoomPlayerAction, RoomPlayerChangeEvent,
    },
    game_backend::GameType,
};
use protobuf::Message;
use serde::Deserialize;
use std::{collections::HashMap, sync::Arc};
use websocket_lite::{AsyncClient, AsyncNetworkStream};

#[derive(Debug)]
pub struct Client {
    pub ws_server_ip: String,
    pub http_server_ip: String,
    pub account: String,
    pub password: String,
    pub room_id: i32,
}

#[async_trait::async_trait]
impl crate::Client for Client {
    async fn run(&self) {
        // 加载数据库
        let cards = self.load_card_pool().await.unwrap();

        // 连接服务器
        let mut stream = self.establish_websocket().await.unwrap();

        // 登录
        self.login(&mut stream).await.unwrap();
        println!("登录成功");

        // 加入房间
        let room_number = self.join_room(&mut stream, cards).await.unwrap();
        println!("加入房间成功");
        if let Some(room_number) = room_number {
            println!("房间号：{room_number}");
        }

        // 主循环
        while let Some(Ok(msg)) = stream.next().await {
            if !matches!(msg.opcode(), websocket_lite::Opcode::Binary) {
                continue;
            }

            self.process(&mut stream, msg.data()).await.unwrap();
        }
    }
}

type WsStream = AsyncClient<Box<dyn AsyncNetworkStream + Send + Sync + Unpin>>;

impl Client {
    async fn load_card_pool(&self) -> Result<HashMap<i64, Arc<CardModel>>> {
        #[derive(Debug, Deserialize)]
        struct GetCardResp {
            code: i32,
            msg: String,
            data: Vec<CardResult>,
        }

        #[derive(Debug, Deserialize)]
        struct CardResult {
            code: String,
            name: String,
            card_type: i32,
            mana_cost: i32,
            derive: bool,
            need_select_target: bool,
            card_info: String,
            description: String,
            resources: String,
        }

        let url = format!("{}/api/game/heartstone/cards", self.http_server_ip);
        let resp: GetCardResp = reqwest::Client::new()
            .get(&url)
            .send()
            .await?
            .json()
            .await?;

        if resp.code != 0 {
            bail!("get heartstone fail: code={}, msg={}", resp.code, resp.msg);
        }

        let map: HashMap<i64, Arc<CardModel>> = resp
            .data
            .into_iter()
            .enumerate()
            .map(|(i, card)| web_db::hearthstone::Card {
                id: i as i64,
                code: card.code,
                name: card.name,
                card_type: card.card_type,
                mana_cost: card.mana_cost,
                derive: card.derive,
                need_select_target: card.need_select_target,
                card_info: card.card_info,
                description: card.description,
                resources: card.resources,
                create_time: 0,
                update_time: 0,
                enable: true,
            })
            .map(|card| -> Result<CardModel> {
                let card_info = serde_json::from_str(&card.card_info)?;
                Ok(CardModel { card_info, card })
            })
            .collect::<Result<Vec<CardModel>>>()?
            .into_iter()
            .map(|card_model| (card_model.card.id, Arc::new(card_model)))
            .collect();

        crate::io().cache_cards(&map.values().cloned().collect::<Vec<_>>());

        Ok(map)
    }

    async fn establish_websocket(&self) -> Result<WsStream> {
        let url = format!("{}/ws/game", self.ws_server_ip);
        let stream = websocket_lite::ClientBuilder::new(&url)?
            .async_connect()
            .await
            .map_err(|e| anyhow!("{e}"))?;

        Ok(stream)
    }

    async fn wait_for_message<T: Message>(&self, stream: &mut WsStream) -> Result<T> {
        while let Some(Ok(msg)) = stream.next().await {
            if !matches!(msg.opcode(), websocket_lite::Opcode::Binary) {
                continue;
            }

            let box_payload = BoxProtobufPayload::parse_from_bytes(msg.data())?;
            if box_payload.name != T::NAME {
                continue;
            }

            return Ok(T::parse_from_bytes(&box_payload.payload)?);
        }

        bail!("EOF when read from stream")
    }

    async fn send_message<T: Message>(&self, stream: &mut WsStream, msg: T) -> Result<()> {
        let box_payload = BoxProtobufPayload {
            name: T::NAME.to_string(),
            payload: msg.write_to_bytes()?,
            ..Default::default()
        };

        let msg = websocket_lite::Message::binary(box_payload.write_to_bytes()?);
        stream.send(msg).await.map_err(|e| anyhow!("{e}"))
    }

    async fn login(&self, stream: &mut WsStream) -> Result<()> {
        self.send_message(
            stream,
            ClientLoginRequest {
                account: self.account.clone(),
                password: self.password.clone(),
                ..Default::default()
            },
        )
        .await?;

        let resp: ClientLoginResponse = self.wait_for_message(stream).await?;
        if resp.code != 0 {
            bail!("login fail: {}", resp.message);
        }

        Ok(())
    }

    async fn join_room(
        &self,
        stream: &mut WsStream,
        cards: HashMap<i64, Arc<CardModel>>,
    ) -> Result<Option<i32>> {
        // 牌库是每张牌（衍生牌除外）各一张
        let deck: Vec<String> = cards
            .iter()
            .filter(|(_, model)| !model.card.derive)
            .map(|(_, model)| model.card.code.clone())
            .collect();

        let extra_data = JoinRoomExtraData {
            card_code: deck,
            ..Default::default()
        }
        .write_to_bytes()?;

        match self.room_id {
            0 => {
                self.send_message(
                    stream,
                    CreateRoomRequest {
                        game_type: GameType::HEARTHSTONE.inner(),
                        init_public: false,
                        game_version: 1,
                        extra_data: Some(extra_data),
                        ..Default::default()
                    },
                )
                .await?
            }
            -1 => {
                self.send_message(
                    stream,
                    MateRoomRequest {
                        game_type: GameType::HEARTHSTONE.inner(),
                        game_version: 1,
                        extra_data: Some(extra_data),
                        ..Default::default()
                    },
                )
                .await?
            }
            room_id => {
                self.send_message(
                    stream,
                    JoinRoomRequest {
                        game_type: GameType::HEARTHSTONE.inner(),
                        room_id,
                        game_version: 1,
                        extra_data: Some(extra_data),
                        ..Default::default()
                    },
                )
                .await?
            }
        }

        let resp: JoinRoomResponse = self.wait_for_message(stream).await?;
        if resp.code != 0 {
            bail!("join room fail: {}", resp.message)
        }

        self.send_message(
            stream,
            RoomPlayerAction {
                ready: Some(true),
                ..Default::default()
            },
        )
        .await?;

        Ok(resp.room_id)
    }

    async fn process(&self, stream: &mut WsStream, msg: &[u8]) -> Result<()> {
        macro_rules! router {
            ($($req:ty => $func:tt ,)*) => {
                let paylod = BoxProtobufPayload::parse_from_bytes(msg)?;

                match paylod.name.as_ref() {
                    $(
                        <$req>::NAME => {
                            let req = <$req>::parse_from_bytes(paylod.payload.as_slice())?;
                            self.$func(stream, req).await?;
                        }
                    )*
                    name => bail!("Received not supported message: {name}"),
                }
            };
        }

        router! {
            RoomPlayerChangeEvent => room_player_change_event,
            PlayerChatEvent => player_chat_event,
            GameEventList => game_event,
        }

        Ok(())
    }

    async fn room_player_change_event(
        &self,
        _stream: &mut WsStream,
        event: RoomPlayerChangeEvent,
    ) -> Result<()> {
        println!("room player change: {event:?}");
        Ok(())
    }

    async fn player_chat_event(
        &self,
        _stream: &mut WsStream,
        event: PlayerChatEvent,
    ) -> Result<()> {
        println!("player chat event: {event:?}");
        Ok(())
    }

    async fn game_event(&self, stream: &mut WsStream, events: GameEventList) -> Result<()> {
        for event in events.list {
            match event.event_type.as_ref() {
                SyncGameStatus::NAME => {
                    crate::io().print_game_status(SyncGameStatus::parse_from_bytes(&event.payload)?)
                }
                NewTurnEvent::NAME => {
                    crate::io().print_new_turn(NewTurnEvent::parse_from_bytes(&event.payload)?)
                }
                PlayerManaChange::NAME => crate::io()
                    .print_player_mana_change(PlayerManaChange::parse_from_bytes(&event.payload)?),
                DrawCardEvent::NAME => crate::io()
                    .print_player_draw_card(DrawCardEvent::parse_from_bytes(&event.payload)?),
                PlayerUseCardEvent::NAME => crate::io()
                    .print_player_use_card(PlayerUseCardEvent::parse_from_bytes(&event.payload)?),
                PlayerUseCardEndEvent::NAME => crate::io().print_player_card_effect_end(
                    PlayerUseCardEndEvent::parse_from_bytes(&event.payload)?,
                ),
                SwapFrontBackEvent::NAME => crate::io().print_player_swap_fightline(
                    SwapFrontBackEvent::parse_from_bytes(&event.payload)?,
                ),
                MinionEnterEvent::NAME => crate::io()
                    .print_minion_enter(MinionEnterEvent::parse_from_bytes(&event.payload)?),
                MinionEffectEvent::NAME => crate::io()
                    .print_minion_effect(MinionEffectEvent::parse_from_bytes(&event.payload)?),
                MinionAttackEvent::NAME => crate::io()
                    .print_minion_attack(MinionAttackEvent::parse_from_bytes(&event.payload)?),
                MinionRemoveEvent::NAME => crate::io()
                    .print_minion_remove(MinionRemoveEvent::parse_from_bytes(&event.payload)?),
                DamageEvent::NAME => {
                    crate::io().print_deal_damage(DamageEvent::parse_from_bytes(&event.payload)?)
                }
                BuffEvent::NAME => {
                    crate::io().print_buff(BuffEvent::parse_from_bytes(&event.payload)?)
                }
                MyTurnStartEvent::NAME => {
                    let action = crate::io().next_action();
                    self.send_message(
                        stream,
                        GameAction {
                            action_type: PlayerTurnAction::NAME.to_string(),
                            payload: action.write_to_bytes()?,
                            ..Default::default()
                        },
                    )
                    .await?;
                }
                MyTurnEndEvent::NAME => {
                    println!("<回合结束>")
                }
                name => bail!("Received not supported event: {name}"),
            }
        }
        Ok(())
    }
}
