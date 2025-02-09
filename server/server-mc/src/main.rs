use std::{
    net::{IpAddr, Ipv4Addr, SocketAddr},
    time::Duration,
};

use anyhow::Result;
use common::tonic_idl_gen::mc_service_server::McServiceServer;
use process::manager::Manager;
use reqwest::Client;
use server_common::{
    db::pool::{create_pool_with, Config},
    external_api::aliyun::oss::OssClient,
};
use service::process::ProcessService;
use sqlx::{MySql, Pool};
use tonic::transport::Server;
use tracing::info;

pub mod dao;
pub mod handler;
pub mod process;
pub mod service;

pub struct Service {
    pub db: Pool<MySql>,
    pub oss_client: OssClient,
    pub http_client: Client,
    pub process_manager: Manager,
}

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt::init();

    let service = Service::new().await.expect("initialize service failed");

    info!("starting service...");
    Server::builder()
        .add_service(McServiceServer::new(service))
        .serve(SocketAddr::new(IpAddr::V4(Ipv4Addr::UNSPECIFIED), 13000))
        .await
        .expect("failed to start server");

    unreachable!("service exited unexpectedly");
}

impl Service {
    async fn new() -> Result<Self> {
        let db = create_pool_with(Config::default()).await?;
        let oss_client = OssClient::from_env()?;
        let http_client = Client::builder()
            .connect_timeout(Duration::from_secs(5))
            .read_timeout(Duration::from_secs(5))
            .build()?;
        let process_service =
            ProcessService::new(db.clone(), oss_client.clone(), http_client.clone());
        let process_manager = Manager::new(process_service);

        Ok(Self {
            db,
            oss_client,
            http_client,
            process_manager,
        })
    }
}
