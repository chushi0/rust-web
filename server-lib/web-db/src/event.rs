use anyhow::Result;
use chrono::{DateTime, Utc};
use futures::TryStreamExt;

#[derive(Debug, Clone, sqlx::FromRow, Default)]
pub struct DisplayEvent {
    pub id: u64,
    pub title: String,
    pub message: String,
    pub link: String,
    pub event_time: DateTime<Utc>,
    pub create_time: DateTime<Utc>,
    pub update_time: DateTime<Utc>,
}

#[derive(Debug, Clone, sqlx::FromRow, Default)]
pub struct GithubActivityEvent {
    pub id: u64,
    pub raw_data: String,
    pub event_time: DateTime<Utc>,
    pub create_time: DateTime<Utc>,
    pub update_time: DateTime<Utc>,
}

pub enum ListDisplayEventParam {
    ByEventTime { min_event_time: DateTime<Utc> },
}

pub async fn insert_display_event(
    db: &mut super::Transaction<'_>,
    event: &mut DisplayEvent,
) -> Result<()> {
    let result = sqlx::query(
        "insert into display_event (title, message, link, event_time) values (?, ?, ?, ?)",
    )
    .bind(&event.title)
    .bind(&event.message)
    .bind(&event.link)
    .bind(event.event_time)
    .execute(&mut db.tx)
    .await?;

    event.id = result.last_insert_id();

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
    let result =
        sqlx::query("insert into github_activity_event (raw_data, event_time) values (?, ?)")
            .bind(&event.raw_data)
            .bind(event.event_time)
            .execute(&mut db.tx)
            .await?;

    event.id = result.last_insert_id();

    Ok(())
}

pub async fn get_last_github_activity_event(
    db: &mut super::Transaction<'_>,
) -> Result<Option<GithubActivityEvent>> {
    Ok(
        sqlx::query_as("select * from github_activity_event order by event_time desc limit 1")
            .fetch_optional(&mut db.tx)
            .await?,
    )
}
