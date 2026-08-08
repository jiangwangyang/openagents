// 对话 API: 列表 / 删除 / 追加消息 / 启动对话 / 历史回放 / SSE 流式订阅
use axum::extract::{Path, State};
use axum::response::sse::{Event, Sse};
use axum::Json;
use serde::Deserialize;
use std::convert::Infallible;
use std::sync::Arc;
use tokio::sync::RwLock;

use crate::error::AppError;
use crate::repository::{agent_repository, conversation_repository};
use crate::repository::entity::ConversationEntity;
use crate::service::conversation_service;
use crate::state::{AppState, ConversationState};

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

// 创建对话 work 请求体
#[derive(Debug, Deserialize)]
pub struct CreateWorkRequest {
    pub task_content: String,
    pub work_dir: String,
    pub model_provider_id: Option<i64>,
    pub model: Option<String>,
    pub thinking: Option<bool>,
    pub agent_id: Option<i64>,
}

// POST /conversation/start 创建对话并启动 work
pub async fn create_conversation_work(State(state): State<AppState>, Json(req): Json<CreateWorkRequest>) -> Result<Json<i64>, AppError> {
    let mut system_prompt = String::new();
    let mut model_provider_id = req.model_provider_id;
    let mut model = req.model;
    let mut thinking = req.thinking;

    if let Some(agent_id) = req.agent_id {
        // 指定 agent 时使用其 prompt 作为 system_prompt
        let agent = agent_repository::get_agent(&state.db, agent_id).await?;
        let agent = match agent {
            Some(a) => a,
            None => return Err(AppError::NotFound("Agent not found".to_string())),
        };
        system_prompt = agent.prompt.clone();

        // 模型配置必须全部为 None 或全部与 agent 配置一致，否则拒绝请求
        let any_user_config = model_provider_id.is_some() || model.is_some() || thinking.is_some();
        if any_user_config {
            let user_model_provider_id = model_provider_id;
            let user_model = model.as_deref().unwrap_or("");
            let user_thinking = thinking.unwrap_or(false);
            if user_model_provider_id != Some(agent.model_provider_id) || user_model != agent.model || user_thinking != agent.thinking {
                return Err(AppError::BadRequest("Model config must be all None or all consistent with agent config".to_string()));
            }
        }
        // 使用 agent 的模型配置，忽略用户传入的参数
        model_provider_id = Some(agent.model_provider_id);
        model = Some(agent.model.clone());
        thinking = Some(agent.thinking);
    } else {
        // 未指定 agent 时模型配置必填
        if model_provider_id.is_none() || model.is_none() || thinking.is_none() {
            return Err(AppError::BadRequest("Model config is required when agent_id is not provided".to_string()));
        }
        // 读取 AGENTS.md，按优先级取第一个存在的文件
        let home = std::env::var("HOME").or_else(|_| std::env::var("USERPROFILE")).unwrap_or_else(|_| ".".to_string());
        let agents_files = [
            std::path::PathBuf::from(&req.work_dir).join("AGENTS.md"),
            std::path::PathBuf::from(&home).join(".openagents").join("AGENTS.md"),
            std::path::PathBuf::from(&home).join(".agents").join("AGENTS.md"),
        ];
        for agents_file in &agents_files {
            if agents_file.exists() && agents_file.is_file() {
                if let Ok(content) = tokio::fs::read_to_string(agents_file).await {
                    system_prompt = content;
                    break;
                }
            }
        }
    }

    // 先创建对话，再根据对话ID开始任务
    let conversation_id = conversation_repository::add_conversation(&state.db, &req.task_content, &req.work_dir, &system_prompt, None, req.agent_id).await?;
    if !conversation_service::start_conversation(conversation_id, req.task_content.clone(), model_provider_id.unwrap(), model.unwrap(), thinking.unwrap(), &state.conversations, &state.db).await {
        // 启动失败时清理刚创建的对话，避免产生孤儿数据
        let _ = conversation_repository::delete_conversation(&state.db, conversation_id).await;
        return Err(AppError::Conflict("Work already running".to_string()));
    }
    Ok(Json(conversation_id))
}

// 启动历史对话 work 请求体
#[derive(Debug, Deserialize)]
pub struct StartWorkRequest {
    pub task_content: String,
    pub model_provider_id: i64,
    pub model: String,
    pub thinking: bool,
}

// POST /conversation/{conversation_id}/start 启动历史回放
pub async fn start_conversation_work(State(state): State<AppState>, Path(conversation_id): Path<i64>, Json(req): Json<StartWorkRequest>) -> Result<(), AppError> {
    if !conversation_service::start_conversation(conversation_id, req.task_content, req.model_provider_id, req.model, req.thinking, &state.conversations, &state.db).await {
        return Err(AppError::Conflict("Work already running".to_string()));
    }
    Ok(())
}

// GET /conversation/{conversation_id}/stream SSE 流式订阅
pub async fn stream_conversation_work(State(state): State<AppState>, Path(conversation_id): Path<i64>) -> Result<Sse<impl futures_util::Stream<Item = Result<Event, Infallible>>>, AppError> {
    let conversation = conversation_repository::get_conversation(&state.db, conversation_id).await?;
    if conversation.is_none() {
        return Err(AppError::NotFound("Conversation not found".to_string()));
    }

    // 查询对话状态
    let conversation_state = conversation_service::get_conversation_state(conversation_id, &state.conversations);
    if conversation_state.is_none() {
        // 如果没有对话启动一个查询不执行业务
        conversation_service::start_conversation_query(conversation_id, &state.conversations, &state.db).await;
    }
    let conversation_state = conversation_service::get_conversation_state(conversation_id, &state.conversations)
        .ok_or_else(|| AppError::Internal(anyhow::anyhow!("Failed to get conversation state")))?;

    // 构造 SSE 流
    let stream = create_sse_stream(conversation_state);
    Ok(Sse::new(stream))
}

// 创建 SSE 流: 先回放历史 chunks，再实时跟随新数据
fn create_sse_stream(conversation_state: Arc<RwLock<ConversationState>>) -> impl futures_util::Stream<Item = Result<Event, Infallible>> {
    let init_state = (0usize, None::<tokio::sync::watch::Receiver<u64>>);
    futures_util::stream::unfold((conversation_state, init_state), |(conversation_state, (mut index, mut rx))| async move {
        loop {
            // 读锁获取 chunks 快照，多个 SSE 读者可并发
            let (chunks_len, done) = {
                let s = conversation_state.read().await;
                (s.chunks.len(), s.done)
            };

            // 回放/输出新 chunks
            if index < chunks_len {
                let data = {
                    let s = conversation_state.read().await;
                    serde_json::to_string(&s.chunks[index]).unwrap_or_default()
                };
                index += 1;
                return Some((Ok(Event::default().data(data)), (conversation_state, (index, rx))));
            }

            if done {
                return None;
            }

            // 初始化 watch receiver
            if rx.is_none() {
                let s = conversation_state.read().await;
                rx = Some(s.notify.subscribe());
            }

            // 异步等待新数据通知
            let receiver = rx.as_mut().unwrap();
            if receiver.changed().await.is_err() {
                return None;
            }
        }
    })
}
