#![feature(type_alias_impl_trait)]

use std::net::SocketAddr;

use volo_grpc::server::{Server, ServiceBuilder};

use game_backend::S;

#[volo::main]
async fn main() {
    log4rs::init_file("log4rs.yaml", Default::default()).unwrap();

    let addr: SocketAddr = "127.0.0.1:13201".parse().unwrap();
    let addr = volo::net::Address::from(addr);

    Server::new()
        .add_service(
            ServiceBuilder::new(idl_gen::game_backend::GameBackendServiceServer::new(S)).build(),
        )
        .run(addr)
        .await
        .unwrap();
}
