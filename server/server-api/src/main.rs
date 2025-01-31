use axum::{routing::get, Router};

pub mod handler;
pub mod model;

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt::init();

    let app = Router::new().route("/api/home/events", get(handler::home::events));
    let listener = tokio::net::TcpListener::bind("0.0.0.0:8080").await.unwrap();
    axum::serve(listener, app).await.unwrap();

    unreachable!("service exited unexpectedly");
}
