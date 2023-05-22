use std::time::Duration;
use std::time::SystemTime;
use std::time::UNIX_EPOCH;

use crate::model::home::*;
use crate::model::Model;
use anyhow::*;
use web_db::event::{list_display_event, ListDisplayEventParam};
use web_db::{begin_tx, create_connection, RDS};

pub type GetEventsResp = Vec<EventData>;
pub async fn get_events() -> Result<Model<GetEventsResp>> {
    let mut conn = create_connection(RDS::Event).await?;
    let mut tx = begin_tx(&mut conn).await?;

    let systime = SystemTime::now().duration_since(UNIX_EPOCH)?;

    let events = list_display_event(
        &mut tx,
        ListDisplayEventParam::ByEventTime {
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
