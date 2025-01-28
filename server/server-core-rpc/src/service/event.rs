use anyhow::{anyhow, Result};
use chrono::DateTime;
use common::tonic_idl_gen::{
    CreateGithubActivityEvent, DisplayEvent, GithubActivityEvent, ListDisplayEventRequest,
    ListDisplayEventResponse, ListGithubActivityEventRequest, ListGithubActivityEventResponse,
};

use crate::dao::{
    display_event::{self, DisplayEventRepository, ListDisplayEventParameters},
    github_activity_event::{
        self, GithubActivityEventRepository, ListGithubActivityEventParameters,
    },
};

pub async fn list_display_event<DB: DisplayEventRepository>(
    db: &mut DB,
    request: ListDisplayEventRequest,
) -> Result<ListDisplayEventResponse> {
    let params = ListDisplayEventParameters {
        offset: request.offset,
        limit: request.count,
        min_event_time: request
            .min_event_time
            .and_then(|timestamp| DateTime::from_timestamp(timestamp, 0)),
        max_event_time: request
            .max_event_time
            .and_then(|timestamp| DateTime::from_timestamp(timestamp, 0)),
    };

    let events = db.list_display_event(&params).await?;
    let count = db.count_display_event(&params).await?;

    Ok(ListDisplayEventResponse {
        total: count,
        events: events
            .into_iter()
            .map(|event| DisplayEvent {
                id: event.id,
                title: event.title,
                message: event.message,
                link: event.link,
                event_time: event.event_time.timestamp(),
                create_time: event.create_time.timestamp(),
                update_time: event.update_time.timestamp(),
            })
            .collect(),
    })
}

pub async fn list_github_activity_event<DB: GithubActivityEventRepository>(
    db: &mut DB,
    request: ListGithubActivityEventRequest,
) -> Result<ListGithubActivityEventResponse> {
    let params = ListGithubActivityEventParameters {
        offset: request.offset,
        limit: request.count,
        order_by_event_time_desc: request.order_by_event_time_desc.unwrap_or(false),
    };

    let events = db.list_github_activity_event(&params).await?;
    let count = db.count_github_activity_event().await?;

    Ok(ListGithubActivityEventResponse {
        total: count,
        events: events
            .into_iter()
            .map(|event| GithubActivityEvent {
                id: event.id,
                raw_data: event.raw_data,
                event_time: event.event_time.timestamp(),
                create_time: event.create_time.timestamp(),
                update_time: event.update_time.timestamp(),
            })
            .collect(),
    })
}

pub async fn create_github_activity_event<
    DB: GithubActivityEventRepository + DisplayEventRepository,
>(
    db: &mut DB,
    to_create_event: CreateGithubActivityEvent,
) -> Result<()> {
    let event_time = DateTime::from_timestamp(to_create_event.event_time, 0)
        .ok_or(anyhow!("invalid event_time"))?;

    let mut event = github_activity_event::GithubActivityEvent {
        raw_data: to_create_event.raw_data,
        event_time,
        ..Default::default()
    };

    db.create_github_activity_event(&mut event).await?;

    if let Some(to_create_display_event) = to_create_event.display_event {
        let mut display_event = display_event::DisplayEvent {
            title: to_create_display_event.title,
            message: to_create_display_event.message,
            link: to_create_display_event.link,
            event_time,
            ..Default::default()
        };

        db.create_display_event(&mut display_event).await?;
    }

    Ok(())
}
