use axum::{Json, http::StatusCode, response::IntoResponse};
use serde::Serialize;
use utoipa::ToSchema;

#[derive(Debug, thiserror::Error)]
pub enum AppError {
    #[error("database error: {0}")]
    Db(#[from] sea_orm::DbErr),
    #[error("http address error: {0}")]
    AddrParse(#[from] std::net::AddrParseError),
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
    #[error("config error: {0}")]
    Config(String),
    #[error("tracing setup error: {0}")]
    Tracing(#[from] tracing_subscriber::util::TryInitError),
    #[error("tracing filter error: {0}")]
    TracingFilter(#[from] tracing_subscriber::filter::ParseError),
    #[error("not found: {0}")]
    NotFound(&'static str),
    #[error("validation error: {0}")]
    Validation(String),
    #[error("unauthorized")]
    Unauthorized,
    #[error("forbidden")]
    Forbidden,
    #[error("rate limit exceeded")]
    RateLimited,
    #[error("rate limit backend error: {0}")]
    RateLimitBackend(String),
    #[error("plugin error: {0}")]
    Plugin(String),
    #[error("password hashing error: {0}")]
    PasswordHash(String),
    #[error("token error: {0}")]
    Token(#[from] jsonwebtoken::errors::Error),
}

#[derive(Clone, Debug, Serialize, ToSchema)]
pub struct ErrorBody {
    pub error: ErrorMessage,
}

#[derive(Clone, Debug, Serialize, ToSchema)]
pub struct ErrorMessage {
    pub code: &'static str,
    pub message: String,
}

impl IntoResponse for AppError {
    fn into_response(self) -> axum::response::Response {
        let (status, code) = match &self {
            Self::NotFound(_) => (StatusCode::NOT_FOUND, "not_found"),
            Self::Validation(_) => (StatusCode::UNPROCESSABLE_ENTITY, "validation_error"),
            Self::Unauthorized => (StatusCode::UNAUTHORIZED, "unauthorized"),
            Self::Forbidden => (StatusCode::FORBIDDEN, "forbidden"),
            Self::RateLimited => (StatusCode::TOO_MANY_REQUESTS, "rate_limited"),
            Self::RateLimitBackend(_) => (StatusCode::SERVICE_UNAVAILABLE, "rate_limit_backend"),
            Self::Plugin(_) => (StatusCode::INTERNAL_SERVER_ERROR, "plugin_error"),
            _ => (StatusCode::INTERNAL_SERVER_ERROR, "internal_error"),
        };

        let body = ErrorBody {
            error: ErrorMessage {
                code,
                message: self.to_string(),
            },
        };

        (status, Json(body)).into_response()
    }
}

pub type AppResult<T> = Result<T, AppError>;

pub fn validation_on_unique(err: sea_orm::DbErr, message: impl Into<String>) -> AppError {
    let err_text = err.to_string().to_lowercase();
    if err_text.contains("unique") || err_text.contains("duplicate") {
        AppError::Validation(message.into())
    } else {
        AppError::Db(err)
    }
}
