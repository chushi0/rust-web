use axum::{
    routing::{get, post},
    Extension, Router,
};
use server_common::rpc_client::{init_core_rpc_service_client, init_mc_service_client};

pub mod extract;
pub mod handler;
pub mod model;

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt::init();

    let core_rpc_service_client = init_core_rpc_service_client();
    let mc_service_client = init_mc_service_client();

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
        .layer(Extension(core_rpc_service_client))
        .layer(Extension(mc_service_client));
    let listener = tokio::net::TcpListener::bind("0.0.0.0:8080").await.unwrap();
    axum::serve(listener, app).await.unwrap();

    unreachable!("service exited unexpectedly");
}
