use std::time::Duration;
use std::time::SystemTime;
use std::time::UNIX_EPOCH;

use crate::model::home::*;
use crate::model::Model;
use crate::rpc;
use anyhow::*;
use idl_gen::core_rpc::ListDisplayEventRequest;

pub type GetEventsResp = Vec<EventData>;
pub async fn get_events() -> Result<Model<GetEventsResp>> {
    let systime = SystemTime::now().duration_since(UNIX_EPOCH)?;
    let min_event_time = (systime - Duration::from_secs(12 * 30 * 86400)).as_secs() as i64;

    let events = rpc::core_rpc::client()?
        .list_display_event(ListDisplayEventRequest {
            offset: 0,
            count: 100,
            min_event_time: Some(min_event_time),
            ..Default::default()
        })
        .await?;

    let events = events
        .into_inner()
        .events
        .into_iter()
        .map(|event| EventData {
            title: event.title.into(),
            msg: event.message.into(),
            time: event.event_time,
            link: event.link.into(),
        })
        .collect();

    Ok(Model::from_success(events))
}
