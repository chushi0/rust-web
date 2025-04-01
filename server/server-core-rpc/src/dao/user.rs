use std::future::Future;

use anyhow::Result;
use chrono::{DateTime, Utc};
use server_common::db::context::Context;
use sqlx::{prelude::FromRow, MySql};

#[derive(Debug, Clone, FromRow, Default)]
pub struct User {
    pub id: u64,
    pub username: String,
    pub password: String,
    pub create_time: DateTime<Utc>,
    pub update_time: DateTime<Utc>,
}

pub trait UserRepository {
    fn create_user(&mut self, user: &mut User) -> impl Future<Output = Result<()>> + Send;

    fn query_user_by_username(
        &mut self,
        username: &str,
    ) -> impl Future<Output = Result<Option<User>>> + Send;

    fn query_user_by_id(&mut self, id: u64) -> impl Future<Output = Result<Option<User>>> + Send;
}

impl UserRepository for Context<'_, MySql> {
    async fn create_user(&mut self, user: &mut User) -> Result<()> {
        let result = sqlx::query("insert into user (username, password) values (?, ?)")
            .bind(&user.username)
            .bind(&user.password)
            .execute(self)
            .await?;
        user.id = result.last_insert_id();
        Ok(())
    }

    async fn query_user_by_username(&mut self, username: &str) -> Result<Option<User>> {
        let result = sqlx::query_as("select * from user where username = ?")
            .bind(username)
            .fetch_optional(self)
            .await?;

        Ok(result)
    }

    async fn query_user_by_id(&mut self, id: u64) -> Result<Option<User>> {
        let result = sqlx::query_as("select * from user where id = ?")
            .bind(id)
            .fetch_optional(self)
            .await?;

        Ok(result)
    }
}
