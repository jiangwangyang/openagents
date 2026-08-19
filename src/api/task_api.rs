// 任务 CRUD API
use axum::extract::{Path, State};
use axum::Json;
use serde::Deserialize;
use serde_json::json;

use crate::error::AppError;
use crate::repository::entity::{TaskEntity, TASK_STATUS_DONE, TASK_STATUS_RUNNING};
use crate::repository::{agent_repository, conversation_repository, task_repository};
use crate::service::{conversation_service, task_service};
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
    let task = task_repository::get_task(&state.db, task_id)
        .await?
        .ok_or_else(|| AppError::NotFound("Task not found".to_string()))?;

    // 查询任务的全部阶段对话并组装(对话按 id 升序, 每条对话含全部按 id 升序的消息)
    let conversations =
        conversation_repository::list_conversations_by_task_id(&state.db, task_id).await?;
    let conversations = conversation_service::conversations_to_json(&state, conversations).await?;

    // 任务基本字段由实体序列化展开(status 由后端各流转点持久化维护, 前端直接采用), 追加阶段对话与运行状态
    let mut result = serde_json::to_value(&task)?;
    result["conversations"] = json!(conversations);
    // 执行循环是否存活: 前端据此区分运行中(含长轮次执行/阶段交接间隙)与运行失败, 替代 SSE 活跃性探针
    result["running"] = json!(task_service::is_task_running(&state, task_id));
    Ok(Json(result))
}

// 任务新增/更新请求体
#[derive(Debug, Deserialize)]
pub struct TaskRequest {
    pub title: String,
    pub content: String,
    pub agent_ids: Vec<i64>,
    pub work_dir: String,
}

// 新增任务接口, agent_ids 为可供 Agent 选择下一个执行者的候选池, work_dir 为任务阶段对话的工作目录, 返回自增 id
pub async fn add_task(
    State(state): State<AppState>,
    Json(req): Json<TaskRequest>,
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

// 按 id 更新任务接口(状态字段由执行流转维护, 不在编辑范围), 任务不存在返回 404, 正在运行返回 409
pub async fn update_task(
    State(state): State<AppState>,
    Path(task_id): Path<i64>,
    Json(req): Json<TaskRequest>,
) -> Result<(), AppError> {
    // 运行中的任务不允许编辑, 避免与执行循环读取的字段产生竞争
    if task_service::is_task_running(&state, task_id) {
        return Err(AppError::Conflict("Task is running".to_string()));
    }
    let updated = task_repository::update_task(
        &state.db,
        task_id,
        &req.title,
        &req.content,
        &req.agent_ids,
        &req.work_dir,
    )
    .await?;
    if !updated {
        return Err(AppError::NotFound("Task not found".to_string()));
    }
    Ok(())
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

// 启动任务请求体, message 为启动前附加的用户消息(必填)
#[derive(Debug, Deserialize)]
pub struct StartTaskRequest {
    pub agent_id: i64,
    pub message: String,
}

// 启动任务执行循环接口: agent_id 为首个执行的 Agent, message 为启动前附加的用户消息(必填)
// 消息落库规则: 最新对话为用户对话(待审核/已完成)时直接追加, 否则(待启动/运行失败)新建用户对话承载
// 任务/agent 不存在返回 404, message 为空返回 400, 执行循环已在运行返回 409
pub async fn start_task(
    State(state): State<AppState>,
    Path(task_id): Path<i64>,
    Json(req): Json<StartTaskRequest>,
) -> Result<(), AppError> {
    let task = task_repository::get_task(&state.db, task_id)
        .await?
        .ok_or_else(|| AppError::NotFound("Task not found".to_string()))?;
    agent_repository::get_agent(&state.db, req.agent_id)
        .await?
        .ok_or_else(|| AppError::NotFound("Agent not found".to_string()))?;
    let message = req.message.trim();
    if message.is_empty() {
        return Err(AppError::BadRequest("Message is required".to_string()));
    }
    // 运行中的任务提前拒绝, 避免重复写入用户消息
    if task_service::is_task_running(&state, task_id) {
        return Err(AppError::Conflict("Task already running".to_string()));
    }
    // 先落一条用户消息(最新对话为用户对话时直接追加, 否则新建用户对话承载), 再执行后续启动流程
    task_service::append_task_user_message(&state, &task, message, true).await?;
    if !task_service::start_task(&state, task_id, req.agent_id) {
        return Err(AppError::Conflict("Task already running".to_string()));
    }
    // 执行循环启动成功: 任务状态置为运行中
    task_repository::update_task_status(&state.db, task_id, TASK_STATUS_RUNNING).await?;
    Ok(())
}

// 停止任务执行循环接口: 循环收到信号后优雅停止当前对话并退出, 任务未在运行返回 409
pub async fn stop_task(
    State(state): State<AppState>,
    Path(task_id): Path<i64>,
) -> Result<(), AppError> {
    if !task_service::stop_task(&state, task_id) {
        return Err(AppError::Conflict("Task is not running".to_string()));
    }
    Ok(())
}

// 完成任务请求体, message 为用户的完成意见(必填)
#[derive(Debug, Deserialize)]
pub struct CompleteTaskRequest {
    pub message: String,
}

// 完成任务接口: 向最新阶段对话(须为用户审核对话)追加一条用户消息并将任务状态置为已完成, 不启动流水线
// 任务不存在返回 404, message 为空返回 400, 任务运行中或最新对话不是用户审核对话返回 409
pub async fn complete_task(
    State(state): State<AppState>,
    Path(task_id): Path<i64>,
    Json(req): Json<CompleteTaskRequest>,
) -> Result<(), AppError> {
    let task = task_repository::get_task(&state.db, task_id)
        .await?
        .ok_or_else(|| AppError::NotFound("Task not found".to_string()))?;
    let message = req.message.trim();
    if message.is_empty() {
        return Err(AppError::BadRequest("Message is required".to_string()));
    }
    // 运行中的任务不允许完成, 避免与执行循环的状态流转冲突
    if task_service::is_task_running(&state, task_id) {
        return Err(AppError::Conflict("Task is running".to_string()));
    }
    // 最新阶段对话须为用户审核对话(agent_id 为空)才落消息, 否则任务不在待审核状态
    if task_service::append_task_user_message(&state, &task, message, false)
        .await?
        .is_none()
    {
        return Err(AppError::Conflict("Task is not in review".to_string()));
    }
    // 用户已提交完成意见: 任务状态置为已完成
    task_repository::update_task_status(&state.db, task_id, TASK_STATUS_DONE).await?;
    Ok(())
}
