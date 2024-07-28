use anyhow::Result;
use sqlx::Row;

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct User {
    pub id: i64,
    pub account: String,
    pub password: String,
    pub username: String,
    pub create_time: i64,
    pub update_time: i64,
    pub last_login_time: Option<i64>,
}

pub enum QueryUserParam {
    ByAccount { account: String },
    ByUid { uid: i64 },
}

pub async fn insert_user(db: &mut super::Transaction<'_>, user: &mut User) -> Result<()> {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)?
        .as_secs() as i64;
    user.create_time = now;
    user.update_time = now;

    sqlx::query("insert into user (account, password, username, create_time, update_time) values (?, ?, ?, ?, ?)")
        .bind(&user.account)
        .bind(&user.password)
        .bind(&user.username)
        .bind(user.create_time)
        .bind(user.update_time)
        .execute(&mut db.tx)
        .await?;

    let id: i64 = sqlx::query("select last_insert_id()")
        .fetch_one(&mut db.tx)
        .await?
        .get(0);

    user.id = id;

    Ok(())
}

pub async fn query_user(db: &mut super::Transaction<'_>, param: QueryUserParam) -> Result<User> {
    let event = match param {
        QueryUserParam::ByAccount { account } => {
            sqlx::query_as("select * from user where account = ? limit 1").bind(account)
        }
        QueryUserParam::ByUid { uid } => {
            sqlx::query_as("select * from user where id = ? limit 1").bind(uid)
        }
    }
    .fetch_one(&mut db.tx)
    .await?;

    Ok(event)
}

pub async fn update_user_login_time(db: &mut super::Transaction<'_>, id: i64) -> Result<()> {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)?
        .as_secs() as i64;

    sqlx::query("update user set last_login_time = ? where id = ?")
        .bind(now)
        .bind(id)
        .execute(&mut db.tx)
        .await?;

    Ok(())
}
