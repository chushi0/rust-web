use common::tonic_idl_gen::{core_rpc_service_server::CoreRpcService, *};

use tonic::{Request, Response, Status};

use crate::Service;

pub mod event;

#[tonic::async_trait]
impl CoreRpcService for Service {
    async fn list_display_event(
        &self,
        request: Request<ListDisplayEventRequest>,
    ) -> Result<Response<ListDisplayEventResponse>, Status> {
        event::list_display_event(self, request).await
    }

    async fn list_github_activity_event(
        &self,
        request: Request<ListGithubActivityEventRequest>,
    ) -> Result<Response<ListGithubActivityEventResponse>, Status> {
        event::list_github_activity_event(&self, request).await
    }

    async fn create_github_activity_event(
        &self,
        request: Request<CreateGithubActivityEventRequest>,
    ) -> Result<Response<CreateGithubActivityEventResponse>, Status> {
        event::create_github_activity_event(&self, request).await
    }
}
