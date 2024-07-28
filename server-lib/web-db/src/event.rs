use anyhow::{anyhow, Result};
use futures::TryStreamExt;
use sqlx::Row;

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct DisplayEvent {
    pub id: u64,
    pub title: String,
    pub message: String,
    pub link: String,
    pub event_time: i64,
    pub create_time: i64,
    pub update_time: i64,
}

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct GithubActivityEvent {
    pub id: u64,
    pub raw_data: String,
    pub event_time: i64,
    pub create_time: i64,
    pub update_time: i64,
}

pub enum ListDisplayEventParam {
    ByEventTime { min_event_time: i64 },
}

pub async fn insert_display_event(
    db: &mut super::Transaction<'_>,
    event: &mut DisplayEvent,
) -> Result<()> {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)?
        .as_secs() as i64;
    event.create_time = now;
    event.update_time = now;

    sqlx::query(
        "insert into display_event (title, message, link, event_time, create_time, update_time) values (?, ?, ?, ?, ?, ?)",
        )
        .bind(&event.title)
        .bind(&event.message)
        .bind(&event.link)
        .bind(event.event_time)
        .bind(event.create_time)
        .bind(event.update_time)
        .execute( &mut db.tx)
        .await?;

    let id = sqlx::query("select last_insert_id()")
        .fetch_one(&mut db.tx)
        .await?
        .get(0);

    event.id = id;

    Ok(())
}

pub async fn list_display_event(
    db: &mut super::Transaction<'_>,
    param: ListDisplayEventParam,
) -> Result<Vec<DisplayEvent>> {
    let mut iter = match param {
        ListDisplayEventParam::ByEventTime { min_event_time } => sqlx::query_as(
            "select * from display_event where event_time > ? order by event_time desc",
        )
        .bind(min_event_time)
        .fetch(&mut db.tx),
    };

    let mut res = Vec::new();
    while let Some(row) = iter.try_next().await? {
        res.push(row)
    }
    Ok(res)
}

pub async fn insert_github_activity_event(
    db: &mut super::Transaction<'_>,
    event: &mut GithubActivityEvent,
) -> Result<()> {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)?
        .as_secs() as i64;
    event.create_time = now;
    event.update_time = now;

    sqlx::query(
    "insert into github_activity_event (raw_data, event_time, create_time, update_time) values (?, ?, ?, ?)",
    )
    .bind(&event.raw_data)
    .bind(event.event_time)
    .bind(event.create_time)
    .bind(event.update_time)
    .execute(&mut db.tx)
    .await?;

    let id = sqlx::query("select last_insert_id()")
        .fetch_one(&mut db.tx)
        .await?
        .get(0);

    event.id = id;

    Ok(())
}

pub async fn get_last_github_activity_event(
    db: &mut super::Transaction<'_>,
) -> Result<Option<GithubActivityEvent>> {
    let event: Result<GithubActivityEvent, sqlx::Error> =
        sqlx::query_as("select * from github_activity_event order by event_time desc limit 1")
            .fetch_one(&mut db.tx)
            .await;

    match event {
        Ok(event) => Ok(Some(event)),
        Err(error) => {
            if let sqlx::Error::RowNotFound = error {
                Ok(None)
            } else {
                Err(anyhow!("{error}"))
            }
        }
    }
}
