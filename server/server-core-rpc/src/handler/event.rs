use common::tonic_idl_gen::{
    CreateGithubActivityEventRequest, CreateGithubActivityEventResponse, ListDisplayEventRequest,
    ListDisplayEventResponse, ListGithubActivityEventRequest, ListGithubActivityEventResponse,
};
use server_common::db::context::Context;
use tonic::{Request, Response, Status};

use crate::{service, Service};

pub async fn list_display_event(
    service: &Service,
    request: Request<ListDisplayEventRequest>,
) -> Result<Response<ListDisplayEventResponse>, Status> {
    if request.get_ref().count <= 0 {
        return Err(Status::invalid_argument("count must be greater than 0"));
    }

    if request.get_ref().offset < 0 {
        return Err(Status::invalid_argument(
            "offset must be greater than or equal to 0",
        ));
    }

    if request.get_ref().count > 100 {
        return Err(Status::invalid_argument(
            "count must be less than or equal to 100",
        ));
    }

    if request.get_ref().offset > 1000 {
        return Err(Status::invalid_argument(
            "offset must be less than or equal to 1000",
        ));
    }

    if request
        .get_ref()
        .min_event_time
        .is_some_and(|time| time <= 0)
    {
        return Err(Status::invalid_argument(
            "min_event_time must be greater than 0",
        ));
    }

    if request
        .get_ref()
        .max_event_time
        .is_some_and(|time| time <= 0)
    {
        return Err(Status::invalid_argument(
            "max_event_time must be greater than 0",
        ));
    }

    if request
        .get_ref()
        .min_event_time
        .is_some_and(|time| time >= 32503680000)
    {
        return Err(Status::invalid_argument(
            "min_event_time must be less than or equal to 32503680000",
        ));
    }

    if request
        .get_ref()
        .max_event_time
        .is_some_and(|time| time >= 32503680000)
    {
        return Err(Status::invalid_argument(
            "max_event_time must be less than or equal to 32503680000",
        ));
    }

    let result = service::event::list_display_event(
        &mut Context::PoolRef(&service.db),
        request.into_inner(),
    )
    .await;
    match result {
        Ok(response) => Ok(Response::new(response)),
        Err(err) => Err(Status::internal(err.to_string())),
    }
}

pub async fn list_github_activity_event(
    service: &Service,
    request: Request<ListGithubActivityEventRequest>,
) -> Result<Response<ListGithubActivityEventResponse>, Status> {
    if request.get_ref().count <= 0 {
        return Err(Status::invalid_argument("count must be greater than 0"));
    }

    if request.get_ref().offset < 0 {
        return Err(Status::invalid_argument(
            "offset must be greater than or equal to 0",
        ));
    }

    if request.get_ref().count > 100 {
        return Err(Status::invalid_argument(
            "count must be less than or equal to 100",
        ));
    }

    if request.get_ref().offset > 1000 {
        return Err(Status::invalid_argument(
            "offset must be less than or equal to 1000",
        ));
    }
    let result = service::event::list_github_activity_event(
        &mut Context::PoolRef(&service.db),
        request.into_inner(),
    )
    .await;
    match result {
        Ok(response) => Ok(Response::new(response)),
        Err(err) => Err(Status::internal(err.to_string())),
    }
}

pub async fn create_github_activity_event(
    service: &Service,
    request: Request<CreateGithubActivityEventRequest>,
) -> Result<Response<CreateGithubActivityEventResponse>, Status> {
    let mut db = Context::PoolRef(&service.db);
    let mut tx = db
        .begin()
        .await
        .map_err(|e| Status::internal(format!("failed to start transaction: {}", e.to_string())))?;

    for event in request.into_inner().events {
        let result = service::event::create_github_activity_event(&mut tx, event).await;
        match result {
            Ok(_) => {}
            Err(err) => {
                return Err(Status::internal(err.to_string()));
            }
        }
    }

    tx.commit().await.map_err(|e| {
        Status::internal(format!("failed to commit transaction: {}", e.to_string()))
    })?;

    Ok(Response::new(CreateGithubActivityEventResponse {}))
}
