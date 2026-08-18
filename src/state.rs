// AppState: 数据库连接池、定时任务调度器、任务执行句柄、对话状态表、技能列表
use std::sync::Arc;

use dashmap::DashMap;
use sqlx::SqlitePool;
use tokio::sync::RwLock;
use tokio::task::JoinHandle;
use tokio_cron_scheduler::JobScheduler;

// 对话流式状态
pub struct ConversationState {
    pub chunks: Vec<serde_json::Value>,
    pub done: bool,
    pub notify: tokio::sync::watch::Sender<u64>,
    pub stop: tokio::sync::watch::Sender<bool>,
}

// Skill 信息
#[derive(Debug, Clone, serde::Serialize)]
pub struct SkillInfo {
    pub name: String,
    pub description: String,
    pub path: String,
    pub content: String,
}

// 全局应用状态
#[derive(Clone)]
pub struct AppState {
    pub db: SqlitePool,
    pub scheduler: JobScheduler,
    pub job_ids: Arc<DashMap<i64, uuid::Uuid>>,
    pub task_loops: Arc<DashMap<i64, JoinHandle<()>>>,
    pub conversation_states: Arc<DashMap<i64, Arc<RwLock<ConversationState>>>>,
    pub skills: Arc<std::sync::RwLock<Vec<SkillInfo>>>,
}
