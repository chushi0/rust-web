use anyhow::Result;
use sqlx::{Connection, Sqlite, SqliteConnection};

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

fn rds_name(rds: RDS) -> &'static str {
    match rds {
        RDS::User => "user",
        RDS::Event => "event",
        RDS::Furuyoni => "furuyoni",
        RDS::Hearthstone => "heartstone",
        RDS::McConfig => "mc-config",
        RDS::Bilibili => "bilibili",
    }
}

pub struct Transaction<'a> {
    tx: sqlx::Transaction<'a, Sqlite>,
}

pub async fn create_connection(rds: RDS) -> Result<SqliteConnection> {
    let db = rds_name(rds);
    let path = format!("{}/{db}.db", env!("RUST_WEB_DB_PATH"));

    Ok(SqliteConnection::connect(&format!("sqlite://{path}")).await?)
}

pub async fn create_connection_with_path(path: &str) -> Result<SqliteConnection> {
    Ok(SqliteConnection::connect(path).await?)
}

pub async fn begin_tx(connection: &mut SqliteConnection) -> Result<Transaction<'_>> {
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
