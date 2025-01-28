use crate::api::github::activity::{list_user_public_events, EventPayload};
use anyhow::{anyhow, Result};
use chrono::DateTime;
use common::tonic_idl_gen::{
    core_rpc_service_client::CoreRpcServiceClient, CreateDisplayEvent, CreateGithubActivityEvent,
    CreateGithubActivityEventRequest, ListGithubActivityEventRequest,
};
use tonic::Request;

pub async fn handle() -> Result<()> {
    let mut core_rpc_client =
        CoreRpcServiceClient::connect("http://core-rpc-service.default.svc.cluster.local:13000")
            .await?;

    // 上次加载的位置
    let last_github_activity = core_rpc_client
        .list_github_activity_event(Request::new(ListGithubActivityEventRequest {
            offset: 0,
            count: 1,
            order_by_event_time_desc: Some(true),
            ..Default::default()
        }))
        .await?
        .into_inner()
        .events;
    let last_github_activity = last_github_activity.first();
    log::debug!("last_github_activity: {last_github_activity:?}");

    let last_event_time = DateTime::from_timestamp(
        last_github_activity
            .map(|event| event.event_time)
            .unwrap_or(1672502400), // 第一次拉取，仅拉取2023/01/01后的信息
        0,
    )
    .ok_or(anyhow!("datetime from timestamp failed"))?;

    // 开始本次加载
    let mut page = 1;
    let mut load = true;
    while load && page < 7 {
        let events = list_user_public_events("chushi0", 50, page).await?;
        if events.is_empty() {
            break;
        }

        for event in &events {
            let event_time = DateTime::from_timestamp(event.created_at.unix_timestamp(), 0).ok_or(
                anyhow!("event.created_at.unix_timestamp is not a valid timestamp"),
            )?;
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

            core_rpc_client
                .create_github_activity_event(Request::new(CreateGithubActivityEventRequest {
                    events: vec![CreateGithubActivityEvent {
                        raw_data,
                        event_time: event_time.timestamp(),
                        display_event: Some(CreateDisplayEvent {
                            title: event_title,
                            message: event_message,
                            link: format!("https://github.com/{}", event.repository.name),
                        }),
                    }],
                }))
                .await?;
        }

        page += 1;
    }

    Ok(())
}
