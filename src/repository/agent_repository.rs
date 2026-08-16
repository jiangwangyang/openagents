// Agent CRUD
use sqlx::SqlitePool;

use super::entity::AgentEntity;

// 查询全部 Agent, 按 id 升序
pub async fn list_agents(pool: &SqlitePool) -> Result<Vec<AgentEntity>, sqlx::Error> {
    sqlx::query_as::<_, AgentEntity>(
        "SELECT id, name, description, prompt, model_provider_id, model, thinking, create_time, update_time FROM t_agent ORDER BY id",
    )
    .fetch_all(pool)
    .await
}

// 按 id 查询 Agent
pub async fn get_agent(
    pool: &SqlitePool,
    agent_id: i64,
) -> Result<Option<AgentEntity>, sqlx::Error> {
    sqlx::query_as::<_, AgentEntity>(
        "SELECT id, name, description, prompt, model_provider_id, model, thinking, create_time, update_time FROM t_agent WHERE id = ?",
    )
    .bind(agent_id)
    .fetch_optional(pool)
    .await
}

// 新增 Agent, 返回自增 id
pub async fn add_agent(
    pool: &SqlitePool,
    name: &str,
    description: &str,
    prompt: &str,
    model_provider_id: i64,
    model: &str,
    thinking: bool,
) -> Result<i64, sqlx::Error> {
    let now = chrono::Local::now().to_rfc3339();
    let result = sqlx::query(
        "INSERT INTO t_agent (name, description, prompt, model_provider_id, model, thinking, create_time, update_time) VALUES (?, ?, ?, ?, ?, ?, ?, ?)",
    )
    .bind(name)
    .bind(description)
    .bind(prompt)
    .bind(model_provider_id)
    .bind(model)
    .bind(thinking)
    .bind(&now)
    .bind(&now)
    .execute(pool)
    .await?;
    Ok(result.last_insert_rowid())
}

// 按 id 更新 Agent, id 不存在返回 false
pub async fn update_agent(
    pool: &SqlitePool,
    agent_id: i64,
    name: &str,
    description: &str,
    prompt: &str,
    model_provider_id: i64,
    model: &str,
    thinking: bool,
) -> Result<bool, sqlx::Error> {
    let now = chrono::Local::now().to_rfc3339();
    let result = sqlx::query(
        "UPDATE t_agent SET name = ?, description = ?, prompt = ?, model_provider_id = ?, model = ?, thinking = ?, update_time = ? WHERE id = ?",
    )
    .bind(name)
    .bind(description)
    .bind(prompt)
    .bind(model_provider_id)
    .bind(model)
    .bind(thinking)
    .bind(&now)
    .bind(agent_id)
    .execute(pool)
    .await?;
    Ok(result.rows_affected() > 0)
}

// 按 id 删除 Agent, 不存在或被对话/定时任务引用返回 false
pub async fn delete_agent(pool: &SqlitePool, agent_id: i64) -> Result<bool, sqlx::Error> {
    let referenced: Option<(i64,)> =
        sqlx::query_as("SELECT id FROM t_conversation WHERE agent_id = ? LIMIT 1")
            .bind(agent_id)
            .fetch_optional(pool)
            .await?;
    if referenced.is_some() {
        return Ok(false);
    }
    let referenced: Option<(i64,)> =
        sqlx::query_as("SELECT id FROM t_schedule WHERE agent_id = ? LIMIT 1")
            .bind(agent_id)
            .fetch_optional(pool)
            .await?;
    if referenced.is_some() {
        return Ok(false);
    }
    let result = sqlx::query("DELETE FROM t_agent WHERE id = ?")
        .bind(agent_id)
        .execute(pool)
        .await?;
    Ok(result.rows_affected() > 0)
}
