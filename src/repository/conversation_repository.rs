// 对话 CRUD
use sqlx::SqlitePool;

use super::entity::{ConversationAgentRow, ConversationEntity, ConversationHistorySummary, ConversationWithMessages, LatestConversationState, MessageEntity};
use super::now_rfc3339;

// 查询全部独立对话(不含任务阶段对话与定时任务对话), 按更新时间倒序
pub async fn list_conversations(pool: &SqlitePool) -> Result<Vec<ConversationEntity>, sqlx::Error> {
    sqlx::query_as::<_, ConversationEntity>("SELECT id, task_id, schedule_id, agent_id, title, work_dir, system_prompt, create_time, update_time FROM t_conversation WHERE task_id IS NULL AND schedule_id IS NULL ORDER BY update_time DESC").fetch_all(pool).await
}

// 按 schedule_id 查询定时任务的全部执行对话, 按 id 升序
pub async fn list_conversations_by_schedule_id(pool: &SqlitePool, schedule_id: i64) -> Result<Vec<ConversationEntity>, sqlx::Error> {
    sqlx::query_as::<_, ConversationEntity>("SELECT id, task_id, schedule_id, agent_id, title, work_dir, system_prompt, create_time, update_time FROM t_conversation WHERE schedule_id = ? ORDER BY id").bind(schedule_id).fetch_all(pool).await
}

// 按 task_id 查询任务的全部阶段对话, 按 id 升序
pub async fn list_conversations_by_task_id(pool: &SqlitePool, task_id: i64) -> Result<Vec<ConversationEntity>, sqlx::Error> {
    sqlx::query_as::<_, ConversationEntity>("SELECT id, task_id, schedule_id, agent_id, title, work_dir, system_prompt, create_time, update_time FROM t_conversation WHERE task_id = ? ORDER BY id").bind(task_id).fetch_all(pool).await
}

// 按对话 id 列表批量查询对话(LEFT JOIN 关联执行 Agent)及消息(两次查询 + 内存分组避免 N+1), 对话与消息均按 id 升序, ids 为空直接返回空
pub async fn list_conversation_with_messages_by_conversation_ids(pool: &SqlitePool, conversation_ids: &[i64]) -> Result<Vec<ConversationWithMessages>, sqlx::Error> {
    if conversation_ids.is_empty() {
        return Ok(Vec::new());
    }
    let placeholders = vec!["?"; conversation_ids.len()].join(", ");

    // 单条 SQL LEFT JOIN 关联执行 Agent
    let sql = format!("SELECT c.id, c.task_id, c.schedule_id, c.agent_id, c.title, c.work_dir, c.system_prompt, c.create_time, c.update_time, a.id AS agent_ref_id, a.name AS agent_name, a.description AS agent_description, a.prompt AS agent_prompt, a.model_provider_id AS agent_model_provider_id, a.model AS agent_model, a.thinking AS agent_thinking, a.create_time AS agent_create_time, a.update_time AS agent_update_time FROM t_conversation c LEFT JOIN t_agent a ON a.id = c.agent_id WHERE c.id IN ({}) ORDER BY c.id", placeholders);
    let mut query = sqlx::query_as::<_, ConversationAgentRow>(&sql);
    for id in conversation_ids {
        query = query.bind(id);
    }
    let rows = query.fetch_all(pool).await?;
    let mut conversations: Vec<ConversationWithMessages> = rows.into_iter().map(ConversationWithMessages::from).collect();

    // 消息列表(一对多子表)单独查询, 按 conversation_id 内存分组填充
    let sql = format!("SELECT id, conversation_id, content FROM t_message WHERE conversation_id IN ({}) ORDER BY id", placeholders);
    let mut query = sqlx::query_as::<_, MessageEntity>(&sql);
    for id in conversation_ids {
        query = query.bind(id);
    }
    let mut message_map: std::collections::HashMap<i64, Vec<MessageEntity>> = std::collections::HashMap::new();
    for message in query.fetch_all(pool).await? {
        message_map.entry(message.conversation_id).or_default().push(message);
    }
    for c in &mut conversations {
        c.messages = message_map.remove(&c.conversation.id).unwrap_or_default();
    }
    Ok(conversations)
}

// 按 id 查询对话, 单条 SQL LEFT JOIN 关联执行 Agent, 消息列表(一对多子表)单独查询, 按 id 升序
pub async fn get_conversation_with_messages(pool: &SqlitePool, conversation_id: i64) -> Result<Option<ConversationWithMessages>, sqlx::Error> {
    Ok(list_conversation_with_messages_by_conversation_ids(pool, &[conversation_id]).await?.into_iter().next())
}

// 新建对话, 返回自增 id
pub async fn add_conversation(pool: &SqlitePool, title: &str, work_dir: &str, system_prompt: &str, task_id: Option<i64>, agent_id: Option<i64>, schedule_id: Option<i64>) -> Result<i64, sqlx::Error> {
    let now = now_rfc3339();
    let result = sqlx::query("INSERT INTO t_conversation (title, work_dir, system_prompt, task_id, schedule_id, agent_id, create_time, update_time) VALUES (?, ?, ?, ?, ?, ?, ?, ?)").bind(title).bind(work_dir).bind(system_prompt).bind(task_id).bind(schedule_id).bind(agent_id).bind(&now).bind(&now).execute(pool).await?;
    Ok(result.last_insert_rowid())
}

// 批量追加对话消息(content 列存整条 pi 消息 JSON), 并原子刷新对话的更新时间
pub async fn add_conversation_messages(pool: &SqlitePool, conversation_id: i64, messages: &[serde_json::Value]) -> Result<(), sqlx::Error> {
    let mut tx = pool.begin().await?;
    for content in messages {
        sqlx::query("INSERT INTO t_message (conversation_id, content) VALUES (?, ?)").bind(conversation_id).bind(content).execute(&mut *tx).await?;
    }
    let now = now_rfc3339();
    sqlx::query("UPDATE t_conversation SET update_time = ? WHERE id = ?").bind(&now).bind(conversation_id).execute(&mut *tx).await?;
    tx.commit().await?;
    Ok(())
}

// 查询任务最新一条阶段对话的状态(id、执行 Agent、是否有消息)
pub async fn get_latest_task_conversation_state(pool: &SqlitePool, task_id: i64) -> Result<Option<LatestConversationState>, sqlx::Error> {
    sqlx::query_as::<_, LatestConversationState>("SELECT c.id, c.agent_id, EXISTS(SELECT 1 FROM t_message m WHERE m.conversation_id = c.id) AS has_messages FROM t_conversation c WHERE c.task_id = ? ORDER BY c.id DESC LIMIT 1").bind(task_id).fetch_optional(pool).await
}

// 查询任务各阶段对话的历史摘要(执行 Agent 名称与最后一条消息内容), 按对话 id 升序
pub async fn list_task_conversation_history(pool: &SqlitePool, task_id: i64) -> Result<Vec<ConversationHistorySummary>, sqlx::Error> {
    sqlx::query_as::<_, ConversationHistorySummary>("SELECT a.name AS agent_name, m.content AS last_content FROM t_conversation c LEFT JOIN t_agent a ON a.id = c.agent_id LEFT JOIN t_message m ON m.id = (SELECT MAX(id) FROM t_message WHERE conversation_id = c.id) WHERE c.task_id = ? ORDER BY c.id").bind(task_id).fetch_all(pool).await
}

// 删除对话, 消息由数据库外键 ON DELETE CASCADE 级联删除
pub async fn delete_conversation(pool: &SqlitePool, conversation_id: i64) -> Result<bool, sqlx::Error> {
    let result = sqlx::query("DELETE FROM t_conversation WHERE id = ?").bind(conversation_id).execute(pool).await?;
    Ok(result.rows_affected() > 0)
}
