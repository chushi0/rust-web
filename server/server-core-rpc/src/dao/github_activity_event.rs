use std::future::Future;

use anyhow::Result;
use server_common::db::{context::Context, count::Counter};
use sqlx::{
    prelude::FromRow,
    types::chrono::{DateTime, Utc},
    MySql, QueryBuilder,
};

#[derive(Debug, Clone, FromRow, Default)]
pub struct GithubActivityEvent {
    pub id: u64,
    pub raw_data: String,
    pub event_time: DateTime<Utc>,
    pub create_time: DateTime<Utc>,
    pub update_time: DateTime<Utc>,
}

pub struct ListGithubActivityEventParameters {
    pub offset: i64,
    pub limit: i64,
    pub order_by_event_time_desc: bool,
}

pub trait GithubActivityEventRepository {
    fn create_github_activity_event(
        &mut self,
        event: &mut GithubActivityEvent,
    ) -> impl Future<Output = Result<()>> + Send;

    fn list_github_activity_event(
        &mut self,
        params: &ListGithubActivityEventParameters,
    ) -> impl Future<Output = Result<Vec<GithubActivityEvent>>> + Send;

    fn count_github_activity_event(&mut self) -> impl Future<Output = Result<i64>> + Send;
}

impl GithubActivityEventRepository for Context<'_, MySql> {
    async fn create_github_activity_event(
        &mut self,
        event: &mut GithubActivityEvent,
    ) -> Result<()> {
        let result =
            sqlx::query("insert into github_activity_event (raw_data, event_time) values (?, ?)")
                .bind(&event.raw_data)
                .bind(event.event_time)
                .execute(self)
                .await?;

        event.id = result.last_insert_id();

        Ok(())
    }

    async fn list_github_activity_event(
        &mut self,
        params: &ListGithubActivityEventParameters,
    ) -> Result<Vec<GithubActivityEvent>> {
        let mut query = QueryBuilder::new("select * from github_activity_event");

        if params.order_by_event_time_desc {
            query.push(" order by event_time desc");
        }

        query
            .push(" limit ")
            .push_bind(params.offset)
            .push(", ")
            .push_bind(params.limit);

        let events = query.build_query_as().fetch_all(self).await?;

        Ok(events)
    }

    async fn count_github_activity_event(&mut self) -> Result<i64> {
        let count: Counter = sqlx::query_as("select count(*) from github_activity_event")
            .fetch_one(self)
            .await?;

        Ok(count.count)
    }
}
