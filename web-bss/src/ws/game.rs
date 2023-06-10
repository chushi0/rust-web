use super::WsBiz;
use anyhow::Result;
use idl_gen::bss_websocket_client::{BoxProtobufPayload, ClientLoginRequest, ClientLoginResponse};
use log::warn;
use protobuf::Message;
use web_db::user::{query_user, update_user_login_time, QueryUserParam, User};
use web_db::{begin_tx, create_connection, RDS};

pub struct GameBiz {
    con: super::WsCon,
    user: Option<User>,
}

impl GameBiz {
    pub fn create(con: super::WsCon) -> GameBiz {
        GameBiz {
            con: con,
            user: None,
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

    async fn on_close(&mut self) {}
}

impl GameBiz {
    async fn do_binary_message(&mut self, msg: &[u8]) -> Result<()> {
        let paylod = BoxProtobufPayload::parse_from_bytes(msg)?;
        if paylod.name == ClientLoginRequest::NAME {
            let req = ClientLoginRequest::parse_from_bytes(paylod.payload.as_slice())?;
            let resp = match self.client_login(req).await {
                Ok(resp) => resp,
                Err(e) => {
                    warn!("error when handle client_login: {e}");
                    let mut resp = ClientLoginResponse::new();
                    resp.code = 500;
                    resp.message = "internal error".to_string();
                    resp
                }
            };
            let mut payload = BoxProtobufPayload::new();
            payload.name = ClientLoginResponse::NAME.to_string();
            payload.payload = resp.write_to_bytes()?;

            self.con.send_binary(payload.write_to_bytes()?).await?;
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
}
