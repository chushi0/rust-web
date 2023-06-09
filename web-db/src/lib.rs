use anyhow::Result;
use sqlx::{Connection, Sqlite, SqliteConnection};

pub mod event;
pub mod mc_config;
pub enum RDS {
    Event,
    McConfig,
}

fn rds_name(rds: RDS) -> &'static str {
    match rds {
        RDS::Event => "event",
        RDS::McConfig => "mc-config",
    }
}

pub struct Transaction<'a> {
    tx: sqlx::Transaction<'a, Sqlite>,
}

pub async fn create_connection(rds: RDS) -> Result<SqliteConnection> {
    let db = rds_name(rds);
    let path = if cfg!(debug_assertions) {
        format!("../db/{db}.db")
    } else {
        format!("/home/chushi0/db/{db}.db")
    };

    Ok(SqliteConnection::connect(&format!("sqlite://{path}")).await?)
}

pub async fn begin_tx<'a>(connection: &'a mut SqliteConnection) -> Result<Transaction<'a>> {
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
