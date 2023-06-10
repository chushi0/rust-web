use crate::model::Model;
use anyhow::Result;
use pilota::serde::Serialize;
use web_db::user::{insert_user, User};
use web_db::{begin_tx, create_connection, RDS};

#[derive(Debug, Clone, Serialize)]
#[serde(crate = "rocket::serde")]
pub struct NewUserResp {
    pub account: String,
    pub password: String,
}

pub async fn new_user() -> Result<Model<NewUserResp>> {
    let mut conn = create_connection(RDS::User).await?;
    let mut tx = begin_tx(&mut conn).await?;

    let mut resp = NewUserResp {
        account: String::default(),
        password: String::default(),
    };

    let uuid = uuid::Uuid::new_v4().to_string();
    resp.account = uuid[..23].to_string();
    resp.password = uuid[24..].to_string();

    let mut user = User {
        rowid: 0,
        account: resp.account.clone(),
        password: resp.password.clone(),
        username: String::default(),
        create_time: 0,
        update_time: 0,
        last_login_time: None,
    };

    insert_user(&mut tx, &mut user).await?;
    tx.commit().await?;

    Ok(Model::from_success(resp))
}
