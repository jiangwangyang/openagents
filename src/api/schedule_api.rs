// 定时任务 CRUD API
use axum::extract::{Path, State};
use axum::Json;
use serde::Deserialize;
use serde_json::json;
use std::str::FromStr;

use crate::error::AppError;
use crate::repository::entity::MessageEntity;
use crate::repository::{conversation_repository, schedule_repository};
use crate::service::schedule_service;
use crate::state::AppState;

// 定时任务响应体(trigger 即 cron 表达式, 含下次触发时间)
#[derive(Debug, serde::Serialize)]
pub struct ScheduleResponse {
    pub id: i64,
    pub name: String,
    pub content: String,
    pub work_dir: String,
    pub trigger: String,
    pub agent_id: i64,
    pub enabled: bool,
    pub next_fire_time: Option<String>,
    pub create_time: String,
    pub update_time: String,
}

// 定时任务列表接口, 按 id 升序返回全部任务, 含下次触发时间
pub async fn list_schedules(
    State(state): State<AppState>,
) -> Result<Json<Vec<ScheduleResponse>>, AppError> {
    let schedules = schedule_repository::list_schedules(&state.db).await?;
    let result: Vec<ScheduleResponse> = schedules
        .into_iter()
        .map(|s| ScheduleResponse {
            id: s.id,
            name: s.name,
            content: s.content,
            work_dir: s.work_dir,
            trigger: s.cron_expr.clone(),
            agent_id: s.agent_id,
            enabled: s.enabled,
            next_fire_time: schedule_service::next_fire_time(&s.cron_expr),
            create_time: s.create_time,
            update_time: s.update_time,
        })
        .collect();
    Ok(Json(result))
}

// 定时任务详情接口, 包含全部执行对话(对话按 id 升序, 每条对话含全部按 id 升序的消息), 不存在返回 404
pub async fn get_schedule(
    State(state): State<AppState>,
    Path(schedule_id): Path<i64>,
) -> Result<Json<serde_json::Value>, AppError> {
    let schedule = schedule_repository::get_schedule(&state.db, schedule_id).await?;
    let s = match schedule {
        Some(s) => s,
        None => return Err(AppError::NotFound("Schedule not found".to_string())),
    };

    // 查询定时任务的全部执行对话, 按 id 升序
    let conversations =
        conversation_repository::list_conversations_by_schedule_id(&state.db, schedule_id).await?;

    // 批量查询全部执行对话的消息(一次 IN 查询 + 内存分组, 避免 N+1), 按 id 升序
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

    let conversations: Vec<serde_json::Value> = conversations
        .into_iter()
        .map(|c| {
            // messages 直接返回数据库字段(id/conversation_id/content)
            let messages = message_map.remove(&c.id).unwrap_or_default();
            json!({
                "id": c.id,
                "schedule_id": c.schedule_id,
                "agent_id": c.agent_id,
                "title": c.title,
                "work_dir": c.work_dir,
                "create_time": c.create_time,
                "update_time": c.update_time,
                "messages": messages,
            })
        })
        .collect();

    Ok(Json(json!({
        "id": s.id,
        "name": s.name,
        "content": s.content,
        "work_dir": s.work_dir,
        "trigger": s.cron_expr.clone(),
        "agent_id": s.agent_id,
        "enabled": s.enabled,
        "next_fire_time": schedule_service::next_fire_time(&s.cron_expr),
        "create_time": s.create_time,
        "update_time": s.update_time,
        "conversations": conversations,
    })))
}

// 定时任务新增/更新请求体(新增时 enabled 字段忽略, 默认启用)
#[derive(Debug, Deserialize)]
pub struct ScheduleRequest {
    pub name: String,
    pub content: String,
    pub work_dir: String,
    pub minute: String,
    pub hour: String,
    pub day: String,
    pub month: String,
    pub day_of_week: String,
    pub second: String,
    pub agent_id: i64,
    pub enabled: bool,
}

// 新增定时任务接口, 返回自增 id
pub async fn add_schedule(
    State(state): State<AppState>,
    Json(req): Json<ScheduleRequest>,
) -> Result<Json<i64>, AppError> {
    let cron_expr = format!(
        "{} {} {} {} {} {}",
        req.second, req.minute, req.hour, req.day, req.month, req.day_of_week
    );
    // 校验 cron 表达式合法性(与调度器使用同一 cron 解析器)
    if cron::Schedule::from_str(&cron_expr).is_err() {
        return Err(AppError::BadRequest("Invalid cron expression".to_string()));
    }
    let id = schedule_service::add_schedule(
        &state,
        &req.name,
        &req.content,
        &req.work_dir,
        &cron_expr,
        req.agent_id,
    )
    .await?;
    Ok(Json(id))
}

// 按 id 更新定时任务, 不存在返回 404
pub async fn update_schedule(
    State(state): State<AppState>,
    Path(schedule_id): Path<i64>,
    Json(req): Json<ScheduleRequest>,
) -> Result<(), AppError> {
    let cron_expr = format!(
        "{} {} {} {} {} {}",
        req.second, req.minute, req.hour, req.day, req.month, req.day_of_week
    );
    // 校验 cron 表达式合法性(与调度器使用同一 cron 解析器)
    if cron::Schedule::from_str(&cron_expr).is_err() {
        return Err(AppError::BadRequest("Invalid cron expression".to_string()));
    }
    let updated = schedule_service::update_schedule(
        &state,
        schedule_id,
        &req.name,
        &req.content,
        &req.work_dir,
        &cron_expr,
        req.agent_id,
        req.enabled,
    )
    .await?;
    if !updated {
        return Err(AppError::NotFound("Schedule not found".to_string()));
    }
    Ok(())
}

// 按 id 删除定时任务, 不存在返回 404
pub async fn delete_schedule(
    State(state): State<AppState>,
    Path(schedule_id): Path<i64>,
) -> Result<(), AppError> {
    let deleted = schedule_service::delete_schedule(&state, schedule_id).await?;
    if !deleted {
        return Err(AppError::NotFound("Schedule not found".to_string()));
    }
    Ok(())
}
