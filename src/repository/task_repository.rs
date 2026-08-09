// 任务 CRUD
use sqlx::SqlitePool;

use super::entity::{
    AgentEntity, AgentWithProvider, ConversationEntity, ConversationWithMessagesAndAgent,
    MessageEntity, ModelProviderEntity, TaskEntity, TaskWithConversations,
};

// 查询全部任务，按 id 升序
pub async fn list_tasks(pool: &SqlitePool) -> Result<Vec<TaskEntity>, sqlx::Error> {
    sqlx::query_as::<_, TaskEntity>(
        "SELECT id, title, content, agent_ids, work_dir, create_time, update_time FROM t_task ORDER BY id",
    )
    .fetch_all(pool)
    .await
}

// 按 id 查询任务基本字段
pub async fn get_task_entity(pool: &SqlitePool, task_id: i64) -> Result<Option<TaskEntity>, sqlx::Error> {
    sqlx::query_as::<_, TaskEntity>(
        "SELECT id, title, content, agent_ids, work_dir, create_time, update_time FROM t_task WHERE id = ?",
    )
    .bind(task_id)
    .fetch_optional(pool)
    .await
}

// 按 id 查询任务，含阶段对话(含消息与执行 Agent)
pub async fn get_task(pool: &SqlitePool, task_id: i64) -> Result<Option<TaskWithConversations>, sqlx::Error> {
    let task = sqlx::query_as::<_, TaskEntity>(
        "SELECT id, title, content, agent_ids, work_dir, create_time, update_time FROM t_task WHERE id = ?",
    )
    .bind(task_id)
    .fetch_optional(pool)
    .await?;

    let task = match task {
        Some(t) => t,
        None => return Ok(None),
    };

    // 查询该任务的所有阶段对话，按 id 升序
    let conversations = sqlx::query_as::<_, ConversationEntity>(
        "SELECT id, task_id, agent_id, title, work_dir, system_prompt, create_time, update_time FROM t_conversation WHERE task_id = ? ORDER BY id",
    )
    .bind(task_id)
    .fetch_all(pool)
    .await?;

    if conversations.is_empty() {
        return Ok(Some(TaskWithConversations {
            task,
            conversations: Vec::new(),
        }));
    }

    // 批量查询全部阶段对话的消息(一次 IN 查询 + 内存分组,避免 N+1)，按 id 升序
    let placeholders = vec!["?"; conversations.len()].join(",");
    let messages_sql = format!(
        "SELECT id, conversation_id, role, content, stop_reason, cache_read_input_tokens, input_tokens, output_tokens, time FROM t_message WHERE conversation_id IN ({}) ORDER BY id",
        placeholders
    );
    let mut messages_query = sqlx::query_as::<_, MessageEntity>(&messages_sql);
    for conv in &conversations {
        messages_query = messages_query.bind(conv.id);
    }
    let all_messages = messages_query.fetch_all(pool).await?;
    let mut message_map: std::collections::HashMap<i64, Vec<MessageEntity>> = std::collections::HashMap::new();
    for message in all_messages {
        message_map.entry(message.conversation_id).or_default().push(message);
    }

    // 按需查询阶段对话关联的 Agent(含 ModelProvider),避免全量加载
    let agent_ids: Vec<i64> = conversations.iter().filter_map(|c| c.agent_id).collect();
    let mut agent_map: std::collections::HashMap<i64, AgentWithProvider> = std::collections::HashMap::new();
    if !agent_ids.is_empty() {
        let placeholders = vec!["?"; agent_ids.len()].join(",");
        let agents_sql = format!(
            "SELECT id, name, description, prompt, model_provider_id, model, thinking, create_time, update_time FROM t_agent WHERE id IN ({})",
            placeholders
        );
        let mut agents_query = sqlx::query_as::<_, AgentEntity>(&agents_sql);
        for id in &agent_ids {
            agents_query = agents_query.bind(id);
        }
        let agents = agents_query.fetch_all(pool).await?;

        // 按需查询上述 Agent 关联的 ModelProvider
        let provider_ids: Vec<i64> = agents.iter().map(|a| a.model_provider_id).collect();
        let placeholders = vec!["?"; provider_ids.len()].join(",");
        let providers_sql = format!(
            "SELECT id, name, protocol_type, base_url, api_key, create_time, update_time FROM t_model_provider WHERE id IN ({})",
            placeholders
        );
        let mut providers_query = sqlx::query_as::<_, ModelProviderEntity>(&providers_sql);
        for id in &provider_ids {
            providers_query = providers_query.bind(id);
        }
        let providers = providers_query.fetch_all(pool).await?;
        let provider_map: std::collections::HashMap<i64, ModelProviderEntity> = providers.into_iter().map(|p| (p.id, p)).collect();

        agent_map = agents
            .into_iter()
            .map(|agent| {
                let provider = provider_map.get(&agent.model_provider_id).cloned();
                (agent.id, AgentWithProvider {
                    agent,
                    model_provider: provider,
                })
            })
            .collect();
    }

    let conv_results = conversations
        .into_iter()
        .map(|conv| {
            let messages = message_map.remove(&conv.id).unwrap_or_default();
            let agent = conv.agent_id.and_then(|aid| agent_map.get(&aid).cloned());
            ConversationWithMessagesAndAgent {
                conversation: conv,
                messages,
                agent,
            }
        })
        .collect();

    Ok(Some(TaskWithConversations {
        task,
        conversations: conv_results,
    }))
}

// 新增任务，返回自增 id
pub async fn add_task(pool: &SqlitePool, title: &str, content: &str, agent_ids: &[i64], work_dir: &str) -> Result<i64, sqlx::Error> {
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

// 按 id 删除任务，不存在返回 false
pub async fn delete_task(pool: &SqlitePool, task_id: i64) -> Result<bool, sqlx::Error> {
    let result = sqlx::query("DELETE FROM t_task WHERE id = ?")
        .bind(task_id)
        .execute(pool)
        .await?;
    Ok(result.rows_affected() > 0)
}
