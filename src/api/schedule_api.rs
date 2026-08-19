// 定时任务 CRUD API
use std::str::FromStr;

use axum::extract::{Path, State};
use axum::Json;
use serde::Deserialize;
use serde_json::json;

use crate::error::AppError;
use crate::repository::entity::ScheduleEntity;
use crate::repository::{conversation_repository, schedule_repository};
use crate::service::{conversation_service, schedule_service};
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

// ScheduleEntity -> ScheduleResponse: cron_expr 更名 trigger, 附带下次触发时间
impl From<ScheduleEntity> for ScheduleResponse {
    fn from(s: ScheduleEntity) -> Self {
        ScheduleResponse {
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
        }
    }
}

// 定时任务列表接口, 按 id 升序返回全部任务, 含下次触发时间
pub async fn list_schedules(
    State(state): State<AppState>,
) -> Result<Json<Vec<ScheduleResponse>>, AppError> {
    let schedules = schedule_repository::list_schedules(&state.db).await?;
    Ok(Json(
        schedules.into_iter().map(ScheduleResponse::from).collect(),
    ))
}

// 定时任务详情接口, 包含外键关联的执行 Agent 与全部执行对话(对话按 id 升序, 每条对话含全部按 id 升序的消息), 不存在返回 404
pub async fn get_schedule(
    State(state): State<AppState>,
    Path(schedule_id): Path<i64>,
) -> Result<Json<serde_json::Value>, AppError> {
    let s = schedule_repository::get_schedule(&state.db, schedule_id)
        .await?
        .ok_or_else(|| AppError::NotFound("Schedule not found".to_string()))?;

    // 查询定时任务的全部执行对话并组装(对话按 id 升序, 每条对话含全部按 id 升序的消息)
    let conversations =
        conversation_repository::list_conversations_by_schedule_id(&state.db, schedule_id).await?;
    let conversations = conversation_service::conversations_to_json(&state, conversations).await?;

    // 定时任务基本字段由响应体序列化展开, 追加外键关联的执行 Agent 与执行对话
    let mut result = serde_json::to_value(&ScheduleResponse::from(s.schedule))?;
    result["agent"] = json!(s.agent);
    result["conversations"] = json!(conversations);
    Ok(Json(result))
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
