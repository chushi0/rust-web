use anyhow::Result;
use futures::TryStreamExt;

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct Card {
    pub rowid: i64,
    pub code: String,
    pub name: String,
    pub card_type: i32,
    pub mana_cost: i32,
    pub derive: bool,
    pub need_select_target: bool,
    pub card_info: String,
    pub description: String,
    pub resources: String,
    pub create_time: i64,
    pub update_time: i64,
    pub enable: bool,
}

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct Resources {
    pub rowid: i64,
    pub uri: String,
    pub md5: String,
    pub sha1: String,
    pub size: i64,
}

pub async fn get_all_cards(db: &mut super::Transaction<'_>) -> Result<Vec<Card>> {
    let mut iter = sqlx::query_as("select rowid, * from card").fetch(&mut db.tx);

    let mut res = Vec::new();
    while let Some(row) = iter.try_next().await? {
        res.push(row);
    }

    Ok(res)
}

pub async fn get_card_by_code(db: &mut super::Transaction<'_>, code: &str) -> Result<Card> {
    Ok(sqlx::query_as("select rowid, * from card where code = ?")
        .bind(code)
        .fetch_one(&mut db.tx)
        .await?)
}

pub async fn get_all_resources(db: &mut super::Transaction<'_>) -> Result<Vec<Resources>> {
    let mut iter = sqlx::query_as("select rowid, * from resources").fetch(&mut db.tx);

    let mut res = Vec::new();
    while let Some(row) = iter.try_next().await? {
        res.push(row);
    }

    Ok(res)
}
