use common::tonic_idl_gen::{mc_service_server::McService, *};

use tonic::{Request, Response, Status};

use crate::Service;

pub mod process;
pub mod server_config;
pub mod version;

#[tonic::async_trait]
impl McService for Service {
    async fn list_mc_version(
        &self,
        req: Request<ListMcVersionRequest>,
    ) -> Result<Response<ListMcVersionResponse>, Status> {
        version::list_mc_version(self, req.into_inner()).await
    }

    async fn sync_mc_version(
        &self,
        req: Request<SyncMcVersionRequest>,
    ) -> Result<Response<SyncMcVersionResponse>, Status> {
        version::sync_mc_version(self, req.into_inner()).await
    }

    async fn create_server_config(
        &self,
        req: Request<CreateServerConfigRequest>,
    ) -> Result<Response<CreateServerConfigResponse>, Status> {
        server_config::create_server_config(self, req.into_inner()).await
    }

    async fn list_server_config(
        &self,
        req: Request<ListServerConfigRequest>,
    ) -> Result<Response<ListServerConfigResponse>, Status> {
        server_config::list_server_config(self, req.into_inner()).await
    }

    async fn delete_server_config(
        &self,
        req: Request<DeleteServerConfigRequest>,
    ) -> Result<Response<DeleteServerConfigResponse>, Status> {
        server_config::delete_server_config(self, req.into_inner()).await
    }

    async fn start_server_config(
        &self,
        req: Request<StartServerConfigRequest>,
    ) -> Result<Response<StartServerConfigResponse>, Status> {
        process::start_server_config(self, req.into_inner()).await
    }

    async fn stop_server_config(
        &self,
        req: Request<StopServerConfigRequest>,
    ) -> Result<Response<StopServerConfigResponse>, Status> {
        process::stop_server_config(self, req.into_inner()).await
    }

    async fn get_current_server_config(
        &self,
        req: Request<GetCurrentServerConfigRequest>,
    ) -> Result<Response<GetCurrentServerConfigResponse>, Status> {
        process::get_current_server_config(self, req.into_inner()).await
    }
}
