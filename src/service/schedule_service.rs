// 定时任务调度服务
use tokio_cron_scheduler::{Job, JobSchedulerError};

use crate::repository::{agent_repository, conversation_repository, model_provider_repository, schedule_repository};
use crate::service::conversation_service;
use crate::state::AppState;

// 从数据库加载所有启用的定时任务加入调度器
pub async fn init_scheduler(state: &AppState) -> anyhow::Result<()> {
    let schedules = schedule_repository::list_schedules(&state.db).await?;
    for schedule in schedules {
        if schedule.enabled {
            if let Err(e) = add_job_to_scheduler(state, schedule.id, &schedule.cron_expr).await {
                tracing::error!("Failed to load schedule {}: {}", schedule.id, e);
            }
        }
    }
    Ok(())
}

// 将定时任务加入调度器
async fn add_job_to_scheduler(state: &AppState, schedule_id: i64, cron_expr: &str) -> Result<(), JobSchedulerError> {
    let job_state = state.clone();
    let cron = cron_expr.to_string();
    let job = Job::new_async(cron.as_str(), move |_uuid, _lock| {
        let state = job_state.clone();
        Box::pin(async move {
            if let Err(e) = execute_schedule(&state, schedule_id).await {
                tracing::error!("Schedule {} execution failed: {}", schedule_id, e);
            }
        })
    })?;

    let job_id = state.scheduler.add(job).await?;
    state.job_ids.insert(schedule_id, job_id);
    Ok(())
}

// 从调度器移除任务
async fn remove_job_from_scheduler(state: &AppState, schedule_id: i64) {
    if let Some(job_id) = state.job_ids.remove(&schedule_id) {
        let _ = state.scheduler.remove(&job_id.1).await;
    }
}

// 执行定时任务：创建对话并触发 agent 执行
async fn execute_schedule(state: &AppState, schedule_id: i64) -> anyhow::Result<()> {
    let schedule = schedule_repository::get_schedule(&state.db, schedule_id)
        .await?
        .ok_or_else(|| anyhow::anyhow!("schedule not found"))?;

    let agent = agent_repository::get_agent(&state.db, schedule.agent_id)
        .await?
        .ok_or_else(|| anyhow::anyhow!("agent not found"))?;
    // 模型提供方由上层按需查询
    let provider = model_provider_repository::get_model_provider(&state.db, agent.model_provider_id)
        .await?
        .ok_or_else(|| anyhow::anyhow!("model provider not configured"))?;

    // 创建对话
    let conversation_id = conversation_repository::add_conversation(
        &state.db,
        &format!("[定时] {}", schedule.name),
        &schedule.work_dir,
        &agent.prompt,
        None,
        Some(schedule.agent_id),
    )
    .await?;

    // 触发对话执行,启动失败时记录日志,避免对话已落库但未执行的静默失败
    let started = conversation_service::start_conversation(
        state,
        conversation_id,
        schedule.content.clone(),
        provider.id,
        agent.model.clone(),
        agent.thinking,
    )
    .await;
    if !started {
        tracing::warn!("Schedule {} conversation {} failed to start", schedule_id, conversation_id);
    }
    tracing::info!("Schedule {} triggered conversation {}", schedule_id, conversation_id);
    Ok(())
}

// 新增定时任务并加入调度器
pub async fn add_schedule(state: &AppState, name: &str, content: &str, work_dir: &str, cron_expr: &str, agent_id: i64) -> anyhow::Result<i64> {
    let id = schedule_repository::add_schedule(&state.db, name, content, work_dir, cron_expr, agent_id).await?;
    // 加入调度器失败时回滚数据库,避免产生无法调度的启用任务
    if let Err(e) = add_job_to_scheduler(state, id, cron_expr).await {
        let _ = schedule_repository::delete_schedule(&state.db, id).await;
        return Err(e.into());
    }
    Ok(id)
}

// 更新定时任务并重置调度器
pub async fn update_schedule(state: &AppState, schedule_id: i64, name: &str, content: &str, work_dir: &str, cron_expr: &str, agent_id: i64, enabled: bool) -> anyhow::Result<bool> {
    // 先取出旧数据,用于加入调度器失败时还原
    let old = schedule_repository::get_schedule(&state.db, schedule_id).await?;
    let updated = schedule_repository::update_schedule(&state.db, schedule_id, name, content, work_dir, cron_expr, agent_id, enabled).await?;
    if updated {
        remove_job_from_scheduler(state, schedule_id).await;
        if enabled {
            // 加入调度器失败时还原旧数据并恢复旧调度,保持数据库与调度器一致
            if let Err(e) = add_job_to_scheduler(state, schedule_id, cron_expr).await {
                if let Some(old) = old {
                    let _ = schedule_repository::update_schedule(&state.db, schedule_id, &old.name, &old.content, &old.work_dir, &old.cron_expr, old.agent_id, old.enabled).await;
                    if old.enabled {
                        let _ = add_job_to_scheduler(state, schedule_id, &old.cron_expr).await;
                    }
                }
                return Err(e.into());
            }
        }
    }
    Ok(updated)
}

// 删除定时任务并从调度器移除
pub async fn delete_schedule(state: &AppState, schedule_id: i64) -> anyhow::Result<bool> {
    remove_job_from_scheduler(state, schedule_id).await;
    let deleted = schedule_repository::delete_schedule(&state.db, schedule_id).await?;
    Ok(deleted)
}

// 计算 cron 表达式的下次触发时间（UTC 转本地）
pub fn next_fire_time(cron_expr: &str) -> Option<String> {
    use std::str::FromStr;
    let schedule = cron::Schedule::from_str(cron_expr).ok()?;
    schedule.upcoming(chrono::Local).next().map(|t| t.to_rfc3339())
}
