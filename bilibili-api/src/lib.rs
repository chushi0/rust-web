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
