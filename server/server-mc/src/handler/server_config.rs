use common::tonic_idl_gen::{
    CreateServerConfigRequest, CreateServerConfigResponse, DeleteServerConfigRequest,
    DeleteServerConfigResponse, ListServerConfigRequest, ListServerConfigResponse,
};
use server_common::db::context::Context;
use tonic::{Response, Status};

use crate::{service, Service};

pub async fn create_server_config(
    service: &Service,
    req: CreateServerConfigRequest,
) -> Result<Response<CreateServerConfigResponse>, Status> {
    let result = service::server_config::create_server_config(
        &mut Context::PoolRef(&service.db),
        service.oss_client.with_http(&service.http_client),
        req,
    )
    .await;

    match result {
        Ok(response) => Ok(Response::new(response)),
        Err(err) => Err(Status::internal(err.to_string())),
    }
}

pub async fn list_server_config(
    service: &Service,
    req: ListServerConfigRequest,
) -> Result<Response<ListServerConfigResponse>, Status> {
    let result =
        service::server_config::list_server_config(&mut Context::PoolRef(&service.db), req).await;

    match result {
        Ok(response) => Ok(Response::new(response)),
        Err(err) => Err(Status::internal(err.to_string())),
    }
}

pub async fn delete_server_config(
    service: &Service,
    req: DeleteServerConfigRequest,
) -> Result<Response<DeleteServerConfigResponse>, Status> {
    let result = service::server_config::delete_server_config(
        &mut Context::PoolRef(&service.db),
        service.oss_client.with_http(&service.http_client),
        req,
    )
    .await;

    match result {
        Ok(response) => Ok(Response::new(response)),
        Err(err) => Err(Status::internal(err.to_string())),
    }
}
