// 模型提供商 CRUD API
use axum::extract::{Path, State};
use axum::Json;
use serde::Deserialize;

use crate::error::AppError;
use crate::repository::entity::ModelProviderEntity;
use crate::repository::model_provider_repository;
use crate::state::AppState;

// 查询全部模型提供商，按 id 升序
pub async fn list_model_providers(State(state): State<AppState>) -> Result<Json<Vec<ModelProviderEntity>>, AppError> {
    let providers = model_provider_repository::list_model_providers(&state.db).await?;
    Ok(Json(providers))
}

// 按 id 查询模型提供商，不存在返回 404
pub async fn get_model_provider(State(state): State<AppState>, Path(provider_id): Path<i64>) -> Result<Json<ModelProviderEntity>, AppError> {
    let provider = model_provider_repository::get_model_provider(&state.db, provider_id).await?;
    match provider {
        Some(p) => Ok(Json(p)),
        None => Err(AppError::NotFound("Model provider not found".to_string())),
    }
}

// 新增模型提供商请求体
#[derive(Debug, Deserialize)]
pub struct AddModelProviderRequest {
    pub name: String,
    pub protocol_type: String,
    pub base_url: String,
    pub api_key: String,
}

// 新增模型提供商，名称已存在返回 409
pub async fn add_model_provider(State(state): State<AppState>, Json(req): Json<AddModelProviderRequest>) -> Result<(), AppError> {
    let result = model_provider_repository::add_model_provider(&state.db, &req.name, &req.protocol_type, &req.base_url, &req.api_key).await?;
    if result.is_none() {
        return Err(AppError::Conflict("Model provider already exists".to_string()));
    }
    Ok(())
}

// 更新模型提供商请求体
#[derive(Debug, Deserialize)]
pub struct UpdateModelProviderRequest {
    pub name: String,
    pub protocol_type: String,
    pub base_url: String,
    pub api_key: String,
}

// 按 id 更新模型提供商，不存在或名称冲突返回 404
pub async fn update_model_provider(State(state): State<AppState>, Path(provider_id): Path<i64>, Json(req): Json<UpdateModelProviderRequest>) -> Result<(), AppError> {
    let updated = model_provider_repository::update_model_provider(&state.db, provider_id, &req.name, &req.protocol_type, &req.base_url, &req.api_key).await?;
    if !updated {
        return Err(AppError::NotFound("Model provider not found".to_string()));
    }
    Ok(())
}

// 按 id 删除模型提供商，不存在返回 404，被 Agent 引用返回 409
pub async fn delete_model_provider(State(state): State<AppState>, Path(provider_id): Path<i64>) -> Result<(), AppError> {
    let provider = model_provider_repository::get_model_provider(&state.db, provider_id).await?;
    if provider.is_none() {
        return Err(AppError::NotFound("Model provider not found".to_string()));
    }
    let deleted = model_provider_repository::delete_model_provider(&state.db, provider_id).await?;
    if !deleted {
        return Err(AppError::Conflict("Model provider is referenced by agents".to_string()));
    }
    Ok(())
}
