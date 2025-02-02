use std::future::Future;

use anyhow::Result;
use chrono::{DateTime, Utc};
use server_common::db::{context::Context, count::Counter};
use sqlx::{prelude::FromRow, MySql, QueryBuilder};

#[derive(Debug, Clone, FromRow, Default)]
pub struct ServerConfig {
    pub id: u64,
    pub name: String,                 // 服务器配置名，用于管理
    pub mc_version: String,           // mc版本号
    pub world_uri: Option<String>,    // 存档地址
    pub resource_uri: Option<String>, // 资源包地址
    pub motd: String,                 // 服务器motd
    pub create_time: DateTime<Utc>,
    pub update_time: DateTime<Utc>,
}

pub struct ListServerConfigParameters {
    pub offset: u64,
    pub limit: u64,
}

pub enum UpdateServerConfig<'v> {
    Name(&'v str),
    McVersion(&'v str),
    WorldUri(Option<&'v String>),
    ResourceUri(Option<&'v String>),
    Motd(&'v str),
}

pub trait ServerConfigRepository {
    fn create_server_config(
        &mut self,
        server_config: &mut ServerConfig,
    ) -> impl Future<Output = Result<()>> + Send;

    fn list_server_config(
        &mut self,
        params: &ListServerConfigParameters,
    ) -> impl Future<Output = Result<Vec<ServerConfig>>> + Send;

    fn count_server_config(
        &mut self,
        params: &ListServerConfigParameters,
    ) -> impl Future<Output = Result<i64>> + Send;

    fn get_server_config_by_id(
        &mut self,
        id: u64,
    ) -> impl Future<Output = Result<Option<ServerConfig>>> + Send;

    fn get_server_config_by_name(
        &mut self,
        name: &str,
    ) -> impl Future<Output = Result<Option<ServerConfig>>> + Send;

    fn update_server_config(
        &mut self,
        id: u64,
        updates: &[UpdateServerConfig<'_>],
    ) -> impl Future<Output = Result<()>> + Send;

    fn delete_server_config(&mut self, id: u64) -> impl Future<Output = Result<()>> + Send;
}

impl ServerConfigRepository for Context<'_, MySql> {
    async fn create_server_config(&mut self, server_config: &mut ServerConfig) -> Result<()> {
        let result = sqlx::query(
            r#"
            INSERT INTO server_config (name, mc_version, world_uri, resource_uri, motd)
            VALUES (?, ?, ?, ?, ?)
            "#,
        )
        .bind(&server_config.name)
        .bind(&server_config.mc_version)
        .bind(&server_config.world_uri)
        .bind(&server_config.resource_uri)
        .bind(&server_config.motd)
        .execute(self)
        .await?;

        server_config.id = result.last_insert_id();

        Ok(())
    }

    async fn list_server_config(
        &mut self,
        params: &ListServerConfigParameters,
    ) -> Result<Vec<ServerConfig>> {
        let mut query = QueryBuilder::new("select * from server_config");
        query.push(" order by id");
        query
            .push(" limit ")
            .push(params.offset)
            .push(",")
            .push(params.limit);

        Ok(query.build_query_as().fetch_all(self).await?)
    }

    async fn count_server_config(&mut self, _params: &ListServerConfigParameters) -> Result<i64> {
        let mut query = QueryBuilder::new("select count(*) from server_config");
        let count: Counter = query.build_query_as().fetch_one(self).await?;
        Ok(count.count)
    }

    async fn get_server_config_by_id(&mut self, id: u64) -> Result<Option<ServerConfig>> {
        let server_config = sqlx::query_as("select * from server_config where id = ?")
            .bind(id)
            .fetch_optional(self)
            .await?;

        Ok(server_config)
    }

    async fn get_server_config_by_name(&mut self, name: &str) -> Result<Option<ServerConfig>> {
        let server_config = sqlx::query_as("select * from server_config where name = ?")
            .bind(name)
            .fetch_optional(self)
            .await?;

        Ok(server_config)
    }

    async fn update_server_config(
        &mut self,
        id: u64,
        updates: &[UpdateServerConfig<'_>],
    ) -> Result<()> {
        if updates.is_empty() {
            return Ok(());
        }

        let mut query = QueryBuilder::new("update server_config set");
        {
            let mut query_update = query.separated(", ");
            for update in updates {
                match update {
                    UpdateServerConfig::Name(name) => {
                        query_update.push("name = ").push_bind(name);
                    }
                    UpdateServerConfig::McVersion(mc_version) => {
                        query_update.push("mc_version = ").push_bind(mc_version);
                    }
                    UpdateServerConfig::WorldUri(world_uri) => {
                        query_update.push("world_uri = ").push_bind(world_uri);
                    }
                    UpdateServerConfig::ResourceUri(resource_uri) => {
                        query_update.push("resource_uri = ").push_bind(resource_uri);
                    }
                    UpdateServerConfig::Motd(motd) => {
                        query_update.push("motd = ").push_bind(motd);
                    }
                }
            }
        }
        query.push(" where id = ").push_bind(id);

        query.build().execute(self).await?;

        Ok(())
    }

    async fn delete_server_config(&mut self, id: u64) -> Result<()> {
        sqlx::query("delete from server_config where id = ?")
            .bind(id)
            .execute(self)
            .await?;

        Ok(())
    }
}
