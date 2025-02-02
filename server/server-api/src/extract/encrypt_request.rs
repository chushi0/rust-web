use std::fmt::Debug;

use aes_gcm::{aead::Aead, Aes256Gcm, KeyInit};
use anyhow::anyhow;
use axum::{
    body::Bytes,
    extract::{FromRequest, FromRequestParts, Query},
    http::StatusCode,
};
use base64::Engine;
use chrono::{DateTime, Utc};
use hkdf::Hkdf;
use serde::{de::DeserializeOwned, Deserialize, Deserializer};
use sha2::{digest::generic_array::GenericArray, Sha256};

use super::error::AppError;

#[derive(Debug, Deserialize)]
struct EncryptPayload {
    #[serde(with = "chrono::serde::ts_seconds")]
    time: DateTime<Utc>,
    #[serde(deserialize_with = "deserialize_bytes")]
    salt: Vec<u8>,
    #[serde(deserialize_with = "deserialize_bytes")]
    nonce: Vec<u8>,
    #[serde(deserialize_with = "deserialize_bytes")]
    payload: Vec<u8>,
}

impl EncryptPayload {
    fn verify_and_deserialize<T>(&self) -> Result<T, AppError>
    where
        T: DeserializeOwned,
    {
        // 1. 检查时间有效性：与服务器时间相差30秒以内
        let now = Utc::now();
        if (now - self.time).abs().num_seconds() > 30 {
            return Err(AppError::HttpError(StatusCode::FORBIDDEN));
        }

        // 2. 计算密钥
        let Ok(auth_key) = std::env::var("RUSTWEB_API_AUTH_KEY") else {
            return Err(AppError::Error(anyhow!("auth_key is not set")));
        };
        let hk = Hkdf::<Sha256>::new(Some(&self.salt), auth_key.as_bytes());
        let mut key = [0u8; 32];
        hk.expand(&self.time.timestamp().to_le_bytes(), &mut key)
            .map_err(|e| anyhow!("failed to expand key: {e}"))?;

        // 3. 解密payload
        // TODO: 需要nonce缓存，防止重放攻击
        if self.nonce.len() != 12 {
            return Err(AppError::HttpError(StatusCode::FORBIDDEN));
        }
        let cipher = Aes256Gcm::new_from_slice(&key)?;
        let nonce = GenericArray::from_slice(&self.nonce);
        let plaintext = cipher
            .decrypt(nonce, &self.payload as &[u8])
            .map_err(|e| anyhow!("failed to decrypt payload: {e}"))?;

        serde_json::from_slice(&plaintext)
            .map_err(|_e| AppError::HttpError(StatusCode::BAD_REQUEST))
    }
}

#[derive(Debug)]
pub struct EncryptBodyRequest<T>(pub T)
where
    T: Debug + DeserializeOwned;

impl<T, S> FromRequest<S> for EncryptBodyRequest<T>
where
    T: Debug + DeserializeOwned,
    S: Send + Sync,
{
    type Rejection = AppError;

    async fn from_request(req: axum::extract::Request, state: &S) -> Result<Self, Self::Rejection> {
        // 使用msgpack解码请求体
        let Ok(request_bytes) = Bytes::from_request(req, state).await else {
            return Err(AppError::HttpError(StatusCode::BAD_REQUEST));
        };
        let Ok(encrypt_payload) = rmp_serde::from_slice::<EncryptPayload>(&request_bytes) else {
            return Err(AppError::HttpError(StatusCode::BAD_REQUEST));
        };

        encrypt_payload
            .verify_and_deserialize()
            .map(EncryptBodyRequest)
    }
}

#[derive(Debug)]
pub struct EncryptQueryRequest<T>(pub T)
where
    T: Debug + DeserializeOwned;

impl<T, S> FromRequestParts<S> for EncryptQueryRequest<T>
where
    T: Debug + DeserializeOwned,
    S: Send + Sync,
{
    type Rejection = AppError;

    async fn from_request_parts(
        parts: &mut axum::http::request::Parts,
        state: &S,
    ) -> Result<Self, Self::Rejection> {
        let Ok(Query(encrypt_payload)) =
            Query::<EncryptPayload>::from_request_parts(parts, state).await
        else {
            return Err(AppError::HttpError(StatusCode::BAD_REQUEST));
        };

        encrypt_payload
            .verify_and_deserialize()
            .map(EncryptQueryRequest)
    }
}

fn deserialize_bytes<'de, D>(deserializer: D) -> Result<Vec<u8>, D::Error>
where
    D: Deserializer<'de>,
{
    if deserializer.is_human_readable() {
        let value = String::deserialize(deserializer)?;

        base64::engine::general_purpose::URL_SAFE_NO_PAD
            .decode(value)
            .map_err(|e| serde::de::Error::custom(e))
    } else {
        <Vec<u8>>::deserialize(deserializer)
    }
}
