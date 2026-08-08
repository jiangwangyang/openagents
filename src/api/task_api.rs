// 任务 CRUD API
use axum::extract::{Path, State};
use axum::Json;
use serde::Deserialize;
use serde_json::json;

use crate::error::AppError;
use crate::repository::{agent_repository, task_repository};
use crate::repository::entity::TaskEntity;
use crate::service::task_service;
use crate::state::AppState;

// 任务列表接口，按 id 升序返回全部任务（仅基本字段，不含阶段对话）
pub async fn list_tasks(State(state): State<AppState>) -> Result<Json<Vec<TaskEntity>>, AppError> {
    let tasks = task_repository::list_tasks(&state.db).await?;
    Ok(Json(tasks))
}

// 任务详情接口，包含各阶段对话（对话按 id 升序，每条对话含全部按 id 升序的消息），任务不存在返回 404
pub async fn get_task(State(state): State<AppState>, Path(task_id): Path<i64>) -> Result<Json<serde_json::Value>, AppError> {
    let task = task_repository::get_task(&state.db, task_id).await?;
    let task = match task {
        Some(t) => t,
        None => return Err(AppError::NotFound("Task not found".to_string())),
    };

    let conversations: Vec<serde_json::Value> = task.conversations.iter().map(|conv| {
        let messages: Vec<serde_json::Value> = conv.messages.iter().map(|msg| {
            json!({
                "id": msg.id,
                "role": msg.role,
                "content": msg.content,
                "time": msg.time,
            })
        }).collect();
        json!({
            "id": conv.id,
            "task_id": conv.task_id,
            "agent_id": conv.agent_id,
            "title": conv.title,
            "work_dir": conv.work_dir,
            "create_time": conv.create_time,
            "update_time": conv.update_time,
            "messages": messages,
        })
    }).collect();

    Ok(Json(json!({
        "id": task.id,
        "title": task.title,
        "content": task.content,
        "agent_ids": task.agent_ids,
        "work_dir": task.work_dir,
        "create_time": task.create_time,
        "update_time": task.update_time,
        "conversations": conversations,
    })))
}

// 新增任务请求体
#[derive(Debug, Deserialize)]
pub struct AddTaskRequest {
    pub title: String,
    pub content: String,
    pub agent_ids: Vec<i64>,
    pub work_dir: String,
}

// 新增任务接口，agent_ids 为可供 Agent 选择下一个执行者的候选池，work_dir 为任务阶段对话的工作目录，返回自增 id
pub async fn add_task(State(state): State<AppState>, Json(req): Json<AddTaskRequest>) -> Result<Json<i64>, AppError> {
    let agent_ids_json = serde_json::to_value(&req.agent_ids).map_err(|e| AppError::Internal(e.into()))?;
    let id = task_repository::add_task(&state.db, &req.title, &req.content, &agent_ids_json, &req.work_dir).await?;
    Ok(Json(id))
}

// 删除任务接口，关联对话由数据库外键 ON DELETE CASCADE 级联删除，任务不存在返回 404
pub async fn delete_task(State(state): State<AppState>, Path(task_id): Path<i64>) -> Result<(), AppError> {
    let deleted = task_repository::delete_task(&state.db, task_id).await?;
    if !deleted {
        return Err(AppError::NotFound("Task not found".to_string()));
    }
    Ok(())
}

// 启动任务请求体
#[derive(Debug, Deserialize)]
pub struct StartTaskRequest {
    pub agent_id: i64,
}

// 启动任务执行循环接口，agent_id 为首个执行的 Agent，阶段对话的工作目录取任务的 work_dir
// 任务/agent 不存在返回 404，执行循环已在运行返回 409
pub async fn start_task(State(state): State<AppState>, Path(task_id): Path<i64>, Json(req): Json<StartTaskRequest>) -> Result<(), AppError> {
    let task = task_repository::get_task(&state.db, task_id).await?;
    if task.is_none() {
        return Err(AppError::NotFound("Task not found".to_string()));
    }
    let agent = agent_repository::get_agent(&state.db, req.agent_id).await?;
    if agent.is_none() {
        return Err(AppError::NotFound("Agent not found".to_string()));
    }
    if !task_service::start_task(task_id, req.agent_id, &state.db, &state.works) {
        return Err(AppError::Conflict("Task already running".to_string()));
    }
    Ok(())
}
