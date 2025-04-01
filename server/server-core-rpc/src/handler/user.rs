use common::tonic_idl_gen::{
    CheckUserLoginBizError, CheckUserLoginRequest, CheckUserLoginResponse, CreateUserBizError,
    CreateUserRequest, CreateUserResponse,
};
use server_common::db::context::Context;
use tonic::{Request, Response, Status};

use crate::{
    service::user::{CreateUserError, UserLoginError},
    Service,
};

pub async fn create_user(
    service: &Service,
    request: Request<CreateUserRequest>,
) -> Result<Response<CreateUserResponse>, Status> {
    let req = request.into_inner();
    if req.username.len() > 30 {
        return Err(Status::invalid_argument("username is too long"));
    }

    let user = crate::service::user::create_user(
        &mut Context::PoolRef(&service.db),
        req.username,
        req.password,
    )
    .await;

    match user {
        Ok(user) => Ok(Response::new(CreateUserResponse {
            error: CreateUserBizError::CreateUserSuccess.into(),
            id: user.id,
        })),
        Err(CreateUserError::DuplicateUsername) => Ok(Response::new(CreateUserResponse {
            error: CreateUserBizError::DuplicateUsername.into(),
            ..Default::default()
        })),
        Err(CreateUserError::InternalError(error)) => Err(Status::internal(error.to_string())),
    }
}

pub async fn check_user_login(
    service: &Service,
    request: Request<CheckUserLoginRequest>,
) -> Result<Response<CheckUserLoginResponse>, Status> {
    let req = request.into_inner();
    if req.username.len() > 30 {
        return Err(Status::invalid_argument("username is too long"));
    }

    let user = crate::service::user::user_login(
        &mut Context::PoolRef(&service.db),
        &req.username,
        &req.password,
    )
    .await;

    match user {
        Ok(user) => Ok(Response::new(CheckUserLoginResponse {
            error: CheckUserLoginBizError::LoginSuccess.into(),
            id: user.id,
        })),
        Err(UserLoginError::UserNotFound) => Ok(Response::new(CheckUserLoginResponse {
            error: CheckUserLoginBizError::WrongUsername.into(),
            ..Default::default()
        })),
        Err(UserLoginError::PasswordIncorrect) => Ok(Response::new(CheckUserLoginResponse {
            error: CheckUserLoginBizError::WrongPassword.into(),
            ..Default::default()
        })),
        Err(UserLoginError::InternalError(error)) => Err(Status::internal(error.to_string())),
    }
}
