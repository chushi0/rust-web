use chrono::{Duration, Utc};
use common::tonic_idl_gen::{
    core_rpc_service_client::CoreRpcServiceClient, ListDisplayEventRequest,
};
use tonic::Request;

use crate::model::{
    home::{DisplayEvent, GetHomeEventResponse},
    AppError, BodyResponse,
};

#[axum::debug_handler]
pub async fn events() -> Result<BodyResponse<GetHomeEventResponse>, AppError> {
    let min_event_time = Utc::now() - Duration::days(30);

    let mut core_rpc_client =
        CoreRpcServiceClient::connect("http://core-rpc-service.default.svc.cluster.local:13000")
            .await
            .map_err(anyhow::Error::new)?;

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
