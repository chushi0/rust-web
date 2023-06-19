use idl_gen::bss_websocket::*;
use lazy_static::lazy_static;
use std::net::SocketAddr;

lazy_static! {
    static ref CLIENT: BssWebsocketServiceClient = {
        let addr: SocketAddr = "127.0.0.1:13202".parse().unwrap();
        BssWebsocketServiceClientBuilder::new("bss-websocket")
            .address(addr)
            .build()
    };
}

pub fn client() -> &'static BssWebsocketServiceClient {
    &CLIENT
}
