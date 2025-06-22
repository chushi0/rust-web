use anyhow::Result;
use common::tonic_idl_gen::{mc_service_client::McServiceClient, SyncMcVersionRequest};

pub async fn handle() -> Result<()> {
    let mut mc_rpc_client =
        McServiceClient::connect("http://mc-service-rpc.default.svc.cluster.local:13000").await?;
    mc_rpc_client
        .sync_mc_version(SyncMcVersionRequest {})
        .await?;

    Ok(())
}
