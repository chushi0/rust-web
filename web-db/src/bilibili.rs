use anyhow::Result;
use futures::TryStreamExt;
use sqlx::Row;

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct BangumiWatch {
    pub rowid: i64,
    pub ssid: i32,
    pub send_ep: i32,
    pub finish: bool,
    pub next_query_time: i64,
    pub create_time: i64,
    pub update_time: i64,
}

pub async fn insert_bangumi_watch(
    db: &mut super::Transaction<'_>,
    bangumi_watch: &mut BangumiWatch,
) -> Result<()> {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)?
        .as_secs() as i64;
    bangumi_watch.create_time = now;
    bangumi_watch.update_time = now;

    sqlx::query(
    "insert into bangumi_watch (ssid, send_ep, finish, next_query_time, create_time, update_time) values (?1, ?2, ?3, ?4, ?5, ?6)",
    )
    .bind(bangumi_watch.ssid)
    .bind(bangumi_watch.send_ep)
    .bind(bangumi_watch.finish)
    .bind(bangumi_watch.next_query_time)
    .bind(bangumi_watch.create_time)
    .bind(bangumi_watch.update_time)
    .execute( &mut db.tx)
    .await?;

    let id: i64 = sqlx::query("select last_insert_rowid()")
        .fetch_one(&mut db.tx)
        .await?
        .get(0);

    bangumi_watch.rowid = id;

    Ok(())
}

pub async fn get_all_bangumi_watch(db: &mut super::Transaction<'_>) -> Result<Vec<BangumiWatch>> {
    let mut iter = sqlx::query_as("select rowid,* from bangumi_watch").fetch(&mut db.tx);

    let mut res = Vec::new();
    while let Some(row) = iter.try_next().await? {
        res.push(row)
    }
    Ok(res)
}

pub async fn update_send_ep_and_query_time(
    db: &mut super::Transaction<'_>,
    bangumi_watch: &mut BangumiWatch,
) -> Result<()> {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)?
        .as_secs() as i64;
    bangumi_watch.update_time = now;

    sqlx::query(
        "update bangumi_watch set send_ep=?1, next_query_time=?2, update_time=?3 where rowid=?4",
    )
    .bind(bangumi_watch.send_ep)
    .bind(bangumi_watch.next_query_time)
    .bind(bangumi_watch.update_time)
    .bind(bangumi_watch.rowid)
    .execute(&mut db.tx)
    .await?;

    Ok(())
}
