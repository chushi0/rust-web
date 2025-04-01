use server_common::db::context::{Context, ContextRef};
use sqlx::Database;
use thiserror::Error;

use crate::dao::user::{User, UserRepository};

const PASSWORD_COST: u32 = 10;

#[derive(Debug, Error)]
pub enum CreateUserError {
    #[error("duplicate username")]
    DuplicateUsername,
    #[error("{0}")]
    InternalError(#[from] anyhow::Error),
}

pub async fn create_user<DB>(
    db: ContextRef<'_, '_, DB>,
    username: String,
    password: String,
) -> Result<User, CreateUserError>
where
    DB: Database,
    for<'db> Context<'db, DB>: UserRepository,
{
    if db.query_user_by_username(&username).await?.is_some() {
        return Err(CreateUserError::DuplicateUsername);
    }

    let password = bcrypt::hash(password, PASSWORD_COST).map_err(anyhow::Error::from)?;

    let mut user = User {
        username,
        password,
        ..Default::default()
    };

    db.create_user(&mut user).await?;

    Ok(user)
}

#[derive(Debug, Error)]
pub enum UserLoginError {
    #[error("user not found")]
    UserNotFound,
    #[error("password incorrect")]
    PasswordIncorrect,
    #[error("{0}")]
    InternalError(#[from] anyhow::Error),
}

pub async fn user_login<DB>(
    db: ContextRef<'_, '_, DB>,
    username: &str,
    password: &str,
) -> Result<User, UserLoginError>
where
    DB: Database,
    for<'db> Context<'db, DB>: UserRepository,
{
    let Some(user) = db.query_user_by_username(&username).await? else {
        return Err(UserLoginError::UserNotFound);
    };

    if !bcrypt::verify(password, &user.password).map_err(anyhow::Error::from)? {
        return Err(UserLoginError::PasswordIncorrect);
    }

    Ok(user)
}
