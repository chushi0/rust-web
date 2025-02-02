use axum::Extension;
use chrono::{Duration, Utc};
use common::tonic_idl_gen::ListDisplayEventRequest;
use server_common::rpc_client::CoreRpcServiceClient;
use tonic::Request;

use crate::{
    extract::{error::AppError, response::BodyResponse},
    model::home::{DisplayEvent, GetHomeEventResponse},
};

#[axum::debug_handler]
pub async fn events(
    Extension(mut core_rpc_client): Extension<CoreRpcServiceClient>,
) -> Result<BodyResponse<GetHomeEventResponse>, AppError> {
    let min_event_time = Utc::now() - Duration::days(30);

    let events = core_rpc_client
        .list_display_event(Request::new(ListDisplayEventRequest {
            offset: 0,
            count: 100,
            min_event_time: Some(min_event_time.timestamp()),
            ..Default::default()
        }))
        .await
        .map_err(anyhow::Error::new)?;

    Ok(BodyResponse::new(GetHomeEventResponse {
        events: events
            .into_inner()
            .events
            .into_iter()
            .map(|event| DisplayEvent {
                title: event.title,
                msg: event.message,
                time: event.event_time,
                link: event.link,
            })
            .collect(),
    }))
}
