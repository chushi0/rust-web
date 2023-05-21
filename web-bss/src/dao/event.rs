use anyhow::*;
use futures::TryStreamExt;

async fn db() -> Result<super::Conn> {
    super::create_connection("event").await
}

#[derive(sqlx::FromRow)]
pub struct DisplayEvent {
    pub rowid: i64,
    pub title: String,
    pub message: String,
    pub link: String,
    pub event_time: i64,
    pub create_time: i64,
    pub update_time: i64,
}

pub enum ListDisplayEventParam {
    ByEventTime { min_event_time: i64 },
}

pub async fn list_display_event(param: ListDisplayEventParam) -> Result<Vec<DisplayEvent>> {
    let mut db = db().await?;

    let mut iter = match param {
        ListDisplayEventParam::ByEventTime { min_event_time } => sqlx::query_as(
            "select rowid,* from display_event where event_time > ?1 order by event_time desc",
        )
        .bind(min_event_time)
        .fetch(&mut db),
    };

    let mut res = Vec::new();
    while let Some(row) = iter.try_next().await? {
        res.push(row)
    }
    Ok(res)
}
