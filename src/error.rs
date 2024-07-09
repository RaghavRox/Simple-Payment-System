use axum::{http::StatusCode, response::IntoResponse};
use thiserror::Error;

pub(crate) type AppResult<T> = Result<T, AppError>;

#[derive(Error, Debug)]
pub(crate) enum AppError {
    #[error("Unauthorized")]
    Unauthorized,
    #[error("{0}")]
    SqlxError(#[from] sqlx::Error),
    #[error("{0}")]
    AnyhowError(#[from] anyhow::Error),
    #[error("{0}")]
    JwtError(#[from] jsonwebtoken::errors::Error),
}

impl IntoResponse for AppError {
    fn into_response(self) -> axum::response::Response {
        tracing::error!("{}", self);

        match self {
            AppError::Unauthorized => StatusCode::UNAUTHORIZED.into_response(),
            AppError::SqlxError(err) => {
                (StatusCode::INTERNAL_SERVER_ERROR, err.to_string()).into_response()
            }
            AppError::AnyhowError(err) => {
                (StatusCode::INTERNAL_SERVER_ERROR, err.to_string()).into_response()
            }
            AppError::JwtError(err) => {
                (StatusCode::INTERNAL_SERVER_ERROR, err.to_string()).into_response()
            }
        }
    }
}
