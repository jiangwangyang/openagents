// 对话 CRUD
use sqlx::SqlitePool;

use super::entity::{ConversationEntity, ConversationWithMessages, MessageEntity};

// 查询全部对话，按更新时间倒序
pub async fn get_conversations(pool: &SqlitePool) -> Result<Vec<ConversationEntity>, sqlx::Error> {
    sqlx::query_as::<_, ConversationEntity>(
        "SELECT id, task_id, agent_id, title, work_dir, system_prompt, create_time, update_time FROM t_conversation ORDER BY update_time DESC",
    )
    .fetch_all(pool)
    .await
}

// 按 id 查询对话，含消息列表(按 id 升序)
pub async fn get_conversation(pool: &SqlitePool, conversation_id: i64) -> Result<Option<ConversationWithMessages>, sqlx::Error> {
    let conv = sqlx::query_as::<_, ConversationEntity>(
        "SELECT id, task_id, agent_id, title, work_dir, system_prompt, create_time, update_time FROM t_conversation WHERE id = ?",
    )
    .bind(conversation_id)
    .fetch_optional(pool)
    .await?;

    match conv {
        Some(c) => {
            let messages = sqlx::query_as::<_, MessageEntity>(
                "SELECT id, conversation_id, role, content, stop_reason, cache_read_input_tokens, input_tokens, output_tokens, time FROM t_message WHERE conversation_id = ? ORDER BY id",
            )
            .bind(conversation_id)
            .fetch_all(pool)
            .await?;
            Ok(Some(ConversationWithMessages {
                id: c.id,
                task_id: c.task_id,
                agent_id: c.agent_id,
                title: c.title,
                work_dir: c.work_dir,
                system_prompt: c.system_prompt,
                create_time: c.create_time,
                update_time: c.update_time,
                messages,
            }))
        }
        None => Ok(None),
    }
}

// 新建对话，返回自增 id
pub async fn add_conversation(pool: &SqlitePool, title: &str, work_dir: &str, system_prompt: &str, task_id: Option<i64>, agent_id: Option<i64>) -> Result<i64, sqlx::Error> {
    let now = chrono::Local::now().to_rfc3339();
    let result = sqlx::query(
        "INSERT INTO t_conversation (title, work_dir, system_prompt, task_id, agent_id, create_time, update_time) VALUES (?, ?, ?, ?, ?, ?, ?)",
    )
    .bind(title)
    .bind(work_dir)
    .bind(system_prompt)
    .bind(task_id)
    .bind(agent_id)
    .bind(&now)
    .bind(&now)
    .execute(pool)
    .await?;
    Ok(result.last_insert_rowid())
}

// 批量追加对话消息，并原子刷新对话的更新时间
pub async fn add_conversation_messages(pool: &SqlitePool, conversation_id: i64, messages: &[(String, serde_json::Value, String, i64, i64, i64, String)]) -> Result<(), sqlx::Error> {
    let mut tx = pool.begin().await?;
    for (role, content, stop_reason, cache_read_input_tokens, input_tokens, output_tokens, time) in messages {
        sqlx::query(
            "INSERT INTO t_message (conversation_id, role, content, stop_reason, cache_read_input_tokens, input_tokens, output_tokens, time) VALUES (?, ?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(conversation_id)
        .bind(role)
        .bind(content)
        .bind(stop_reason)
        .bind(cache_read_input_tokens)
        .bind(input_tokens)
        .bind(output_tokens)
        .bind(time)
        .execute(&mut *tx)
        .await?;
    }
    let now = chrono::Local::now().to_rfc3339();
    sqlx::query("UPDATE t_conversation SET update_time = ? WHERE id = ?")
        .bind(&now)
        .bind(conversation_id)
        .execute(&mut *tx)
        .await?;
    tx.commit().await?;
    Ok(())
}

// 删除对话，消息由数据库外键 ON DELETE CASCADE 级联删除
pub async fn delete_conversation(pool: &SqlitePool, conversation_id: i64) -> Result<bool, sqlx::Error> {
    let result = sqlx::query("DELETE FROM t_conversation WHERE id = ?")
        .bind(conversation_id)
        .execute(pool)
        .await?;
    Ok(result.rows_affected() > 0)
}
