// Agent CRUD
use sqlx::SqlitePool;

use super::entity::{AgentEntity, AgentWithProvider, ModelProviderEntity};

// 查询全部 Agent，按 id 升序，关联查询模型提供商
pub async fn list_agents(pool: &SqlitePool) -> Result<Vec<AgentWithProvider>, sqlx::Error> {
    let agents = sqlx::query_as::<_, AgentEntity>(
        "SELECT id, name, description, prompt, model_provider_id, model, thinking, create_time, update_time FROM t_agent ORDER BY id",
    )
    .fetch_all(pool)
    .await?;

    let mut result = Vec::with_capacity(agents.len());
    for agent in agents {
        let provider = sqlx::query_as::<_, ModelProviderEntity>(
            "SELECT id, name, protocol_type, base_url, api_key, create_time, update_time FROM t_model_provider WHERE id = ?",
        )
        .bind(agent.model_provider_id)
        .fetch_optional(pool)
        .await?;
        result.push(AgentWithProvider {
            id: agent.id,
            name: agent.name,
            description: agent.description,
            prompt: agent.prompt,
            model_provider_id: agent.model_provider_id,
            model: agent.model,
            thinking: agent.thinking,
            create_time: agent.create_time,
            update_time: agent.update_time,
            model_provider: provider,
        });
    }
    Ok(result)
}

// 按 id 查询 Agent，关联查询模型提供商
pub async fn get_agent(pool: &SqlitePool, agent_id: i64) -> Result<Option<AgentWithProvider>, sqlx::Error> {
    let agent = sqlx::query_as::<_, AgentEntity>(
        "SELECT id, name, description, prompt, model_provider_id, model, thinking, create_time, update_time FROM t_agent WHERE id = ?",
    )
    .bind(agent_id)
    .fetch_optional(pool)
    .await?;

    match agent {
        Some(agent) => {
            let provider = sqlx::query_as::<_, ModelProviderEntity>(
                "SELECT id, name, protocol_type, base_url, api_key, create_time, update_time FROM t_model_provider WHERE id = ?",
            )
            .bind(agent.model_provider_id)
            .fetch_optional(pool)
            .await?;
            Ok(Some(AgentWithProvider {
                id: agent.id,
                name: agent.name,
                description: agent.description,
                prompt: agent.prompt,
                model_provider_id: agent.model_provider_id,
                model: agent.model,
                thinking: agent.thinking,
                create_time: agent.create_time,
                update_time: agent.update_time,
                model_provider: provider,
            }))
        }
        None => Ok(None),
    }
}

// 新增 Agent，返回自增 id
pub async fn add_agent(pool: &SqlitePool, name: &str, description: &str, prompt: &str, model_provider_id: i64, model: &str, thinking: bool) -> Result<i64, sqlx::Error> {
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

// 按 id 更新 Agent，id 不存在返回 false
pub async fn update_agent(pool: &SqlitePool, agent_id: i64, name: &str, description: &str, prompt: &str, model_provider_id: i64, model: &str, thinking: bool) -> Result<bool, sqlx::Error> {
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

// 按 id 删除 Agent，不存在返回 false
pub async fn delete_agent(pool: &SqlitePool, agent_id: i64) -> Result<bool, sqlx::Error> {
    let result = sqlx::query("DELETE FROM t_agent WHERE id = ?")
        .bind(agent_id)
        .execute(pool)
        .await?;
    Ok(result.rows_affected() > 0)
}
