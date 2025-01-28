use std::net::{IpAddr, Ipv4Addr, SocketAddr};

use anyhow::Result;
use common::tonic_idl_gen::core_rpc_service_server::CoreRpcServiceServer;
use server_common::db::pool::{create_pool_with, Config};
use sqlx::{MySql, Pool};
use tonic::transport::Server;
use tracing::info;

pub mod dao;
pub mod handler;
pub mod service;

pub struct Service {
    pub db: Pool<MySql>,
}

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt::init();

    let service = Service::new().await.expect("initialize service failed");

    info!("starting service...");
    Server::builder()
        .add_service(CoreRpcServiceServer::new(service))
        .serve(SocketAddr::new(IpAddr::V4(Ipv4Addr::UNSPECIFIED), 13000))
        .await
        .expect("failed to start server");

    unreachable!("service exited unexpectedly");
}

impl Service {
    async fn new() -> Result<Self> {
        let db = create_pool_with(Config::default()).await?;

        Ok(Self { db })
    }
}
