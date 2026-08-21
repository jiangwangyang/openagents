// 定时任务调度服务
use std::str::FromStr;

use tokio_cron_scheduler::{Job, JobSchedulerError};

use crate::repository::{conversation_repository, model_provider_repository, schedule_repository};
use crate::service::conversation_service;
use crate::state::AppState;

// 从数据库加载所有启用的定时任务加入调度器
pub async fn init_scheduler(state: &AppState) -> anyhow::Result<()> {
    let schedules = schedule_repository::list_schedules(&state.db).await?;
    for schedule in schedules {
        if !schedule.enabled {
            continue;
        }
        if let Err(e) = add_job_to_scheduler(state, schedule.id, &schedule.cron_expr).await {
            tracing::error!("Failed to load schedule {}: {}", schedule.id, e);
        }
    }
    Ok(())
}

// 新增定时任务, 启用时加入调度器
pub async fn add_schedule(state: &AppState, name: &str, content: &str, work_dir: &str, cron_expr: &str, agent_id: i64, enabled: bool) -> anyhow::Result<i64> {
    let id = schedule_repository::add_schedule(&state.db, name, content, work_dir, cron_expr, agent_id, enabled).await?;
    // 仅启用的任务加入调度器; 加入失败时回滚数据库, 避免产生无法调度的启用任务
    if enabled {
        if let Err(e) = add_job_to_scheduler(state, id, cron_expr).await {
            let _ = schedule_repository::delete_schedule(&state.db, id).await;
            return Err(e.into());
        }
    }
    Ok(id)
}

// 更新定时任务并重置调度器, 不存在返回 false
pub async fn update_schedule(state: &AppState, schedule_id: i64, name: &str, content: &str, work_dir: &str, cron_expr: &str, agent_id: i64, enabled: bool) -> anyhow::Result<bool> {
    // 先校验任务存在并取出旧数据, 用于加入调度器失败时恢复旧调度
    let old = schedule_repository::get_schedule(&state.db, schedule_id).await?;
    let old = match old {
        Some(o) => o,
        None => return Ok(false),
    };
    // 先重置调度再更新数据库: 加入失败时恢复旧调度并报错, 数据库保持原状, 无需回滚
    remove_job_from_scheduler(state, schedule_id).await;
    if enabled {
        if let Err(e) = add_job_to_scheduler(state, schedule_id, cron_expr).await {
            if old.schedule.enabled {
                let _ = add_job_to_scheduler(state, schedule_id, &old.schedule.cron_expr).await;
            }
            return Err(e.into());
        }
    }
    schedule_repository::update_schedule(&state.db, schedule_id, name, content, work_dir, cron_expr, agent_id, enabled).await?;
    Ok(true)
}

// 删除定时任务并从调度器移除
pub async fn delete_schedule(state: &AppState, schedule_id: i64) -> anyhow::Result<bool> {
    remove_job_from_scheduler(state, schedule_id).await;
    let deleted = schedule_repository::delete_schedule(&state.db, schedule_id).await?;
    Ok(deleted)
}

// 手动触发定时任务: 立即执行一次, 不影响原调度计划, 不存在返回 false
pub async fn trigger_schedule(state: &AppState, schedule_id: i64) -> anyhow::Result<bool> {
    if schedule_repository::get_schedule(&state.db, schedule_id).await?.is_none() {
        return Ok(false);
    }
    execute_schedule(state, schedule_id, "手动").await?;
    Ok(true)
}

// 计算 cron 表达式的下次触发时间(与调度器一致按本地时区解析)
pub fn next_fire_time(cron_expr: &str) -> Option<String> {
    let schedule = cron::Schedule::from_str(cron_expr).ok()?;
    schedule.upcoming(chrono::Local).next().map(|t| t.to_rfc3339())
}

// 将定时任务加入调度器
async fn add_job_to_scheduler(state: &AppState, schedule_id: i64, cron_expr: &str) -> Result<(), JobSchedulerError> {
    let job_state = state.clone();
    let cron = cron_expr.to_string();
    // 显式指定本地时区, 与 next_fire_time 的展示口径一致(默认 UTC 会差一个时区偏移)
    let job = Job::new_async_tz(cron.as_str(), chrono::Local, move |_uuid, _lock| {
        let state = job_state.clone();
        Box::pin(async move {
            if let Err(e) = execute_schedule(&state, schedule_id, "定时").await {
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

// 触发对话执行的类型擦除封装: 同步函数边界隐藏内部 future 类型,
// 打断 start_conversation -> 工具执行 -> schedule_service -> start_conversation 的异步递归类型循环(E0391)
fn start_conversation_boxed(state: &AppState, conversation_id: i64, task_content: String, model_provider_id: i64, model: String, thinking: bool) -> std::pin::Pin<Box<dyn std::future::Future<Output = bool> + Send>> {
    let state = state.clone();
    Box::pin(async move { conversation_service::start_conversation(&state, conversation_id, task_content, model_provider_id, model, thinking, false).await })
}

// 执行定时任务: 创建对话并触发 agent 执行, 对话标题以 trigger_label 为前缀
async fn execute_schedule(state: &AppState, schedule_id: i64, trigger_label: &str) -> anyhow::Result<()> {
    let detail = schedule_repository::get_schedule(&state.db, schedule_id).await?.ok_or_else(|| anyhow::anyhow!("schedule not found"))?;
    let schedule = detail.schedule;
    // 执行 Agent 已由 get_schedule 单条 SQL 关联查出, 引用删除保护保证关联必然命中
    let agent = detail.agent.ok_or_else(|| anyhow::anyhow!("agent not found"))?;
    let provider = model_provider_repository::get_model_provider(&state.db, agent.model_provider_id).await?.ok_or_else(|| anyhow::anyhow!("model provider not configured"))?;

    // 创建对话
    let conversation_id = conversation_repository::add_conversation(&state.db, &format!("[{}] {}", trigger_label, schedule.name), &schedule.work_dir, &agent.prompt, None, Some(schedule.agent_id), Some(schedule_id)).await?;

    // 触发对话执行, 启动失败时记录日志, 避免对话已落库但未执行的静默失败
    let started = start_conversation_boxed(state, conversation_id, schedule.content.clone(), provider.id, agent.model.clone(), agent.thinking).await;
    if !started {
        tracing::warn!("Schedule {} conversation {} failed to start", schedule_id, conversation_id);
    }
    tracing::info!("Schedule {} triggered conversation {}", schedule_id, conversation_id);
    Ok(())
}
