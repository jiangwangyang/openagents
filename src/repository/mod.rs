// 数据访问层: sqlx 手写 SQL
pub mod agent_repository;
pub mod conversation_repository;
pub mod database;
pub mod entity;
pub mod mcp_server_repository;
pub mod model_provider_repository;
pub mod schedule_repository;
pub mod task_repository;
pub mod web_storage_repository;

// 删除结果: 已删除/不存在/被其它实体引用(仅带外键引用保护的删除使用)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DeleteResult {
    Deleted,
    NotFound,
    Referenced,
}
