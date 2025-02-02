use tonic::transport::{Channel, Endpoint};

const HOST_CORE_RPC: &str = "http://core-rpc-service.default.svc.cluster.local:13000";
const HOST_MC: &str = "http://mc-service-rpc.default.svc.cluster.local:13000";

pub type CoreRpcServiceClient =
    common::tonic_idl_gen::core_rpc_service_client::CoreRpcServiceClient<Channel>;
pub type McServiceClient = common::tonic_idl_gen::mc_service_client::McServiceClient<Channel>;

pub fn init_core_rpc_service_client() -> CoreRpcServiceClient {
    CoreRpcServiceClient::new(Endpoint::from_static(HOST_CORE_RPC).connect_lazy())
}

pub fn init_mc_service_client() -> McServiceClient {
    McServiceClient::new(Endpoint::from_static(&HOST_MC).connect_lazy())
}
