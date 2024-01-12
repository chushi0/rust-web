use idl_gen::game_backend::*;
use lazy_static::lazy_static;
use std::net::SocketAddr;

lazy_static! {
    static ref CLIENT: GameBackendServiceClient = {
        let addr: SocketAddr = "127.0.0.1:13201".parse().unwrap();
        GameBackendServiceClientBuilder::new("game-backend")
            .address(addr)
            .build()
    };
}

pub fn client() -> &'static GameBackendServiceClient {
    &CLIENT
}
