use std::sync::Arc;

use anyhow::Result;
use base64::Engine;
use chrono::{DateTime, Utc};
use hmac::{Hmac, Mac};
use jwt::{SignWithKey, VerifyWithKey};
use serde::{Deserialize, Serialize};
use sha2::Sha256;

#[derive(Clone)]
pub struct TokenKey(Arc<Hmac<Sha256>>);

pub fn init_token_key() -> TokenKey {
    let key = std::env::var("RUSTWEB_TOKEN_KEY").expect("RUSTWEB_TOKEN_KEY should be set by k8s");
    let key = base64::engine::general_purpose::STANDARD
        .decode(key)
        .expect("RUSTWEB_TOKEN_KEY should be a valid base64");
    TokenKey(Arc::new(Hmac::new_from_slice(&key).expect(
        "RUSTWEB_TOKEN_KEY should can be decode to a valid sha256 key",
    )))
}

#[derive(Serialize, Deserialize)]
pub struct UserToken {
    pub uid: u64,
    #[serde(with = "chrono::serde::ts_seconds")]
    pub sign: DateTime<Utc>,
    #[serde(with = "chrono::serde::ts_seconds")]
    pub exp: DateTime<Utc>,
}

#[inline]
pub fn sign_token(user_token: &UserToken, key: &TokenKey) -> Result<String> {
    user_token
        .sign_with_key(&*key.0)
        .map_err(anyhow::Error::from)
}

#[inline]
#[allow(unused)] // TODO: unused now
pub fn verify_token(token: &str, key: &TokenKey) -> Result<UserToken> {
    token.verify_with_key(&*key.0).map_err(anyhow::Error::from)
}
