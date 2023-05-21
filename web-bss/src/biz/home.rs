use std::time::Duration;
use std::time::SystemTime;
use std::time::UNIX_EPOCH;

use crate::model::home::*;
use crate::model::Model;
use anyhow::*;

pub type GetEventsResp = Vec<EventData>;
pub async fn get_events() -> Result<Model<GetEventsResp>> {
    let systime = SystemTime::now().duration_since(UNIX_EPOCH)?;

    let events = crate::dao::event::list_display_event(
        crate::dao::event::ListDisplayEventParam::ByEventTime {
            min_event_time: (systime - Duration::from_secs(30 * 86400)).as_secs() as i64,
        },
    )
    .await?;
    let events = events
        .iter()
        .map(|event| EventData {
            title: event.title.clone(),
            msg: event.message.clone(),
            time: event.event_time,
            link: event.link.clone(),
        })
        .collect();

    Ok(Model::from_success(events))
}
