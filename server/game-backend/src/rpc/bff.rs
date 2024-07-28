use idl_gen::bff_websocket::*;
use lazy_static::lazy_static;
use std::net::SocketAddr;

lazy_static! {
    static ref CLIENT: BffWebsocketServiceClient = {
        let addr: SocketAddr = "rustweb.chushi0.web-bff:13500".parse().unwrap();
        BffWebsocketServiceClientBuilder::new("bff-websocket")
            .address(addr)
            .build()
    };
}

pub fn client() -> &'static BffWebsocketServiceClient {
    &CLIENT
}
