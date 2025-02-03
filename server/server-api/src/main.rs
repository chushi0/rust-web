use axum::{
    routing::{get, post},
    Extension, Router,
};
use server_common::{
    external_api::aliyun::oss::OssClient,
    rpc_client::{init_core_rpc_service_client, init_mc_service_client},
};

pub mod extract;
pub mod handler;
pub mod model;

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt::init();

    let core_rpc_service_client = init_core_rpc_service_client();
    let mc_service_client = init_mc_service_client();
    let oss_client = OssClient::from_env().expect("failed to initialize oss_client");

    let app = Router::new()
        .route("/api/home/events", get(handler::home::events))
        .route("/api/mc/version/list", get(handler::mc::list_mc_version))
        .route(
            "/api/mc/server_config/create",
            post(handler::mc::create_server_config),
        )
        .route(
            "/api/mc/server_config/list",
            get(handler::mc::list_server_config),
        )
        .route(
            "/api/mc/server_config/process/start",
            post(handler::mc::start_server_config),
        )
        .route(
            "/api/mc/server_config/process/stop",
            post(handler::mc::stop_server_config),
        )
        .route(
            "/api/mc/server_config/process/info",
            get(handler::mc::get_current_server_config),
        )
        .route("/api/mc/resource-pack", get(handler::mc::get_resource_pack))
        .route("/api/oss/upload", get(handler::oss::get_upload_signature))
        .layer(Extension(core_rpc_service_client))
        .layer(Extension(mc_service_client))
        .layer(Extension(oss_client));
    let listener = tokio::net::TcpListener::bind("0.0.0.0:8080").await.unwrap();
    axum::serve(listener, app).await.unwrap();

    unreachable!("service exited unexpectedly");
}
