// Agent CRUD API
use axum::extract::{Path, State};
use axum::Json;
use serde::Deserialize;

use crate::error::AppError;
use crate::repository::agent_repository;
use crate::repository::entity::AgentEntity;
use crate::state::AppState;

// 查询全部 Agent, 按 id 升序
pub async fn list_agents(State(state): State<AppState>) -> Result<Json<Vec<AgentEntity>>, AppError> {
    let agents = agent_repository::list_agents(&state.db).await?;
    Ok(Json(agents))
}

// 按 id 查询 Agent, 不存在返回 404
pub async fn get_agent(State(state): State<AppState>, Path(agent_id): Path<i64>) -> Result<Json<AgentEntity>, AppError> {
    let agent = agent_repository::get_agent(&state.db, agent_id).await?;
    match agent {
        Some(a) => Ok(Json(a)),
        None => Err(AppError::NotFound("Agent not found".to_string())),
    }
}

// Agent 新增/更新请求体
#[derive(Debug, Deserialize)]
pub struct AgentRequest {
    pub name: String,
    pub description: String,
    pub prompt: String,
    pub model_provider_id: i64,
    pub model: String,
    pub thinking: bool,
}

// 新增 Agent, 返回自增 id
pub async fn add_agent(State(state): State<AppState>, Json(req): Json<AgentRequest>) -> Result<Json<i64>, AppError> {
    let id = agent_repository::add_agent(&state.db, &req.name, &req.description, &req.prompt, req.model_provider_id, &req.model, req.thinking).await?;
    Ok(Json(id))
}

// 按 id 更新 Agent, 不存在返回 404
pub async fn update_agent(State(state): State<AppState>, Path(agent_id): Path<i64>, Json(req): Json<AgentRequest>) -> Result<(), AppError> {
    let updated = agent_repository::update_agent(&state.db, agent_id, &req.name, &req.description, &req.prompt, req.model_provider_id, &req.model, req.thinking).await?;
    if !updated {
        return Err(AppError::NotFound("Agent not found".to_string()));
    }
    Ok(())
}

// 按 id 删除 Agent, 不存在返回 404, 被对话或定时任务引用返回 409
pub async fn delete_agent(State(state): State<AppState>, Path(agent_id): Path<i64>) -> Result<(), AppError> {
    let agent = agent_repository::get_agent(&state.db, agent_id).await?;
    if agent.is_none() {
        return Err(AppError::NotFound("Agent not found".to_string()));
    }
    let deleted = agent_repository::delete_agent(&state.db, agent_id).await?;
    if !deleted {
        return Err(AppError::Conflict("Agent is referenced by conversations or schedules".to_string()));
    }
    Ok(())
}
