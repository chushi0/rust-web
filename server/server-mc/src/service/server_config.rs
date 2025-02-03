use anyhow::{anyhow, Result};
use common::tonic_idl_gen::{
    CreateServerConfigRequest, CreateServerConfigResponse, DeleteServerConfigRequest,
    DeleteServerConfigResponse, ListServerConfigRequest, ListServerConfigResponse,
};
use const_format::concatcp;
use server_common::{
    db::context::{Context, ContextRef},
    external_api::aliyun::oss::{HttpOssClient, RUSTWEB_PREFIX},
};
use sqlx::Database;

use crate::{
    dao::server_config::{
        ListServerConfigParameters, ServerConfig, ServerConfigRepository, UpdateServerConfig,
    },
    process::manager::Manager,
};

const WORLD_URI_PREFIX: &str = concatcp!(RUSTWEB_PREFIX, "mc/world/");
const RESOURCE_URI_PREFIX: &str = concatcp!(RUSTWEB_PREFIX, "mc/resource/");

pub async fn create_server_config<DB: Database>(
    db: ContextRef<'_, '_, DB>,
    oss_client: HttpOssClient<'_, '_>,
    req: CreateServerConfigRequest,
) -> Result<CreateServerConfigResponse>
where
    for<'db> Context<'db, DB>: ServerConfigRepository,
{
    let mut server_config = ServerConfig {
        name: req.name,
        mc_version: req.version,
        motd: req.motd,
        ..Default::default()
    };

    let mut tx = db.begin().await?;
    tx.create_server_config(&mut server_config).await?;

    let mut updates = Vec::new();
    if let Some(world_uri) = req.world_uri {
        let permanent_world_uri = format!("{WORLD_URI_PREFIX}world-{}.zip", server_config.id);
        oss_client
            .copy_object(&world_uri, &permanent_world_uri)
            .await?;
        server_config.world_uri = Some(permanent_world_uri);
        updates.push(UpdateServerConfig::WorldUri(
            server_config.world_uri.as_ref(),
        ));
    }

    if let Some(resource_uri) = req.resource_uri {
        let permanent_resource_uri = format!("{RESOURCE_URI_PREFIX}world-{}.zip", server_config.id);
        oss_client
            .copy_object(&resource_uri, &permanent_resource_uri)
            .await?;
        server_config.resource_uri = Some(permanent_resource_uri);
        updates.push(UpdateServerConfig::ResourceUri(
            server_config.resource_uri.as_ref(),
        ));
    }

    tx.update_server_config(server_config.id, &updates).await?;
    tx.commit().await?;

    Ok(CreateServerConfigResponse {})
}

pub async fn list_server_config<DB: Database>(
    db: ContextRef<'_, '_, DB>,
    req: ListServerConfigRequest,
) -> Result<ListServerConfigResponse>
where
    for<'db> Context<'db, DB>: ServerConfigRepository,
{
    let params = ListServerConfigParameters {
        offset: req.offset,
        limit: req.count,
    };

    let server_configs = db
        .list_server_config(&params)
        .await?
        .into_iter()
        .map(ServerConfig::into)
        .collect();
    let count = db.count_server_config(&params).await?;

    Ok(ListServerConfigResponse {
        total: count,
        configs: server_configs,
    })
}

pub async fn delete_server_config<DB: Database>(
    db: ContextRef<'_, '_, DB>,
    oss_client: HttpOssClient<'_, '_>,
    manager: &Manager,
    req: DeleteServerConfigRequest,
) -> Result<DeleteServerConfigResponse>
where
    for<'db> Context<'db, DB>: ServerConfigRepository,
{
    let server_config = db
        .get_server_config_by_id(req.id)
        .await?
        .ok_or(anyhow!("server config not found"))?;

    db.delete_server_config(server_config.id).await?;

    // delete oss reources
    if let Some(world_uri) = &server_config.world_uri {
        oss_client.delete_object(world_uri).await?;
    }

    if let Some(resource_uri) = &server_config.resource_uri {
        oss_client.delete_object(resource_uri).await?;
    }

    // clean disk cache
    if manager
        .running_config()
        .await
        .is_some_and(|server_config| server_config.id == req.id)
    {
        manager.stop_server_config().await?;
    }

    manager.clean_world_cache(&server_config).await?;

    Ok(DeleteServerConfigResponse {})
}

impl From<ServerConfig> for common::tonic_idl_gen::ServerConfig {
    fn from(value: ServerConfig) -> Self {
        Self {
            id: value.id,
            name: value.name,
            version: value.mc_version,
            world_uri: value.world_uri,
            resource_uri: value.resource_uri,
            motd: value.motd,
        }
    }
}
