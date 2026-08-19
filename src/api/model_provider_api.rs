// 模型提供商 CRUD API
use axum::extract::{Path, State};
use axum::Json;
use serde::Deserialize;

use crate::ai;
use crate::error::AppError;
use crate::repository::entity::ModelProviderEntity;
use crate::repository::{model_provider_repository, DeleteResult};
use crate::state::AppState;

// 查询全部模型提供商, 按 id 升序
pub async fn list_model_providers(
    State(state): State<AppState>,
) -> Result<Json<Vec<ModelProviderEntity>>, AppError> {
    let providers = model_provider_repository::list_model_providers(&state.db).await?;
    Ok(Json(providers))
}

// 按 id 查询模型提供商, 不存在返回 404
pub async fn get_model_provider(
    State(state): State<AppState>,
    Path(provider_id): Path<i64>,
) -> Result<Json<ModelProviderEntity>, AppError> {
    let provider = model_provider_repository::get_model_provider(&state.db, provider_id)
        .await?
        .ok_or_else(|| AppError::NotFound("Model provider not found".to_string()))?;
    Ok(Json(provider))
}

// 模型提供商新增/更新请求体
#[derive(Debug, Deserialize)]
pub struct ModelProviderRequest {
    pub name: String,
    pub protocol_type: String,
    pub base_url: String,
    pub api_key: String,
}

// 新增模型提供商, 返回自增 id
pub async fn add_model_provider(
    State(state): State<AppState>,
    Json(req): Json<ModelProviderRequest>,
) -> Result<Json<i64>, AppError> {
    let id = model_provider_repository::add_model_provider(
        &state.db,
        &req.name,
        &req.protocol_type,
        &req.base_url,
        &req.api_key,
    )
    .await?;
    Ok(Json(id))
}

// 按 id 更新模型提供商, 不存在返回 404
pub async fn update_model_provider(
    State(state): State<AppState>,
    Path(provider_id): Path<i64>,
    Json(req): Json<ModelProviderRequest>,
) -> Result<(), AppError> {
    let updated = model_provider_repository::update_model_provider(
        &state.db,
        provider_id,
        &req.name,
        &req.protocol_type,
        &req.base_url,
        &req.api_key,
    )
    .await?;
    if !updated {
        return Err(AppError::NotFound("Model provider not found".to_string()));
    }
    Ok(())
}

// 按 id 删除模型提供商, 不存在返回 404, 被 Agent 引用返回 409
pub async fn delete_model_provider(
    State(state): State<AppState>,
    Path(provider_id): Path<i64>,
) -> Result<(), AppError> {
    match model_provider_repository::delete_model_provider(&state.db, provider_id).await? {
        DeleteResult::Deleted => Ok(()),
        DeleteResult::NotFound => Err(AppError::NotFound("Model provider not found".to_string())),
        DeleteResult::Referenced => Err(AppError::Conflict(
            "Model provider is referenced by agents".to_string(),
        )),
    }
}

// 查询模型提供商的可用模型列表, 按 provider 协议类型路由实时获取
pub async fn list_provider_models(
    State(state): State<AppState>,
    Path(provider_id): Path<i64>,
) -> Result<Json<Vec<String>>, AppError> {
    let provider = model_provider_repository::get_model_provider(&state.db, provider_id)
        .await?
        .ok_or_else(|| AppError::NotFound("Model provider not found".to_string()))?;
    let models = ai::client::list_models(&provider)
        .await
        .map_err(|e| AppError::Internal(e.into()))?;
    Ok(Json(models))
}
