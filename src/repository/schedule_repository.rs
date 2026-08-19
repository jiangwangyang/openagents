// 定时任务 CRUD
use sqlx::SqlitePool;

use super::entity::{ScheduleAgentRow, ScheduleEntity, ScheduleWithAgent};

// 查询全部定时任务, 按 id 升序
pub async fn list_schedules(pool: &SqlitePool) -> Result<Vec<ScheduleEntity>, sqlx::Error> {
    sqlx::query_as::<_, ScheduleEntity>(
        "SELECT id, name, content, work_dir, cron_expr, agent_id, enabled, create_time, update_time FROM t_schedule ORDER BY id",
    )
    .fetch_all(pool)
    .await
}

// 按 id 查询定时任务, 单条 SQL LEFT JOIN 关联执行 Agent
pub async fn get_schedule(
    pool: &SqlitePool,
    schedule_id: i64,
) -> Result<Option<ScheduleWithAgent>, sqlx::Error> {
    let row = sqlx::query_as::<_, ScheduleAgentRow>(
        "SELECT s.id, s.name, s.content, s.work_dir, s.cron_expr, s.agent_id, s.enabled, s.create_time, s.update_time, a.id AS agent_ref_id, a.name AS agent_name, a.description AS agent_description, a.prompt AS agent_prompt, a.model_provider_id AS agent_model_provider_id, a.model AS agent_model, a.thinking AS agent_thinking, a.create_time AS agent_create_time, a.update_time AS agent_update_time FROM t_schedule s LEFT JOIN t_agent a ON a.id = s.agent_id WHERE s.id = ?",
    )
    .bind(schedule_id)
    .fetch_optional(pool)
    .await?;
    Ok(row.map(ScheduleWithAgent::from))
}

// 新增定时任务, 返回自增 id
pub async fn add_schedule(
    pool: &SqlitePool,
    name: &str,
    content: &str,
    work_dir: &str,
    cron_expr: &str,
    agent_id: i64,
) -> Result<i64, sqlx::Error> {
    let now = chrono::Local::now().to_rfc3339();
    let result = sqlx::query(
        "INSERT INTO t_schedule (name, content, work_dir, cron_expr, agent_id, enabled, create_time, update_time) VALUES (?, ?, ?, ?, ?, 1, ?, ?)",
    )
    .bind(name)
    .bind(content)
    .bind(work_dir)
    .bind(cron_expr)
    .bind(agent_id)
    .bind(&now)
    .bind(&now)
    .execute(pool)
    .await?;
    Ok(result.last_insert_rowid())
}

// 按 id 更新定时任务, 不存在返回 false
pub async fn update_schedule(
    pool: &SqlitePool,
    schedule_id: i64,
    name: &str,
    content: &str,
    work_dir: &str,
    cron_expr: &str,
    agent_id: i64,
    enabled: bool,
) -> Result<bool, sqlx::Error> {
    let now = chrono::Local::now().to_rfc3339();
    let result = sqlx::query(
        "UPDATE t_schedule SET name = ?, content = ?, work_dir = ?, cron_expr = ?, agent_id = ?, enabled = ?, update_time = ? WHERE id = ?",
    )
    .bind(name)
    .bind(content)
    .bind(work_dir)
    .bind(cron_expr)
    .bind(agent_id)
    .bind(enabled)
    .bind(&now)
    .bind(schedule_id)
    .execute(pool)
    .await?;
    Ok(result.rows_affected() > 0)
}

// 按 id 删除定时任务, 关联执行对话一并删除(消息由外键 ON DELETE CASCADE 级联), 不存在返回 false
pub async fn delete_schedule(pool: &SqlitePool, schedule_id: i64) -> Result<bool, sqlx::Error> {
    // 对话与定时任务在同一事务中删除, 避免半截状态
    let mut tx = pool.begin().await?;
    sqlx::query("DELETE FROM t_conversation WHERE schedule_id = ?")
        .bind(schedule_id)
        .execute(&mut *tx)
        .await?;
    let result = sqlx::query("DELETE FROM t_schedule WHERE id = ?")
        .bind(schedule_id)
        .execute(&mut *tx)
        .await?;
    tx.commit().await?;
    Ok(result.rows_affected() > 0)
}
