use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize)]
pub struct LoginRequest {
    pub username: String,
    pub password: String,
}

#[derive(Debug, Serialize)]
pub struct LoginResponse {
    pub token: String,
    #[serde(with = "chrono::serde::ts_seconds")]
    pub expire: DateTime<Utc>,
    pub user_id: u64,
}

#[derive(Debug, Deserialize)]
pub struct RegisterRequest {
    pub username: String,
    pub password: String,
}

#[derive(Debug, Serialize)]
pub struct RegisterResponse {
    pub token: String,
    #[serde(with = "chrono::serde::ts_seconds")]
    pub expire: DateTime<Utc>,
    pub user_id: u64,
}
