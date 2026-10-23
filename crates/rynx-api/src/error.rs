use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::Json;
use serde_json::json;

use rynx_panel::PanelError;
use rynx_proto::{ErrorCode, WireError};

#[derive(Debug)]
pub enum ApiError {
    Unauthorized,
    Forbidden,
    BadRequest(String),
    NotFound(String),
    Conflict(String),
    RateLimited,
    Upstream(String),
    Internal(String),
}

impl ApiError {
    pub fn internal(err: impl std::fmt::Display) -> Self {
        tracing::error!(error = %err, "internal API error");
        ApiError::Internal(err.to_string())
    }
}

impl From<PanelError> for ApiError {
    fn from(value: PanelError) -> Self {
        match value {
            PanelError::InvalidCredentials => ApiError::Unauthorized,
            PanelError::Validation(msg) => ApiError::BadRequest(msg),
            PanelError::Conflict(msg) => ApiError::Conflict(msg),
            PanelError::NotFound(msg) => ApiError::NotFound(msg),
            PanelError::Db(msg) => ApiError::internal(msg),
            PanelError::Crypto(msg) => ApiError::internal(msg),
        }
    }
}

impl From<WireError> for ApiError {
    fn from(value: WireError) -> Self {
        match value.code {
            ErrorCode::Unauthorized | ErrorCode::Protocol => ApiError::Upstream(value.message),
            ErrorCode::InvalidInput => ApiError::BadRequest(value.message),
            ErrorCode::NotFound => ApiError::NotFound(value.message),
            ErrorCode::Conflict => ApiError::Conflict(value.message),
            ErrorCode::Unsupported => ApiError::BadRequest(value.message),
            ErrorCode::Io | ErrorCode::Internal => ApiError::internal(value.message),
        }
    }
}

impl From<rynx_proto::ProtocolError> for ApiError {
    fn from(value: rynx_proto::ProtocolError) -> Self {
        ApiError::internal(value)
    }
}

impl From<std::io::Error> for ApiError {
    fn from(value: std::io::Error) -> Self {
        ApiError::internal(value)
    }
}

impl std::fmt::Display for ApiError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let message = match self {
            ApiError::Unauthorized => "authentication required",
            ApiError::Forbidden => "insufficient permissions",
            ApiError::RateLimited => "too many attempts",
            ApiError::BadRequest(msg)
            | ApiError::NotFound(msg)
            | ApiError::Conflict(msg)
            | ApiError::Upstream(msg)
            | ApiError::Internal(msg) => msg,
        };
        write!(f, "{message}")
    }
}

impl std::error::Error for ApiError {}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let (status, code, message) = match self {
            ApiError::Unauthorized => (
                StatusCode::UNAUTHORIZED,
                "unauthorized",
                "authentication required".to_string(),
            ),
            ApiError::Forbidden => (
                StatusCode::FORBIDDEN,
                "forbidden",
                "insufficient permissions".to_string(),
            ),
            ApiError::BadRequest(msg) => (StatusCode::BAD_REQUEST, "invalid_request", msg),
            ApiError::NotFound(msg) => (StatusCode::NOT_FOUND, "not_found", msg),
            ApiError::Conflict(msg) => (StatusCode::CONFLICT, "conflict", msg),
            ApiError::RateLimited => (
                StatusCode::TOO_MANY_REQUESTS,
                "rate_limited",
                "too many attempts, try again later".to_string(),
            ),
            ApiError::Upstream(msg) => (StatusCode::BAD_GATEWAY, "daemon_unavailable", msg),
            ApiError::Internal(msg) => (StatusCode::INTERNAL_SERVER_ERROR, "internal_error", msg),
        };
        (
            status,
            Json(json!({ "error": { "code": code, "message": message } })),
        )
            .into_response()
    }
}
