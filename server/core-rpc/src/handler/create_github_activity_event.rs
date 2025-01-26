use anyhow::{anyhow, Result};
use chrono::DateTime;
use idl_gen::core_rpc::{CreateGithubActivityEventRequest, CreateGithubActivityEventResponse};
use volo_grpc::{Request, Response};
use web_db::{
    begin_tx, create_connection,
    event::{self, DisplayEvent, GithubActivityEvent},
    RDS,
};

pub async fn handle(
    req: Request<CreateGithubActivityEventRequest>,
) -> Result<Response<CreateGithubActivityEventResponse>> {
    let req = req.get_ref();

    let mut conn = create_connection(RDS::Event).await?;
    let mut tx = begin_tx(&mut conn).await?;

    for event in &req.events {
        let mut raw_event = GithubActivityEvent {
            id: 0,
            raw_data: event.raw_data.to_string(),
            event_time: DateTime::from_timestamp(event.event_time, 0)
                .ok_or(anyhow!("invalid event_time"))?,
            ..Default::default()
        };

        event::insert_github_activity_event(&mut tx, &mut raw_event).await?;
        if let Some(req_display_event) = &event.display_event {
            let mut display_event = DisplayEvent {
                id: 0,
                title: req_display_event.title.to_string(),
                message: req_display_event.message.to_string(),
                link: req_display_event.link.to_string(),
                event_time: DateTime::from_timestamp(event.event_time, 0)
                    .ok_or(anyhow!("invalid event_time"))?,
                ..Default::default()
            };
            event::insert_display_event(&mut tx, &mut display_event).await?;
        }
    }

    Ok(Response::new(CreateGithubActivityEventResponse {}))
}
