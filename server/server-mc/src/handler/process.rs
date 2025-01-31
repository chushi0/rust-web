use common::tonic_idl_gen::*;
use server_common::db::context::Context;
use tonic::{Response, Status};

use crate::{service, Service};

pub async fn start_server_config(
    service: &Service,
    req: StartServerConfigRequest,
) -> Result<Response<StartServerConfigResponse>, Status> {
    let result = service::process::start_server_config(
        &mut Context::PoolRef(&service.db),
        &service.process_manager,
        req,
    )
    .await;

    match result {
        Ok(response) => Ok(Response::new(response)),
        Err(err) => Err(Status::internal(err.to_string())),
    }
}

pub async fn stop_server_config(
    service: &Service,
    req: StopServerConfigRequest,
) -> Result<Response<StopServerConfigResponse>, Status> {
    let result = service::process::stop_server_config(&service.process_manager, req).await;

    match result {
        Ok(response) => Ok(Response::new(response)),
        Err(err) => Err(Status::internal(err.to_string())),
    }
}

pub async fn get_current_server_config(
    service: &Service,
    req: GetCurrentServerConfigRequest,
) -> Result<Response<GetCurrentServerConfigResponse>, Status> {
    let result = service::process::get_current_server_config(&service.process_manager, req).await;

    match result {
        Ok(response) => Ok(Response::new(response)),
        Err(err) => Err(Status::internal(err.to_string())),
    }
}
