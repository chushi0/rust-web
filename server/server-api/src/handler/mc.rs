use axum::{extract::Query, http::StatusCode, response::Redirect, Extension};
use chrono::Duration;
use common::tonic_idl_gen::McVersionType;
use server_common::{external_api::aliyun::oss::OssClient, rpc_client::McServiceClient};

use crate::{
    extract::{
        encrypt_request::{EncryptBodyRequest, EncryptQueryRequest},
        error::AppError,
        response::BodyResponse,
    },
    model::mc::{
        CreateServerConfigRequest, GetCurrentServerConfigResponse, GetResourcePackRequest,
        ListMcVersionRequest, ListMcVersionResponse, ListServerConfigRequest,
        ListServerConfigResponse, McVersion, RunningServerStage, RunningServerStageInfo,
        ServerConfig, StartServerConfigRequest,
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

#[axum::debug_handler]
pub async fn start_server_config(
    Extension(mut mc_client): Extension<McServiceClient>,
    EncryptBodyRequest(req): EncryptBodyRequest<StartServerConfigRequest>,
) -> Result<BodyResponse<()>, AppError> {
    mc_client
        .start_server_config(common::tonic_idl_gen::StartServerConfigRequest { id: req.id })
        .await?;

    Ok(BodyResponse::new(()))
}

#[axum::debug_handler]
pub async fn stop_server_config(
    Extension(mut mc_client): Extension<McServiceClient>,
    EncryptBodyRequest(_req): EncryptBodyRequest<()>,
) -> Result<BodyResponse<()>, AppError> {
    mc_client
        .stop_server_config(common::tonic_idl_gen::StopServerConfigRequest {})
        .await?;

    Ok(BodyResponse::new(()))
}

#[axum::debug_handler]
pub async fn get_current_server_config(
    Extension(mut mc_client): Extension<McServiceClient>,
    EncryptQueryRequest(_req): EncryptQueryRequest<()>,
) -> Result<BodyResponse<GetCurrentServerConfigResponse>, AppError> {
    let current_server_config = mc_client
        .get_current_server_config(common::tonic_idl_gen::GetCurrentServerConfigRequest {})
        .await?
        .into_inner();

    Ok(BodyResponse::new(GetCurrentServerConfigResponse {
        running_config: current_server_config
            .running_config
            .map(|server_config| ServerConfig {
                id: server_config.id,
                name: server_config.name,
                version: server_config.version,
                motd: server_config.motd,
            }),
        status: current_server_config
            .status
            .map(|status| {
                status
                    .stage_info
                    .into_iter()
                    .filter_map(|info| {
                        // FIXME: extract this code and avoid repeat
                        Some((
                            match info.stage {
                                v if v
                                    == common::tonic_idl_gen::RunningServerStage::Init as i32 =>
                                {
                                    RunningServerStage::Init
                                }
                                v if v
                                    == common::tonic_idl_gen::RunningServerStage::PullingServer
                                        as i32 =>
                                {
                                    RunningServerStage::PullingServer
                                }
                                v if v
                                    == common::tonic_idl_gen::RunningServerStage::PullingWorld
                                        as i32 =>
                                {
                                    RunningServerStage::PullingWorld
                                }
                                v if v
                                    == common::tonic_idl_gen::RunningServerStage::InitializingFile
                                        as i32 =>
                                {
                                    RunningServerStage::InitializingFile
                                }
                                v if v
                                    == common::tonic_idl_gen::RunningServerStage::Starting
                                        as i32 =>
                                {
                                    RunningServerStage::Starting
                                }
                                v if v
                                    == common::tonic_idl_gen::RunningServerStage::Running
                                        as i32 =>
                                {
                                    RunningServerStage::Running
                                }
                                v if v
                                    == common::tonic_idl_gen::RunningServerStage::Stopping
                                        as i32 =>
                                {
                                    RunningServerStage::Stopping
                                }
                                v if v
                                    == common::tonic_idl_gen::RunningServerStage::Stopped
                                        as i32 =>
                                {
                                    RunningServerStage::Stopped
                                }

                                _ => return None,
                            },
                            RunningServerStageInfo {
                                enter_time: info.enter_time,
                                finish_time: info.finish_time,
                                in_error: info.in_error,
                                error_message: info.error_message,
                            },
                        ))
                    })
                    .collect()
            })
            .unwrap_or_default(),
    }))
}

#[axum::debug_handler]
pub async fn get_resource_pack(
    Extension(mut mc_client): Extension<McServiceClient>,
    Extension(oss_client): Extension<OssClient>,
    Query(req): Query<GetResourcePackRequest>,
) -> Result<Redirect, StatusCode> {
    let current_server_config = mc_client
        .get_current_server_config(common::tonic_idl_gen::GetCurrentServerConfigRequest {})
        .await
        .map_err(|_e| StatusCode::INTERNAL_SERVER_ERROR)?
        .into_inner();

    let Some(running_config) = current_server_config.running_config else {
        return Err(StatusCode::NOT_FOUND);
    };

    if running_config.id != req.id {
        return Err(StatusCode::NOT_FOUND);
    }

    let Some(resource_uri) = running_config.resource_uri else {
        return Err(StatusCode::NOT_FOUND);
    };

    let download_url = oss_client.download_url(&resource_uri, Duration::hours(1));
    Ok(Redirect::to(&download_url))
}
