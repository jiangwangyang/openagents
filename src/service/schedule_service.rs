// 定时任务调度服务
use std::sync::Arc;
use dashmap::DashMap;
use sqlx::SqlitePool;
use tokio_cron_scheduler::{Job, JobScheduler, JobSchedulerError};

use crate::repository::{agent_repository, conversation_repository, schedule_repository};
use crate::service::conversation_service;
use crate::state::ConversationState;

// 调度器全局句柄
static SCHEDULER: std::sync::LazyLock<Arc<tokio::sync::Mutex<Option<JobScheduler>>>> =
    std::sync::LazyLock::new(|| Arc::new(tokio::sync::Mutex::new(None)));

// 每个 schedule 对应的 scheduler job uuid
static JOB_IDS: std::sync::LazyLock<Arc<DashMap<i64, uuid::Uuid>>> =
    std::sync::LazyLock::new(|| Arc::new(DashMap::new()));

// 初始化调度器并从数据库加载所有启用的定时任务
pub async fn init_scheduler(db: &SqlitePool, conversations: &Arc<DashMap<i64, Arc<tokio::sync::RwLock<ConversationState>>>>) -> anyhow::Result<()> {
    let scheduler = JobScheduler::new().await?;
    scheduler.start().await?;
    {
        let mut guard = SCHEDULER.lock().await;
        *guard = Some(scheduler);
    }
    // 加载所有启用的任务
    let schedules = schedule_repository::list_schedules(db).await?;
    for schedule in schedules {
        if schedule.enabled {
            if let Err(e) = add_job_to_scheduler(schedule.id, &schedule.cron_expr, db, conversations).await {
                tracing::error!("Failed to load schedule {}: {}", schedule.id, e);
            }
        }
    }
    Ok(())
}

// 将定时任务加入调度器
async fn add_job_to_scheduler(schedule_id: i64, cron_expr: &str, db: &SqlitePool, conversations: &Arc<DashMap<i64, Arc<tokio::sync::RwLock<ConversationState>>>>) -> Result<(), JobSchedulerError> {
    let guard = SCHEDULER.lock().await;
    let scheduler = match guard.as_ref() {
        Some(s) => s.clone(),
        None => return Ok(()),
    };
    drop(guard);

    let db = db.clone();
    let conversations = conversations.clone();
    let cron = cron_expr.to_string();
    let job = Job::new_async(cron.as_str(), move |_uuid, _lock| {
        let db = db.clone();
        let conversations = conversations.clone();
        Box::pin(async move {
            if let Err(e) = execute_schedule(schedule_id, &db, &conversations).await {
                tracing::error!("Schedule {} execution failed: {}", schedule_id, e);
            }
        })
    })?;

    let job_id = scheduler.add(job).await?;
    JOB_IDS.insert(schedule_id, job_id);
    Ok(())
}

// 从调度器移除任务
async fn remove_job_from_scheduler(schedule_id: i64) {
    if let Some(job_id) = JOB_IDS.remove(&schedule_id) {
        let guard = SCHEDULER.lock().await;
        if let Some(scheduler) = guard.as_ref() {
            let _ = scheduler.remove(&job_id.1).await;
        }
    }
}

// 执行定时任务：创建对话并触发 agent 执行
async fn execute_schedule(schedule_id: i64, db: &SqlitePool, conversations: &Arc<DashMap<i64, Arc<tokio::sync::RwLock<ConversationState>>>>) -> anyhow::Result<()> {
    let schedule = schedule_repository::get_schedule(db, schedule_id)
        .await?
        .ok_or_else(|| anyhow::anyhow!("schedule not found"))?;

    let agent = agent_repository::get_agent(db, schedule.agent_id)
        .await?
        .ok_or_else(|| anyhow::anyhow!("agent not found"))?;
    let provider = agent.model_provider.as_ref()
        .ok_or_else(|| anyhow::anyhow!("model provider not configured"))?;

    // 创建对话
    let conversation_id = conversation_repository::add_conversation(
        db,
        &format!("[定时] {}", schedule.name),
        &schedule.work_dir,
        &agent.prompt,
        None,
        Some(schedule.agent_id),
    )
    .await?;

    // 触发对话执行
    conversation_service::start_conversation(
        conversation_id,
        schedule.content.clone(),
        provider.id,
        agent.model.clone(),
        agent.thinking,
        conversations,
        db,
    )
    .await;
    tracing::info!("Schedule {} triggered conversation {}", schedule_id, conversation_id);
    Ok(())
}

// 新增定时任务并加入调度器
pub async fn add_schedule(db: &SqlitePool, conversations: &Arc<DashMap<i64, Arc<tokio::sync::RwLock<ConversationState>>>>, name: &str, content: &str, work_dir: &str, cron_expr: &str, agent_id: i64) -> anyhow::Result<i64> {
    let id = schedule_repository::add_schedule(db, name, content, work_dir, cron_expr, agent_id).await?;
    add_job_to_scheduler(id, cron_expr, db, conversations).await?;
    Ok(id)
}

// 更新定时任务并重置调度器
pub async fn update_schedule(db: &SqlitePool, conversations: &Arc<DashMap<i64, Arc<tokio::sync::RwLock<ConversationState>>>>, schedule_id: i64, name: &str, content: &str, work_dir: &str, cron_expr: &str, agent_id: i64, enabled: bool) -> anyhow::Result<bool> {
    let updated = schedule_repository::update_schedule(db, schedule_id, name, content, work_dir, cron_expr, agent_id, enabled).await?;
    if updated {
        remove_job_from_scheduler(schedule_id).await;
        if enabled {
            add_job_to_scheduler(schedule_id, cron_expr, db, conversations).await?;
        }
    }
    Ok(updated)
}

// 删除定时任务并从调度器移除
pub async fn delete_schedule(db: &SqlitePool, schedule_id: i64) -> anyhow::Result<bool> {
    remove_job_from_scheduler(schedule_id).await;
    let deleted = schedule_repository::delete_schedule(db, schedule_id).await?;
    Ok(deleted)
}

// 计算 cron 表达式的下次触发时间（UTC 转本地）
pub fn next_fire_time(cron_expr: &str) -> Option<String> {
    use std::str::FromStr;
    let schedule = cron::Schedule::from_str(cron_expr).ok()?;
    schedule.upcoming(chrono::Local).next().map(|t| t.to_rfc3339())
}
