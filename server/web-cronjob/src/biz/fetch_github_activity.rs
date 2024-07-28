use crate::api::github::activity::{list_user_public_events, EventPayload};
use anyhow::Result;
use std::time::{SystemTime, UNIX_EPOCH};
use web_db::event::{
    get_last_github_activity_event, insert_display_event, insert_github_activity_event,
    DisplayEvent, GithubActivityEvent,
};
use web_db::{begin_tx, create_connection, RDS};

pub async fn handle() -> Result<()> {
    let mut conn = create_connection(RDS::Event).await?;
    let mut tx = begin_tx(&mut conn).await?;

    // 上次加载的位置
    let last_github_activity = get_last_github_activity_event(&mut tx).await?;
    log::debug!("last_github_activity: {last_github_activity:?}");
    let last_event_time = match last_github_activity {
        Some(event) => event.event_time,
        None => 1672502400, // 第一次拉取，仅拉取2023/01/01后的信息
    };

    // 开始本次加载
    let mut page = 1;
    let mut load = true;
    while load && page < 7 {
        let events = list_user_public_events("chushi0", 50, page).await?;
        if events.is_empty() {
            break;
        }

        let now = SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs() as i64;

        for event in &events {
            let event_time = event.created_at.unix_timestamp();
            log::debug!("event_time: {event_time}");

            if event_time <= last_event_time {
                load = false;
                break;
            }

            let (event_title, event_message) = match &event.payload {
                EventPayload::ForkEvent { payload } => (
                    format!("fork 了 {}", payload.forkee.name),
                    payload.forkee.description.as_str().to_string(),
                ),
                EventPayload::PublicEvent => (
                    format!("公开了仓库 {}", event.repository.name),
                    "".to_string(),
                ),
                EventPayload::PushEvent { payload } => {
                    let mut messages = String::new();
                    for commit in &payload.commits {
                        messages.push_str(commit.message.as_str());
                        messages.push(';');
                    }

                    (
                        format!(
                            "向 {} 仓库推送了 {} 条提交",
                            event.repository.name, payload.size
                        ),
                        messages,
                    )
                }
                EventPayload::WatchEvent { payload: _ } => (
                    format!("开始关注仓库 {}", event.repository.name),
                    "".to_string(),
                ),
                EventPayload::Unknown => continue,
            };

            let raw_data = serde_json::to_string(event)?;

            let mut github_activity_event = GithubActivityEvent {
                id: 0,
                raw_data,
                event_time,
                create_time: now,
                update_time: now,
            };

            let mut display_event = DisplayEvent {
                id: 0,
                title: event_title,
                message: event_message,
                link: format!("https://github.com/{}", event.repository.name),
                event_time,
                create_time: now,
                update_time: now,
            };

            insert_github_activity_event(&mut tx, &mut github_activity_event).await?;
            insert_display_event(&mut tx, &mut display_event).await?;
        }

        page += 1;
    }

    tx.commit().await?;
    Ok(())
}
