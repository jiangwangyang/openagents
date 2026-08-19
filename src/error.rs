// 统一错误类型与 IntoResponse 实现
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};

#[derive(Debug, thiserror::Error)]
pub enum AppError {
    #[error("{0}")]
    NotFound(String),

    #[error("{0}")]
    Conflict(String),

    #[error("{0}")]
    BadRequest(String),

    #[error("数据库错误: {0}")]
    Db(#[from] sqlx::Error),

    #[error("JSON 序列化失败: {0}")]
    Json(#[from] serde_json::Error),

    #[error("内部错误: {0}")]
    Internal(#[from] anyhow::Error),
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let (status, message) = match &self {
            AppError::NotFound(msg) => (StatusCode::NOT_FOUND, msg.clone()),
            AppError::Conflict(msg) => (StatusCode::CONFLICT, msg.clone()),
            AppError::BadRequest(msg) => (StatusCode::BAD_REQUEST, msg.clone()),
            AppError::Db(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()),
            AppError::Json(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()),
            AppError::Internal(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()),
        };
        // 5xx 为服务端故障记 ERROR, 4xx 为客户端问题记 WARN
        if status.is_server_error() {
            tracing::error!(
                "Request failed: status={} error={}",
                status.as_u16(),
                message
            );
        } else {
            tracing::warn!(
                "Request rejected: status={} error={}",
                status.as_u16(),
                message
            );
        }
        (status, message).into_response()
    }
}
