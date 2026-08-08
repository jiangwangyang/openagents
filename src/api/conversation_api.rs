// 对话 API
use axum::extract::{Path, State};
use axum::Json;
use serde::Deserialize;

use crate::error::AppError;
use crate::repository::conversation_repository;
use crate::repository::entity::ConversationEntity;
use crate::state::AppState;

// 对话列表接口，按更新时间倒序返回独立对话（不含任务中的阶段对话）
pub async fn get_conversations(State(state): State<AppState>) -> Result<Json<Vec<ConversationEntity>>, AppError> {
    let conversations = conversation_repository::get_conversations(&state.db).await?;
    let filtered: Vec<_> = conversations.into_iter().filter(|c| c.task_id.is_none()).collect();
    Ok(Json(filtered))
}

// 删除对话接口，消息由数据库外键 ON DELETE CASCADE 级联删除，对话不存在时返回 404
pub async fn delete_conversation(State(state): State<AppState>, Path(conversation_id): Path<i64>) -> Result<(), AppError> {
    let deleted = conversation_repository::delete_conversation(&state.db, conversation_id).await?;
    if !deleted {
        return Err(AppError::NotFound("Conversation not found".to_string()));
    }
    Ok(())
}

// 追加用户消息请求体
#[derive(Debug, Deserialize)]
pub struct AddMessageRequest {
    pub content: String,
}

// 追加用户消息接口，向指定对话追加一条 role 为 user 的消息并刷新对话更新时间，对话不存在返回 404
pub async fn add_conversation_message(State(state): State<AppState>, Path(conversation_id): Path<i64>, Json(req): Json<AddMessageRequest>) -> Result<(), AppError> {
    let conversation = conversation_repository::get_conversation(&state.db, conversation_id).await?;
    if conversation.is_none() {
        return Err(AppError::NotFound("Conversation not found".to_string()));
    }
    let now = chrono::Local::now().to_rfc3339();
    let messages = vec![("user".to_string(), serde_json::Value::String(req.content), "".to_string(), 0i64, 0i64, 0i64, now)];
    conversation_repository::add_conversation_messages(&state.db, conversation_id, &messages).await?;
    Ok(())
}
