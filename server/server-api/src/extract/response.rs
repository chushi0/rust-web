use std::fmt::Debug;

use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
};
use serde::Serialize;
use tracing::info;

#[derive(Debug, Serialize)]
pub struct BodyResponse<T>
where
    T: Debug + Serialize,
{
    pub code: i32,
    #[serde(flatten)]
    pub body: T,
}

impl<T> BodyResponse<T>
where
    T: Debug + Serialize,
{
    pub fn new(body: T) -> Self {
        Self { code: 0, body }
    }
}

impl<T> IntoResponse for BodyResponse<T>
where
    T: Debug + Serialize,
{
    fn into_response(self) -> Response {
        info!("Response: {:?}", self);

        let body = serde_json::to_string(&self).unwrap();
        let mut response = Response::new(body.into());
        *response.status_mut() = StatusCode::OK;
        response.headers_mut().insert(
            "content-type",
            "application/json; charset=utf-8".parse().unwrap(),
        );
        response
    }
}
