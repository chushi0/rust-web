use anyhow::anyhow;
use anyhow::Result;
use idl_gen::core_rpc::*;
use lazy_static::lazy_static;
use std::net::SocketAddr;
use std::net::ToSocketAddrs;

lazy_static! {
    static ref CLIENT: CoreRpcServiceClient = {
        let addr: SocketAddr = "core-rpc.default.svc.cluster.local:13000".parse().unwrap();
        CoreRpcServiceClientBuilder::new("core-rpc")
            .address(addr)
            .build()
    };
}

pub fn client() -> Result<CoreRpcServiceClient> {
    let addr = "core-rpc-service.default.svc.cluster.local:13000"
        .to_socket_addrs()?
        .next()
        .ok_or(anyhow!("dns lookup failed"))?;

    Ok(CoreRpcServiceClientBuilder::new("core-rpc")
        .address(addr)
        .build())
}
