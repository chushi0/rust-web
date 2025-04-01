use common::tonic_idl_gen::{core_rpc_service_server::CoreRpcService, *};

use tonic::{Request, Response, Status};

use crate::Service;

mod event;
mod user;

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
        event::list_github_activity_event(self, request).await
    }

    async fn create_github_activity_event(
        &self,
        request: Request<CreateGithubActivityEventRequest>,
    ) -> Result<Response<CreateGithubActivityEventResponse>, Status> {
        event::create_github_activity_event(self, request).await
    }

    async fn create_user(
        &self,
        request: Request<CreateUserRequest>,
    ) -> Result<Response<CreateUserResponse>, Status> {
        user::create_user(self, request).await
    }

    async fn check_user_login(
        &self,
        request: Request<CheckUserLoginRequest>,
    ) -> Result<Response<CheckUserLoginResponse>, Status> {
        user::check_user_login(self, request).await
    }
}
