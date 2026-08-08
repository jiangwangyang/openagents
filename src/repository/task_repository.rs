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

    let mut conv_results = Vec::with_capacity(conversations.len());
    for conv in conversations {
        // 查询该对话的消息，按 id 升序
        let messages = sqlx::query_as::<_, MessageEntity>(
            "SELECT id, conversation_id, role, content, stop_reason, cache_read_input_tokens, input_tokens, output_tokens, time FROM t_message WHERE conversation_id = ? ORDER BY id",
        )
        .bind(conv.id)
        .fetch_all(pool)
        .await?;

        // 查询关联的 Agent(含 ModelProvider)
        let agent = match conv.agent_id {
            Some(aid) => {
                let agent_row = sqlx::query_as::<_, AgentEntity>(
                    "SELECT id, name, description, prompt, model_provider_id, model, thinking, create_time, update_time FROM t_agent WHERE id = ?",
                )
                .bind(aid)
                .fetch_optional(pool)
                .await?;
                match agent_row {
                    Some(a) => {
                        let provider = sqlx::query_as::<_, ModelProviderEntity>(
                            "SELECT id, name, type as provider_type, base_url, api_key, create_time, update_time FROM t_model_provider WHERE id = ?",
                        )
                        .bind(a.model_provider_id)
                        .fetch_optional(pool)
                        .await?;
                        Some(AgentWithProvider {
                            id: a.id,
                            name: a.name,
                            description: a.description,
                            prompt: a.prompt,
                            model_provider_id: a.model_provider_id,
                            model: a.model,
                            thinking: a.thinking,
                            create_time: a.create_time,
                            update_time: a.update_time,
                            model_provider: provider,
                        })
                    }
                    None => None,
                }
            }
            None => None,
        };

        conv_results.push(ConversationWithMessagesAndAgent {
            id: conv.id,
            task_id: conv.task_id,
            agent_id: conv.agent_id,
            title: conv.title,
            work_dir: conv.work_dir,
            system_prompt: conv.system_prompt,
            create_time: conv.create_time,
            update_time: conv.update_time,
            messages,
            agent,
        });
    }

    Ok(Some(TaskWithConversations {
        id: task.id,
        title: task.title,
        content: task.content,
        agent_ids: task.agent_ids,
        work_dir: task.work_dir,
        create_time: task.create_time,
        update_time: task.update_time,
        conversations: conv_results,
    }))
}

// 新增任务，返回自增 id
pub async fn add_task(pool: &SqlitePool, title: &str, content: &str, agent_ids: &serde_json::Value, work_dir: &str) -> Result<i64, sqlx::Error> {
    let now = chrono::Local::now().to_rfc3339();
    let result = sqlx::query(
        "INSERT INTO t_task (title, content, agent_ids, work_dir, create_time, update_time) VALUES (?, ?, ?, ?, ?, ?)",
    )
    .bind(title)
    .bind(content)
    .bind(agent_ids)
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
