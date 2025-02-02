use axum::Extension;
use common::tonic_idl_gen::McVersionType;
use server_common::rpc_client::McServiceClient;

use crate::{
    extract::{
        encrypt_request::{EncryptBodyRequest, EncryptQueryRequest},
        error::AppError,
        response::BodyResponse,
    },
    model::mc::{
        CreateServerConfigRequest, ListMcVersionRequest, ListMcVersionResponse,
        ListServerConfigRequest, ListServerConfigResponse, McVersion, ServerConfig,
    },
};

use super::validate::validate_upload_oss_uri;

#[axum::debug_handler]
pub async fn list_mc_version(
    Extension(mut mc_client): Extension<McServiceClient>,
    EncryptQueryRequest(req): EncryptQueryRequest<ListMcVersionRequest>,
) -> Result<BodyResponse<ListMcVersionResponse>, AppError> {
    if req.offset > 10000 {
        return Err(AppError::BadRequest("Invalid request offset"));
    }

    if req.limit > 100 {
        return Err(AppError::BadRequest("Invalid request limit"));
    }

    let get_versions = mc_client
        .list_mc_version(common::tonic_idl_gen::ListMcVersionRequest {
            offset: req.offset,
            count: req.limit,
            has_snapshot: Some(req.has_snapshot),
        })
        .await?
        .into_inner();

    Ok(BodyResponse::new(ListMcVersionResponse {
        count: get_versions.total,
        versions: get_versions
            .versions
            .into_iter()
            .map(|mc_version| McVersion {
                id: mc_version.id,
                snapshot: mc_version.r#type == McVersionType::Snapshot as i32,
            })
            .collect(),
    }))
}

#[axum::debug_handler]
pub async fn create_server_config(
    Extension(mut mc_client): Extension<McServiceClient>,
    EncryptBodyRequest(req): EncryptBodyRequest<CreateServerConfigRequest>,
) -> Result<BodyResponse<()>, AppError> {
    // TODO: check version is exist

    if req.name.is_empty() {
        return Err(AppError::BadRequest("invalid name"));
    }

    if req.name.len() > 100 {
        return Err(AppError::BadRequest("name too long"));
    }

    if req
        .world_uri
        .as_ref()
        .is_some_and(|uri| !validate_upload_oss_uri(uri))
    {
        return Err(AppError::BadRequest("invalid world_uri"));
    }

    if req
        .world_uri
        .as_ref()
        .is_some_and(|uri| !validate_upload_oss_uri(uri))
    {
        return Err(AppError::BadRequest("invalid resource_uri"));
    }

    mc_client
        .create_server_config(common::tonic_idl_gen::CreateServerConfigRequest {
            name: req.name,
            version: req.version,
            world_uri: req.world_uri,
            resource_uri: req.resource_uri,
            motd: req.motd,
        })
        .await?;

    Ok(BodyResponse::new(()))
}

#[axum::debug_handler]
pub async fn list_server_config(
    Extension(mut mc_client): Extension<McServiceClient>,
    EncryptQueryRequest(req): EncryptQueryRequest<ListServerConfigRequest>,
) -> Result<BodyResponse<ListServerConfigResponse>, AppError> {
    if req.offset > 10000 {
        return Err(AppError::BadRequest("Invalid request offset"));
    }

    if req.limit > 100 {
        return Err(AppError::BadRequest("Invalid request limit"));
    }

    let list_configs = mc_client
        .list_server_config(common::tonic_idl_gen::ListServerConfigRequest {
            offset: req.offset,
            count: req.limit,
        })
        .await?
        .into_inner();

    Ok(BodyResponse::new(ListServerConfigResponse {
        count: list_configs.total,
        configs: list_configs
            .configs
            .into_iter()
            .map(|config| ServerConfig {
                id: config.id,
                name: config.name,
                version: config.version,
                motd: config.motd,
            })
            .collect(),
    }))
}
