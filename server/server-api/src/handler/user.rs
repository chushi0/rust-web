use axum::{Extension, Json};
use chrono::{TimeDelta, Utc};
use common::tonic_idl_gen::{CheckUserLoginBizError, CreateUserBizError};
use server_common::rpc_client::CoreRpcServiceClient;

use crate::{
    extract::{error::AppError, response::BodyResponse},
    model::{
        bizerror::BizError,
        user::{LoginRequest, LoginResponse, RegisterRequest, RegisterResponse},
    },
    service::{
        self,
        token::{TokenKey, UserToken},
    },
};

pub async fn login(
    Extension(mut core_rpc_client): Extension<CoreRpcServiceClient>,
    Extension(key): Extension<TokenKey>,
    Json(req): Json<LoginRequest>,
) -> Result<BodyResponse<LoginResponse>, AppError> {
    if req.username.len() > 20 {
        return Err(AppError::BadRequest("username is too long"));
    }
    if req
        .username
        .chars()
        .any(|c| !c.is_ascii() || c.is_ascii_control())
    {
        return Err(AppError::BadRequest(
            "invalid username, only accept ascii characters",
        ));
    }

    let login_response = core_rpc_client
        .check_user_login(common::tonic_idl_gen::CheckUserLoginRequest {
            username: req.username,
            password: req.password,
        })
        .await?
        .into_inner();

    match CheckUserLoginBizError::try_from(login_response.error) {
        Ok(CheckUserLoginBizError::LoginSuccess) => {
            let now = Utc::now();
            let token = UserToken {
                uid: login_response.id,
                sign: now,
                exp: now + TimeDelta::hours(12),
            };
            let token_str = service::token::sign_token(&token, &key)?;
            Ok(BodyResponse::new(LoginResponse {
                token: token_str,
                expire: token.exp,
                user_id: token.uid,
            }))
        }
        Ok(CheckUserLoginBizError::WrongUsername | CheckUserLoginBizError::WrongPassword) => {
            Err(AppError::BizError(BizError::InvalidUsernameOrPassword))
        }
        Err(_) => Err(AppError::BizError(BizError::InternalError)),
    }
}

pub async fn register(
    Extension(mut core_rpc_client): Extension<CoreRpcServiceClient>,
    Extension(key): Extension<TokenKey>,
    Json(req): Json<RegisterRequest>,
) -> Result<BodyResponse<RegisterResponse>, AppError> {
    if req.username.len() > 20 {
        return Err(AppError::BadRequest("username is too long"));
    }
    if req
        .username
        .chars()
        .any(|c| !c.is_ascii() || c.is_ascii_control())
    {
        return Err(AppError::BadRequest(
            "invalid username, only accept ascii characters",
        ));
    }

    let register_response = core_rpc_client
        .create_user(common::tonic_idl_gen::CreateUserRequest {
            username: req.username,
            password: req.password,
        })
        .await?
        .into_inner();

    match CreateUserBizError::try_from(register_response.error) {
        Ok(CreateUserBizError::CreateUserSuccess) => {
            let now = Utc::now();
            let token = UserToken {
                uid: register_response.id,
                sign: now,
                exp: now + TimeDelta::hours(12),
            };
            let token_str = service::token::sign_token(&token, &key)?;
            Ok(BodyResponse::new(RegisterResponse {
                token: token_str,
                expire: token.exp,
                user_id: token.uid,
            }))
        }
        Ok(CreateUserBizError::DuplicateUsername) => {
            Err(AppError::BizError(BizError::DuplicateUsername))
        }
        Err(_) => Err(AppError::BizError(BizError::InternalError)),
    }
}
