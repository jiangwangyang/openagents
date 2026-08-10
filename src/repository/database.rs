// 连接池、建表
use sqlx::sqlite::{SqliteConnectOptions, SqlitePoolOptions};
use sqlx::SqlitePool;
use std::str::FromStr;

use crate::config;

// 创建数据库连接池并初始化
pub async fn init_db() -> anyhow::Result<SqlitePool> {
    // 确保数据目录存在
    let db_file = config::database_file();
    if let Some(parent) = db_file.parent() {
        std::fs::create_dir_all(parent)?;
    }

    let db_url = format!("sqlite:{}", db_file.display());
    let options = SqliteConnectOptions::from_str(&db_url)?
        .create_if_missing(true)
        .pragma("journal_mode", "WAL")
        .pragma("foreign_keys", "ON");

    // WAL 模式下读写可并发,连接数放宽到 5;写事务仍串行,由 sqlx 默认 5s busy_timeout 兜底
    let pool = SqlitePoolOptions::new()
        .max_connections(5)
        .connect_with(options)
        .await?;

    // 建表
    create_tables(&pool).await?;

    Ok(pool)
}

// 创建所有表
async fn create_tables(pool: &SqlitePool) -> anyhow::Result<()> {
    sqlx::query(
        "CREATE TABLE IF NOT EXISTS t_model_provider (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            name TEXT NOT NULL,
            protocol_type TEXT NOT NULL,
            base_url TEXT NOT NULL,
            api_key TEXT NOT NULL,
            create_time TEXT NOT NULL,
            update_time TEXT NOT NULL
        )",
    )
        .execute(pool)
        .await?;

    sqlx::query(
        "CREATE TABLE IF NOT EXISTS t_agent (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            name TEXT NOT NULL,
            description TEXT NOT NULL,
            prompt TEXT NOT NULL,
            model_provider_id INTEGER NOT NULL REFERENCES t_model_provider(id) ON DELETE RESTRICT,
            model TEXT NOT NULL,
            thinking INTEGER NOT NULL,
            create_time TEXT NOT NULL,
            update_time TEXT NOT NULL
        )",
    )
        .execute(pool)
        .await?;

    sqlx::query(
        "CREATE TABLE IF NOT EXISTS t_task (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            title TEXT NOT NULL,
            content TEXT NOT NULL,
            agent_ids TEXT NOT NULL,
            work_dir TEXT NOT NULL,
            create_time TEXT NOT NULL,
            update_time TEXT NOT NULL
        )",
    )
        .execute(pool)
        .await?;

    sqlx::query(
        "CREATE TABLE IF NOT EXISTS t_conversation (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            task_id INTEGER REFERENCES t_task(id) ON DELETE CASCADE,
            agent_id INTEGER REFERENCES t_agent(id) ON DELETE RESTRICT,
            title TEXT NOT NULL,
            work_dir TEXT NOT NULL,
            system_prompt TEXT NOT NULL,
            create_time TEXT NOT NULL,
            update_time TEXT NOT NULL
        )",
    )
        .execute(pool)
        .await?;

    sqlx::query(
        "CREATE TABLE IF NOT EXISTS t_message (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            conversation_id INTEGER NOT NULL REFERENCES t_conversation(id) ON DELETE CASCADE,
            role TEXT NOT NULL,
            content TEXT NOT NULL,
            stop_reason TEXT NOT NULL,
            cache_read_input_tokens INTEGER NOT NULL,
            input_tokens INTEGER NOT NULL,
            output_tokens INTEGER NOT NULL,
            time TEXT NOT NULL
        )",
    )
        .execute(pool)
        .await?;

    sqlx::query(
        "CREATE TABLE IF NOT EXISTS t_schedule (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            name TEXT NOT NULL,
            content TEXT NOT NULL,
            work_dir TEXT NOT NULL,
            cron_expr TEXT NOT NULL,
            agent_id INTEGER REFERENCES t_agent(id) ON DELETE RESTRICT,
            enabled INTEGER NOT NULL DEFAULT 1,
            create_time TEXT NOT NULL,
            update_time TEXT NOT NULL
        )",
    )
        .execute(pool)
        .await?;

    sqlx::query(
        "CREATE TABLE IF NOT EXISTS t_mcp_server (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            name TEXT NOT NULL,
            description TEXT NOT NULL,
            protocol_type TEXT NOT NULL,
            url TEXT,
            headers TEXT,
            command TEXT,
            args TEXT,
            create_time TEXT NOT NULL,
            update_time TEXT NOT NULL
        )",
    )
        .execute(pool)
        .await?;

    sqlx::query(
        "CREATE TABLE IF NOT EXISTS t_web_storage (
            key TEXT PRIMARY KEY,
            value TEXT NOT NULL,
            create_time TEXT NOT NULL,
            update_time TEXT NOT NULL
        )",
    )
        .execute(pool)
        .await?;

    // 创建索引
    sqlx::query("CREATE INDEX IF NOT EXISTS idx_message_conversation ON t_message(conversation_id)")
        .execute(pool)
        .await?;

    sqlx::query("CREATE INDEX IF NOT EXISTS idx_conversation_task ON t_conversation(task_id)")
        .execute(pool)
        .await?;

    Ok(())
}
