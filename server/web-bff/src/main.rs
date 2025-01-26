#[macro_use]
extern crate rocket;

pub mod biz;
mod boot;
#[allow(unused)]
mod handler_api;
#[allow(unused)]
mod handler_grpc;
#[allow(unused)]
mod handler_ws;
pub mod model;
pub mod rpc;
pub mod service;
pub mod util;
pub mod ws;

#[tokio::main]
async fn main() {
    log4rs::init_file("conf/log4rs.yaml", Default::default()).unwrap();

    let grpc = boot::init_grpc();
    let ws = boot::init_websocket();
    let api = boot::init_rocket();

    tokio::select! {
        _ = grpc => {
            info!("grpc stream stop");
            std::process::exit(0);
        }
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
