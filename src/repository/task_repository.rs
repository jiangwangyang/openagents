// 任务 CRUD
use sqlx::SqlitePool;

use super::entity::TaskEntity;

// 查询全部任务, 按 id 升序
pub async fn list_tasks(pool: &SqlitePool) -> Result<Vec<TaskEntity>, sqlx::Error> {
    sqlx::query_as::<_, TaskEntity>(
        "SELECT id, title, content, agent_ids, work_dir, create_time, update_time FROM t_task ORDER BY id",
    )
        .fetch_all(pool)
        .await
}

// 按 id 查询任务基本字段
pub async fn get_task(pool: &SqlitePool, task_id: i64) -> Result<Option<TaskEntity>, sqlx::Error> {
    sqlx::query_as::<_, TaskEntity>(
        "SELECT id, title, content, agent_ids, work_dir, create_time, update_time FROM t_task WHERE id = ?",
    )
        .bind(task_id)
        .fetch_optional(pool)
        .await
}

// 新增任务, 返回自增 id
pub async fn add_task(
    pool: &SqlitePool,
    title: &str,
    content: &str,
    agent_ids: &[i64],
    work_dir: &str,
) -> Result<i64, sqlx::Error> {
    let now = chrono::Local::now().to_rfc3339();
    let result = sqlx::query(
        "INSERT INTO t_task (title, content, agent_ids, work_dir, create_time, update_time) VALUES (?, ?, ?, ?, ?, ?)",
    )
        .bind(title)
        .bind(content)
        .bind(sqlx::types::Json(agent_ids))
        .bind(work_dir)
        .bind(&now)
        .bind(&now)
        .execute(pool)
        .await?;
    Ok(result.last_insert_rowid())
}

// 按 id 更新任务, 不存在返回 false
pub async fn update_task(
    pool: &SqlitePool,
    task_id: i64,
    title: &str,
    content: &str,
    agent_ids: &[i64],
    work_dir: &str,
) -> Result<bool, sqlx::Error> {
    let now = chrono::Local::now().to_rfc3339();
    let result = sqlx::query(
        "UPDATE t_task SET title = ?, content = ?, agent_ids = ?, work_dir = ?, update_time = ? WHERE id = ?",
    )
        .bind(title)
        .bind(content)
        .bind(sqlx::types::Json(agent_ids))
        .bind(work_dir)
        .bind(&now)
        .bind(task_id)
        .execute(pool)
        .await?;
    Ok(result.rows_affected() > 0)
}

// 按 id 删除任务, 不存在返回 false
pub async fn delete_task(pool: &SqlitePool, task_id: i64) -> Result<bool, sqlx::Error> {
    let result = sqlx::query("DELETE FROM t_task WHERE id = ?")
        .bind(task_id)
        .execute(pool)
        .await?;
    Ok(result.rows_affected() > 0)
}
