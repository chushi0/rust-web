use anyhow::Result;
use common::tonic_idl_gen::SyncMcVersionRequest;
use server_common::rpc_client::init_mc_service_client;

pub async fn handle() -> Result<()> {
    let mut mc_rpc_client = init_mc_service_client();
    mc_rpc_client
        .sync_mc_version(SyncMcVersionRequest {})
        .await?;

    Ok(())
}
