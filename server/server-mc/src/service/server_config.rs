use anyhow::{anyhow, Result};
use common::tonic_idl_gen::{
    CreateServerConfigRequest, CreateServerConfigResponse, DeleteServerConfigRequest,
    DeleteServerConfigResponse, ListServerConfigRequest, ListServerConfigResponse,
};
use server_common::{
    db::context::{Context, ContextRef},
    external_api::aliyun::oss::HttpOssClient,
};
use sqlx::Database;

use crate::dao::server_config::{
    ListServerConfigParameters, ServerConfig, ServerConfigRepository, UpdateServerConfig,
};

const WORLD_URI_PREFIX: &str = "rust-web/mc/world/";
const RESOURCE_URI_PREFIX: &str = "rust-web/mc/resource/";

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
        let permanent_world_uri = format!("{WORLD_URI_PREFIX}/world-{}.zip", server_config.id);
        oss_client
            .copy_object(&world_uri, &permanent_world_uri)
            .await?;
        server_config.world_uri = Some(permanent_world_uri);
        updates.push(UpdateServerConfig::WorldUri(
            server_config.world_uri.as_ref(),
        ));
    }

    if let Some(resource_uri) = req.resource_uri {
        let permanent_resource_uri =
            format!("{RESOURCE_URI_PREFIX}/world-{}.zip", server_config.id);
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
        .map(|server_config| common::tonic_idl_gen::ServerConfig {
            id: server_config.id,
            name: server_config.name,
            version: server_config.mc_version,
            world_uri: server_config.world_uri,
            resource_uri: server_config.resource_uri,
            motd: server_config.motd,
        })
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
    if let Some(world_uri) = server_config.world_uri {
        oss_client.delete_object(&world_uri).await?;
    }

    if let Some(resource_uri) = server_config.resource_uri {
        oss_client.delete_object(&resource_uri).await?;
    }

    // TODO: clean disk cache

    Ok(DeleteServerConfigResponse {})
}
