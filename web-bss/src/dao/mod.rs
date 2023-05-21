use anyhow::*;
use sqlx::{Connection, SqliteConnection};

pub mod event;

type Conn = SqliteConnection;

async fn create_connection(db: &str) -> Result<Conn> {
    let path = if cfg!(debug_assertions) {
        format!("./db/{db}.db")
    } else {
        format!("/home/chushi0/db/{db}.db")
    };

    Ok(SqliteConnection::connect(&path).await?)
}
