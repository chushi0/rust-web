use common::tonic_idl_gen::*;
use server_common::db::context::Context;
use tonic::{Response, Status};

use crate::{service, Service};

pub async fn list_mc_version(
    service: &Service,
    req: ListMcVersionRequest,
) -> Result<Response<ListMcVersionResponse>, Status> {
    let result = service::version::list_mc_version(&mut Context::PoolRef(&service.db), req).await;
    match result {
        Ok(response) => Ok(Response::new(response)),
        Err(err) => Err(Status::internal(err.to_string())),
    }
}

pub async fn sync_mc_version(
    service: &Service,
    _req: SyncMcVersionRequest,
) -> Result<Response<SyncMcVersionResponse>, Status> {
    let result =
        service::version::sync_version(&service.http_client, &mut Context::PoolRef(&service.db))
            .await;
    match result {
        Ok(_) => Ok(Response::new(SyncMcVersionResponse {})),
        Err(err) => Err(Status::internal(err.to_string())),
    }
}
