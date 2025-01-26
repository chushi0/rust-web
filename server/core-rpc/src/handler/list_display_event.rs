use anyhow::{anyhow, Result};
use chrono::DateTime;
use idl_gen::core_rpc::{DisplayEvent, ListDisplayEventRequest, ListDisplayEventResponse};
use volo_grpc::{Request, Response};
use web_db::{begin_tx, create_connection, event, RDS};

pub async fn handle(
    req: Request<ListDisplayEventRequest>,
) -> Result<Response<ListDisplayEventResponse>> {
    let req = req.get_ref();

    let Some(min_event_time) = req.min_event_time else {
        return Err(anyhow!("min event time not set"));
    };

    let mut conn = create_connection(RDS::Event).await?;
    let mut tx = begin_tx(&mut conn).await?;

    let display_events = event::list_display_event(
        &mut tx,
        event::ListDisplayEventParam::ByEventTime {
            min_event_time: DateTime::from_timestamp(min_event_time, 0)
                .ok_or(anyhow!("min event time not set"))?,
        },
    )
    .await?;

    let mut api_display_event = Vec::with_capacity(display_events.len());
    for event in display_events {
        api_display_event.push(DisplayEvent {
            id: event.id,
            title: event.title.into(),
            message: event.message.into(),
            link: event.link.into(),
            event_time: event.event_time.timestamp(),
            create_time: event.create_time.timestamp(),
            update_time: event.update_time.timestamp(),
        })
    }

    Ok(Response::new(ListDisplayEventResponse {
        total: api_display_event.len() as i64,
        events: api_display_event,
    }))
}
