// Agent CRUD
use sqlx::SqlitePool;

use super::entity::{AgentEntity, AgentProviderRow, AgentWithProvider};
use super::now_rfc3339;
use super::DeleteResult;

// 查询全部 Agent, 按 id 升序
pub async fn list_agents(pool: &SqlitePool) -> Result<Vec<AgentEntity>, sqlx::Error> {
    sqlx::query_as::<_, AgentEntity>("SELECT id, name, description, prompt, model_provider_id, model, thinking, create_time, update_time FROM t_agent ORDER BY id").fetch_all(pool).await
}

// 按 id 查询 Agent, 单条 SQL LEFT JOIN 关联模型提供商
pub async fn get_agent(pool: &SqlitePool, agent_id: i64) -> Result<Option<AgentWithProvider>, sqlx::Error> {
    let row = sqlx::query_as::<_, AgentProviderRow>("SELECT a.id, a.name, a.description, a.prompt, a.model_provider_id, a.model, a.thinking, a.create_time, a.update_time, p.id AS provider_id, p.name AS provider_name, p.protocol_type AS provider_protocol_type, p.base_url AS provider_base_url, p.api_key AS provider_api_key, p.create_time AS provider_create_time, p.update_time AS provider_update_time FROM t_agent a LEFT JOIN t_model_provider p ON p.id = a.model_provider_id WHERE a.id = ?").bind(agent_id).fetch_optional(pool).await?;
    Ok(row.map(AgentWithProvider::from))
}

// 新增 Agent, 返回自增 id
pub async fn add_agent(pool: &SqlitePool, name: &str, description: &str, prompt: &str, model_provider_id: i64, model: &str, thinking: bool) -> Result<i64, sqlx::Error> {
    let now = now_rfc3339();
    let result = sqlx::query("INSERT INTO t_agent (name, description, prompt, model_provider_id, model, thinking, create_time, update_time) VALUES (?, ?, ?, ?, ?, ?, ?, ?)").bind(name).bind(description).bind(prompt).bind(model_provider_id).bind(model).bind(thinking).bind(&now).bind(&now).execute(pool).await?;
    Ok(result.last_insert_rowid())
}

// 按 id 更新 Agent, id 不存在返回 false
pub async fn update_agent(pool: &SqlitePool, agent_id: i64, name: &str, description: &str, prompt: &str, model_provider_id: i64, model: &str, thinking: bool) -> Result<bool, sqlx::Error> {
    let now = now_rfc3339();
    let result = sqlx::query("UPDATE t_agent SET name = ?, description = ?, prompt = ?, model_provider_id = ?, model = ?, thinking = ?, update_time = ? WHERE id = ?").bind(name).bind(description).bind(prompt).bind(model_provider_id).bind(model).bind(thinking).bind(&now).bind(agent_id).execute(pool).await?;
    Ok(result.rows_affected() > 0)
}

// 按 id 删除 Agent, 不存在返回 NotFound, 被对话/定时任务引用返回 Referenced
pub async fn delete_agent(pool: &SqlitePool, agent_id: i64) -> Result<DeleteResult, sqlx::Error> {
    // 引用检查先于删除: t_conversation/t_schedule 的 agent_id 为 ON DELETE RESTRICT, 直接删被引用行会触发外键错误
    let referenced: Option<(i64,)> = sqlx::query_as("SELECT id FROM t_conversation WHERE agent_id = ? LIMIT 1").bind(agent_id).fetch_optional(pool).await?;
    if referenced.is_some() {
        return Ok(DeleteResult::Referenced);
    }
    let referenced: Option<(i64,)> = sqlx::query_as("SELECT id FROM t_schedule WHERE agent_id = ? LIMIT 1").bind(agent_id).fetch_optional(pool).await?;
    if referenced.is_some() {
        return Ok(DeleteResult::Referenced);
    }
    let result = sqlx::query("DELETE FROM t_agent WHERE id = ?").bind(agent_id).execute(pool).await?;
    Ok(if result.rows_affected() > 0 { DeleteResult::Deleted } else { DeleteResult::NotFound })
}
