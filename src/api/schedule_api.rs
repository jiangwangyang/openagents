// 定时任务 CRUD API
use axum::extract::{Path, State};
use axum::Json;
use serde::Deserialize;
use serde_json::json;

use crate::error::AppError;
use crate::repository::schedule_repository;
use crate::service::schedule_service;
use crate::state::AppState;

// 定时任务列表接口，按 id 升序返回全部任务，含下次触发时间
pub async fn list_schedules(State(state): State<AppState>) -> Result<Json<Vec<serde_json::Value>>, AppError> {
    let schedules = schedule_repository::list_schedules(&state.db).await?;
    let result: Vec<serde_json::Value> = schedules.iter().map(|s| {
        json!({
            "id": s.id,
            "name": s.name,
            "content": s.content,
            "work_dir": s.work_dir,
            "trigger": s.cron_expr,
            "agent_id": s.agent_id,
            "enabled": s.enabled,
            "next_fire_time": schedule_service::next_fire_time(&s.cron_expr),
            "create_time": s.create_time,
            "update_time": s.update_time,
        })
    }).collect();
    Ok(Json(result))
}

// 定时任务详情接口，不存在返回 404
pub async fn get_schedule(State(state): State<AppState>, Path(schedule_id): Path<i64>) -> Result<Json<serde_json::Value>, AppError> {
    let schedule = schedule_repository::get_schedule(&state.db, schedule_id).await?;
    let s = match schedule {
        Some(s) => s,
        None => return Err(AppError::NotFound("Schedule not found".to_string())),
    };
    Ok(Json(json!({
        "id": s.id,
        "name": s.name,
        "content": s.content,
        "work_dir": s.work_dir,
        "trigger": s.cron_expr,
        "agent_id": s.agent_id,
        "enabled": s.enabled,
        "next_fire_time": schedule_service::next_fire_time(&s.cron_expr),
        "create_time": s.create_time,
        "update_time": s.update_time,
    })))
}

// 新增定时任务请求体
#[derive(Debug, Deserialize)]
pub struct AddScheduleRequest {
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
}

// 新增定时任务接口，返回自增 id
pub async fn add_schedule(State(state): State<AppState>, Json(req): Json<AddScheduleRequest>) -> Result<Json<i64>, AppError> {
    let cron_expr = format!("{} {} {} {} {} {}", req.second, req.minute, req.hour, req.day, req.month, req.day_of_week);
    let id = schedule_service::add_schedule(&state.db, &state.conversations, &req.name, &req.content, &req.work_dir, &cron_expr, req.agent_id).await?;
    Ok(Json(id))
}

// 更新定时任务请求体
#[derive(Debug, Deserialize)]
pub struct UpdateScheduleRequest {
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

// 按 id 更新定时任务，不存在返回 404
pub async fn update_schedule(State(state): State<AppState>, Path(schedule_id): Path<i64>, Json(req): Json<UpdateScheduleRequest>) -> Result<(), AppError> {
    let cron_expr = format!("{} {} {} {} {} {}", req.second, req.minute, req.hour, req.day, req.month, req.day_of_week);
    let updated = schedule_service::update_schedule(&state.db, &state.conversations, schedule_id, &req.name, &req.content, &req.work_dir, &cron_expr, req.agent_id, req.enabled).await?;
    if !updated {
        return Err(AppError::NotFound("Schedule not found".to_string()));
    }
    Ok(())
}

// 按 id 删除定时任务，不存在返回 404
pub async fn delete_schedule(State(state): State<AppState>, Path(schedule_id): Path<i64>) -> Result<(), AppError> {
    let deleted = schedule_service::delete_schedule(&state.db, schedule_id).await?;
    if !deleted {
        return Err(AppError::NotFound("Schedule not found".to_string()));
    }
    Ok(())
}
