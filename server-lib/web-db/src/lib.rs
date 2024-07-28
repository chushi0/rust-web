use anyhow::Result;
use sqlx::{mysql::MySqlConnectOptions, Connection, MySql, MySqlConnection};

pub mod bilibili;
pub mod event;
pub mod furuyoni;
pub mod hearthstone;
pub mod mc_config;
pub mod user;

pub enum RDS {
    User,
    Event,
    Furuyoni,
    Hearthstone,
    McConfig,
    Bilibili,
}

impl RDS {
    fn database_name(&self) -> &'static str {
        match self {
            RDS::User => "user",
            RDS::Event => "event",
            RDS::Furuyoni => "furuyoni",
            RDS::Hearthstone => "heartstone",
            RDS::McConfig => "mc-config",
            RDS::Bilibili => "bilibili",
        }
    }
}

pub struct Transaction<'a> {
    tx: sqlx::Transaction<'a, MySql>,
}

pub async fn create_connection(rds: RDS) -> Result<MySqlConnection> {
    let db = rds.database_name();
    let username = std::env::var("RUST_WEB_DB_USERNAME")?;
    let password = std::env::var("RUST_WEB_DB_PASSWORD")?;

    let db_option = MySqlConnectOptions::new()
        .host("rustweb.chushi0.mysql")
        .username(&username)
        .password(&password)
        .database(db);

    Ok(MySqlConnection::connect_with(&db_option).await?)
}

pub async fn create_connection_with_path(path: &str) -> Result<MySqlConnection> {
    Ok(MySqlConnection::connect(path).await?)
}

pub async fn begin_tx(connection: &mut MySqlConnection) -> Result<Transaction<'_>> {
    Ok(Transaction {
        tx: connection.begin().await?,
    })
}

impl<'a> Transaction<'a> {
    pub async fn commit(self) -> Result<()> {
        Ok(self.tx.commit().await?)
    }

    pub async fn rollback(self) -> Result<()> {
        Ok(self.tx.rollback().await?)
    }
}
