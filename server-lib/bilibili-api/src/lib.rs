use anyhow::Result;
use serde::{Deserialize, Serialize};

pub(crate) mod internal;

pub mod bangumi;

#[derive(Debug, Serialize, Deserialize)]
pub struct Client {
    dede_user_id: i32,
    dede_user_id_ckmd5: String,
    sessdata: String,
    bili_jct: String,
}

impl Client {
    pub fn from_env() -> Result<Self> {
        Ok(Client {
            dede_user_id: env!("RUST_WEB_BILIBILI_CLIENT_DEDE_USER_ID").parse()?,
            dede_user_id_ckmd5: env!("RUST_WEB_BILIBILI_CLIENT_DEDE_USER_ID_CKMD5").to_string(),
            sessdata: env!("RUST_WEB_BILIBILI_CLIENT_SESSDATA").to_string(),
            bili_jct: env!("RUST_WEB_BILIBILI_CLIENT_BILI_JCT").to_string(),
        })
    }
}
