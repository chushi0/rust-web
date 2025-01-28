use std::{str::FromStr, time::Duration};

use anyhow::{anyhow, Error, Result};
use log::LevelFilter;
use sqlx::{
    mysql::MySqlConnectOptions, pool::PoolOptions, ConnectOptions, Connection, Database, Pool,
};
use tracing::info;

#[derive(Debug, Clone, Default)]
pub struct Config {
    pub host: Option<String>,
    pub port: Option<u16>,
    pub username: Option<String>,
    pub password: Option<String>,
    pub database: Option<String>,
}

const ENV_HOST: &'static str = "RUSTWEB_DB_HOST";
const ENV_PORT: &'static str = "RUSTWEB_DB_PORT";
const ENV_USERNAME: &'static str = "RUSTWEB_DB_USERNAME";
const ENV_PASSWORD: &'static str = "RUSTWEB_DB_PASSWORD";
const ENV_DATABASE: &'static str = "RUSTWEB_DB_DATABASE";

pub async fn create_pool_with<DB>(config: Config) -> Result<Pool<DB>>
where
    DB: Database,
    <DB::Connection as Connection>::Options: TryFrom<Config, Error = Error>,
{
    let pool_options = PoolOptions::new()
        .acquire_timeout(Duration::from_secs(1))
        .idle_timeout(Duration::from_secs(30));
    let connection_options = config.try_into()?;
    info!("connecting to pool with options: {:?}", connection_options);
    Ok(pool_options.connect_with(connection_options).await?)
}

fn from_env<T>(key: &str) -> Result<T>
where
    T: FromStr,
    <T as FromStr>::Err: std::error::Error + Send + Sync + 'static,
{
    Ok(std::env::var(key)?.parse()?)
}

impl TryFrom<Config> for MySqlConnectOptions {
    type Error = Error;

    fn try_from(value: Config) -> Result<Self> {
        let mut options = MySqlConnectOptions::new();

        options = options.host(
            &value
                .host
                .or_else(|| from_env(ENV_HOST).ok())
                .ok_or(anyhow!("host is not set"))?,
        );

        if let Some(port) = value.port.or_else(|| from_env(&ENV_PORT).ok()) {
            options = options.port(port);
        }

        options = options.username(
            &value
                .username
                .or_else(|| from_env(&ENV_USERNAME).ok())
                .ok_or(anyhow!("username is not set"))?,
        );

        options = options.password(
            &value
                .password
                .or_else(|| from_env(&ENV_PASSWORD).ok())
                .ok_or(anyhow!("password is not set"))?,
        );

        options = options.database(
            &value
                .database
                .or_else(|| from_env(&ENV_DATABASE).ok())
                .ok_or(anyhow!("database is not set"))?,
        );

        options = options
            .log_statements(LevelFilter::Info)
            .log_slow_statements(LevelFilter::Warn, Duration::from_secs(1));

        Ok(options)
    }
}
