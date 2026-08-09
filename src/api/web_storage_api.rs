// Web 存储 API
use axum::extract::{Path, State};
use axum::Json;
use serde::Deserialize;
use serde_json::{json, Value};

use crate::error::AppError;
use crate::repository::web_storage_repository;
use crate::state::AppState;

// 按 key 查询 Web 存储，不存在返回 value 为 null
pub async fn get_web_storage(State(state): State<AppState>, Path(key): Path<String>) -> Result<Json<Value>, AppError> {
    let storage = web_storage_repository::get_web_storage(&state.db, &key).await?;
    Ok(Json(json!({"value": storage.map(|s| s.value)})))
}

// Web 存储写入请求体
#[derive(Debug, Deserialize)]
pub struct WebStorageRequest {
    pub value: String,
}

// 按 key 写入 Web 存储，不存在则新增，存在则更新
pub async fn put_web_storage(State(state): State<AppState>, Path(key): Path<String>, Json(req): Json<WebStorageRequest>) -> Result<(), AppError> {
    web_storage_repository::put_web_storage(&state.db, &key, &req.value).await?;
    Ok(())
}
