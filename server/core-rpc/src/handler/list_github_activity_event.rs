use anyhow::Result;
use idl_gen::core_rpc::{
    GithubActivityEvent, ListGithubActivityEventRequest, ListGithubActivityEventResponse,
};
use volo_grpc::{Request, Response};
use web_db::{begin_tx, create_connection, event, RDS};

pub async fn handle(
    _req: Request<ListGithubActivityEventRequest>,
) -> Result<Response<ListGithubActivityEventResponse>> {
    let mut conn = create_connection(RDS::Event).await?;
    let mut tx = begin_tx(&mut conn).await?;

    let github_activity_events = event::get_last_github_activity_event(&mut tx)
        .await?
        .as_slice()
        .to_vec();

    let mut api_github_activity_event = Vec::with_capacity(github_activity_events.len());

    for event in github_activity_events {
        api_github_activity_event.push(GithubActivityEvent {
            id: event.id,
            raw_data: event.raw_data.into(),
            event_time: event.event_time.timestamp(),
            create_time: event.create_time.timestamp(),
            update_time: event.update_time.timestamp(),
        });
    }

    Ok(Response::new(ListGithubActivityEventResponse {
        total: api_github_activity_event.len() as i64,
        events: api_github_activity_event,
    }))
}
