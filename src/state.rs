// AppState: 数据库连接池、定时任务调度器、任务执行句柄(含停止信号)、对话状态表、技能列表
use std::sync::Arc;

use dashmap::DashMap;
use sqlx::SqlitePool;
use tokio::sync::RwLock;
use tokio::task::JoinHandle;
use tokio_cron_scheduler::JobScheduler;

// 对话流式状态
pub struct ConversationState {
    pub chunks: Vec<serde_json::Value>,
    // 回放查询会话标记: true 表示仅回放历史不执行模型调用, 不计入运行状态
    pub query: bool,
    // 新 chunk 通知, 对话结束时置 None 关闭通道即完成信号(与 entry 移除同点发生, 保证完成状态只有一个事实来源)
    pub notify: Option<tokio::sync::watch::Sender<u64>>,
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
    pub task_loops: Arc<DashMap<i64, (JoinHandle<()>, tokio::sync::watch::Sender<bool>)>>,
    pub conversation_states: Arc<DashMap<i64, Arc<RwLock<ConversationState>>>>,
    pub skills: Arc<std::sync::RwLock<Vec<SkillInfo>>>,
}
