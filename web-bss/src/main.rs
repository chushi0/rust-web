#[macro_use]
extern crate rocket;

pub mod biz;
mod boot;
mod handler_api;
mod handler_ws;
pub mod model;
pub mod rpc;
pub mod service;
pub mod util;
pub mod ws;

#[tokio::main]
async fn main() {
    let ws = boot::init_websocket();
    let api = boot::init_rocket();

    tokio::select! {
        _ = ws => {
            info!("websocket stream stop");
            std::process::exit(0);
        },
        _ = api => {
            info!("api stream stop");
            std::process::exit(0);
        },
    }
}
