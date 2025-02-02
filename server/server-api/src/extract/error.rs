use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
};

pub enum AppError {
    BadRequest(&'static str),
    HttpError(StatusCode),
    Error(anyhow::Error),
}

impl<E> From<E> for AppError
where
    anyhow::Error: From<E>,
{
    fn from(value: E) -> Self {
        Self::Error(value.into())
    }
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        match self {
            AppError::BadRequest(reason) => (StatusCode::BAD_REQUEST, reason).into_response(),
            AppError::HttpError(status_code) => status_code.into_response(),
            AppError::Error(error) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Internal Server Error: {:?}", error),
            )
                .into_response(),
        }
    }
}
