// AppState:数据库连接池、work 状态表
use std::sync::Arc;
use dashmap::DashMap;
use sqlx::SqlitePool;
use tokio::sync::RwLock;

// Work 流式状态
pub struct WorkState {
    pub chunks: Vec<serde_json::Value>,
    pub done: bool,
    pub notify: tokio::sync::watch::Sender<u64>,
}

// 全局应用状态
#[derive(Clone)]
pub struct AppState {
    pub db: SqlitePool,
    pub works: Arc<DashMap<i64, Arc<RwLock<WorkState>>>>,
}
