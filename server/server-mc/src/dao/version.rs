use std::future::Future;

use anyhow::{anyhow, Result};
use chrono::{DateTime, Utc};
use server_common::{
    db::{
        bound_separated::BoundSeparatedHelper, context::Context, count::Counter,
        dbtype::DBTypeConvertError,
    },
    impl_sqlx_type,
};
use sqlx::{prelude::FromRow, MySql, QueryBuilder};
use strum_macros::FromRepr;

#[derive(Debug, Clone, FromRow, Default)]
pub struct Version {
    pub id: u64,
    pub mc_id: String,
    pub r#type: VersionType,
    pub server_url: String,
    pub release_time: DateTime<Utc>,
    pub create_time: DateTime<Utc>,
    pub update_time: DateTime<Utc>,
}

#[repr(u32)]
#[derive(Debug, Clone, Copy, Default, FromRepr)]
pub enum VersionType {
    #[default]
    Release = 1,
    Snapshot = 2,
    OldBeta = 3,
    OldAlpha = 4,
}

pub struct ListVersionParameters {
    pub offset: i64,
    pub limit: i64,
    pub has_snapshot: bool,
}

pub trait VersionRepository {
    fn create_version(&mut self, version: &mut Version) -> impl Future<Output = Result<()>> + Send;

    fn list_version(
        &mut self,
        params: &ListVersionParameters,
    ) -> impl Future<Output = Result<Vec<Version>>> + Send;

    fn count_version(
        &mut self,
        params: &ListVersionParameters,
    ) -> impl Future<Output = Result<i64>> + Send;

    fn get_version_by_mcid(
        &mut self,
        mcid: &str,
    ) -> impl Future<Output = Result<Option<Version>>> + Send;
}

impl_sqlx_type!(VersionType, u32);

impl From<&VersionType> for u32 {
    fn from(v: &VersionType) -> u32 {
        *v as u32
    }
}

impl TryFrom<u32> for VersionType {
    type Error = DBTypeConvertError;

    fn try_from(value: u32) -> Result<Self, Self::Error> {
        VersionType::from_repr(value).ok_or(DBTypeConvertError::Anyhow(anyhow!(
            "Invalid version: {value}"
        )))
    }
}

impl VersionRepository for Context<'_, MySql> {
    async fn create_version(&mut self, version: &mut Version) -> Result<()> {
        let result = sqlx::query(
            "insert into mc_version (mc_id, type, server_url, release_time) values (?, ?, ?, ?)",
        )
        .bind(&version.mc_id)
        .bind(version.r#type)
        .bind(&version.server_url)
        .bind(version.release_time)
        .execute(self)
        .await?;

        version.id = result.last_insert_id();

        Ok(())
    }

    async fn list_version(&mut self, params: &ListVersionParameters) -> Result<Vec<Version>> {
        let mut query = QueryBuilder::new("select * from mc_version");
        params.append_where_clause(&mut query);
        query.push(" order by release_time desc, id desc");
        query
            .push(" limit ")
            .push_bind(params.offset)
            .push(", ")
            .push_bind(params.limit);

        Ok(query.build_query_as().fetch_all(self).await?)
    }

    async fn count_version(&mut self, params: &ListVersionParameters) -> Result<i64> {
        let mut query = QueryBuilder::new("select count(*) from mc_version");
        params.append_where_clause(&mut query);
        let count: Counter = query.build_query_as().fetch_one(self).await?;
        Ok(count.count)
    }

    async fn get_version_by_mcid(&mut self, mcid: &str) -> Result<Option<Version>> {
        let version = sqlx::query_as("select * from mc_version where mc_id = ?")
            .bind(mcid)
            .fetch_optional(self)
            .await?;

        Ok(version)
    }
}

impl ListVersionParameters {
    fn append_where_clause<'args>(&'args self, query_builder: &mut QueryBuilder<'args, MySql>) {
        let mut where_clause = query_builder.bound_separated(" where", "", " and");
        if !self.has_snapshot {
            where_clause.push(" type = 1");
        }
    }
}
