use std::future::Future;

use anyhow::Result;
use server_common::db::{bound_separated::BoundSeparatedHelper, context::Context, count::Counter};
use sqlx::{
    prelude::FromRow,
    types::chrono::{DateTime, Utc},
    MySql, QueryBuilder,
};

#[derive(Debug, Clone, FromRow, Default)]
pub struct DisplayEvent {
    pub id: u64,
    pub title: String,
    pub message: String,
    pub link: String,
    pub event_time: DateTime<Utc>,
    pub create_time: DateTime<Utc>,
    pub update_time: DateTime<Utc>,
}

pub struct ListDisplayEventParameters {
    pub offset: i64,
    pub limit: i64,
    pub min_event_time: Option<DateTime<Utc>>,
    pub max_event_time: Option<DateTime<Utc>>,
}

pub trait DisplayEventRepository {
    fn create_display_event(
        &mut self,
        event: &mut DisplayEvent,
    ) -> impl Future<Output = Result<()>> + Send;

    fn list_display_event(
        &mut self,
        params: &ListDisplayEventParameters,
    ) -> impl Future<Output = Result<Vec<DisplayEvent>>> + Send;

    fn count_display_event(
        &mut self,
        params: &ListDisplayEventParameters,
    ) -> impl Future<Output = Result<i64>> + Send;
}

impl DisplayEventRepository for Context<'_, MySql> {
    async fn create_display_event(&mut self, event: &mut DisplayEvent) -> Result<()> {
        let result = sqlx::query(
            "insert into display_event (title, message, link, event_time) values (?, ?, ?, ?)",
        )
        .bind(&event.title)
        .bind(&event.message)
        .bind(&event.link)
        .bind(event.event_time)
        .execute(self)
        .await?;

        event.id = result.last_insert_id();

        Ok(())
    }

    async fn list_display_event(
        &mut self,
        params: &ListDisplayEventParameters,
    ) -> Result<Vec<DisplayEvent>> {
        let mut query = QueryBuilder::new("select * from display_event");
        params.append_where_clause(&mut query);
        query.push(" order by event_time desc");
        query
            .push(" limit ")
            .push_bind(params.offset)
            .push(", ")
            .push_bind(params.limit);

        Ok(query.build_query_as().fetch_all(self).await?)
    }

    async fn count_display_event(&mut self, params: &ListDisplayEventParameters) -> Result<i64> {
        let mut query = QueryBuilder::new("select count(*) from display_event");
        params.append_where_clause(&mut query);
        let count: Counter = query.build_query_as().fetch_one(self).await?;
        Ok(count.count)
    }
}

impl ListDisplayEventParameters {
    fn append_where_clause<'args>(&'args self, query_builder: &mut QueryBuilder<'args, MySql>) {
        let mut where_clause = query_builder.bound_separated(" where", "", " and");

        if let Some(min_event_time) = &self.min_event_time {
            where_clause
                .push(" event_time > ")
                .push_bind_unseparated(min_event_time);
        }

        if let Some(max_event_time) = &self.max_event_time {
            where_clause
                .push(" event_time < ")
                .push_bind_unseparated(max_event_time);
        }
    }
}
