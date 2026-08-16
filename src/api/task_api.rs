// 任务 CRUD API
use axum::extract::{Path, State};
use axum::Json;
use serde::Deserialize;
use serde_json::json;

use crate::error::AppError;
use crate::repository::entity::{MessageEntity, TaskEntity};
use crate::repository::{agent_repository, conversation_repository, task_repository};
use crate::service::task_service;
use crate::state::AppState;

// 任务列表接口, 按 id 升序返回全部任务(仅基本字段, 不含阶段对话)
pub async fn list_tasks(State(state): State<AppState>) -> Result<Json<Vec<TaskEntity>>, AppError> {
    let tasks = task_repository::list_tasks(&state.db).await?;
    Ok(Json(tasks))
}

// 任务详情接口, 包含各阶段对话(对话按 id 升序, 每条对话含全部按 id 升序的消息), 任务不存在返回 404
pub async fn get_task(
    State(state): State<AppState>,
    Path(task_id): Path<i64>,
) -> Result<Json<serde_json::Value>, AppError> {
    let task = task_repository::get_task(&state.db, task_id).await?;
    let task = match task {
        Some(t) => t,
        None => return Err(AppError::NotFound("Task not found".to_string())),
    };

    // 查询任务的全部阶段对话, 按 id 升序
    let conversations =
        conversation_repository::list_conversations_by_task_id(&state.db, task_id).await?;

    // 批量查询全部阶段对话的消息(一次 IN 查询 + 内存分组, 避免 N+1), 按 id 升序
    let mut message_map: std::collections::HashMap<i64, Vec<MessageEntity>> =
        std::collections::HashMap::new();
    if !conversations.is_empty() {
        let conversation_ids: Vec<i64> = conversations.iter().map(|c| c.id).collect();
        let messages = conversation_repository::list_messages_by_conversation_ids(
            &state.db,
            &conversation_ids,
        )
        .await?;
        for message in messages {
            message_map
                .entry(message.conversation_id)
                .or_default()
                .push(message);
        }
    }

    let conversations: Vec<serde_json::Value> = conversations.into_iter().map(|c| {
        let messages = message_map.remove(&c.id).unwrap_or_default();
        // role/time 从 content(pi 消息 JSON)派生
        let messages: Vec<serde_json::Value> = messages.iter().map(|msg| {
            json!({
                "id": msg.id,
                "role": msg.content["role"].as_str().unwrap_or(""),
                "content": msg.content,
                "time": crate::repository::entity::timestamp_to_rfc3339(msg.content["timestamp"].as_u64().unwrap_or(0)),
            })
        }).collect();
        json!({
            "id": c.id,
            "task_id": c.task_id,
            "agent_id": c.agent_id,
            "title": c.title,
            "work_dir": c.work_dir,
            "create_time": c.create_time,
            "update_time": c.update_time,
            "messages": messages,
        })
    }).collect();

    let t = &task;
    Ok(Json(json!({
        "id": t.id,
        "title": t.title,
        "content": t.content,
        "agent_ids": t.agent_ids,
        "work_dir": t.work_dir,
        "create_time": t.create_time,
        "update_time": t.update_time,
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

// 新增任务接口, agent_ids 为可供 Agent 选择下一个执行者的候选池, work_dir 为任务阶段对话的工作目录, 返回自增 id
pub async fn add_task(
    State(state): State<AppState>,
    Json(req): Json<AddTaskRequest>,
) -> Result<Json<i64>, AppError> {
    let id = task_repository::add_task(
        &state.db,
        &req.title,
        &req.content,
        &req.agent_ids,
        &req.work_dir,
    )
    .await?;
    Ok(Json(id))
}

// 删除任务接口, 关联对话由数据库外键 ON DELETE CASCADE 级联删除, 任务不存在返回 404, 正在运行返回 409
pub async fn delete_task(
    State(state): State<AppState>,
    Path(task_id): Path<i64>,
) -> Result<(), AppError> {
    // 运行中的任务不允许删除, 避免级联删除阶段对话导致后台循环写消息失败
    if task_service::is_task_running(&state, task_id) {
        return Err(AppError::Conflict("Task is running".to_string()));
    }
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

// 启动任务执行循环接口, agent_id 为首个执行的 Agent, 阶段对话的工作目录取任务的 work_dir
// 任务/agent 不存在返回 404, 执行循环已在运行返回 409
pub async fn start_task(
    State(state): State<AppState>,
    Path(task_id): Path<i64>,
    Json(req): Json<StartTaskRequest>,
) -> Result<(), AppError> {
    let task = task_repository::get_task(&state.db, task_id).await?;
    if task.is_none() {
        return Err(AppError::NotFound("Task not found".to_string()));
    }
    let agent = agent_repository::get_agent(&state.db, req.agent_id).await?;
    if agent.is_none() {
        return Err(AppError::NotFound("Agent not found".to_string()));
    }
    if !task_service::start_task(&state, task_id, req.agent_id) {
        return Err(AppError::Conflict("Task already running".to_string()));
    }
    Ok(())
}
