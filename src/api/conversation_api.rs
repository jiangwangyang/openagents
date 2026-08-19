// 对话 API: 列表 / 删除 / 启动对话 / 历史回放 / SSE 流式订阅
use std::convert::Infallible;
use std::sync::Arc;

use axum::extract::{Path, State};
use axum::response::sse::{Event, Sse};
use axum::Json;
use serde::Deserialize;
use serde_json::json;
use tokio::sync::RwLock;

use crate::error::AppError;
use crate::repository::entity::ConversationEntity;
use crate::repository::{agent_repository, conversation_repository};
use crate::service::conversation_service;
use crate::state::{AppState, ConversationState};

// 对话列表接口, 按更新时间倒序返回独立对话(不含任务中的阶段对话)
pub async fn list_conversations(
    State(state): State<AppState>,
) -> Result<Json<Vec<ConversationEntity>>, AppError> {
    let conversations = conversation_repository::list_conversations(&state.db).await?;
    Ok(Json(conversations))
}

// 查询对话详情接口: 返回对话基本字段、消息列表、外键关联的执行 Agent 及运行状态, 对话不存在返回 404
pub async fn get_conversation(
    State(state): State<AppState>,
    Path(conversation_id): Path<i64>,
) -> Result<Json<serde_json::Value>, AppError> {
    let conversation = conversation_repository::get_conversation(&state.db, conversation_id)
        .await?
        .ok_or_else(|| AppError::NotFound("Conversation not found".to_string()))?;
    // 对话基本字段由实体序列化展开, messages 直接返回数据库字段(id/conversation_id/content), agent 为外键关联的执行 Agent, 追加 running 字段
    let mut result = serde_json::to_value(&conversation)?;
    result["running"] = json!(conversation_service::is_conversation_running(
        &state,
        conversation_id
    ));
    Ok(Json(result))
}

// 删除对话接口, 消息由数据库外键 ON DELETE CASCADE 级联删除, 对话不存在返回 404, 正在运行返回 409
pub async fn delete_conversation(
    State(state): State<AppState>,
    Path(conversation_id): Path<i64>,
) -> Result<(), AppError> {
    // 运行中的对话不允许删除, 避免后台任务存消息时外键失败
    if let Some(conv_state) = conversation_service::get_conversation_state(&state, conversation_id)
    {
        if !conv_state.read().await.done {
            return Err(AppError::Conflict("Conversation is running".to_string()));
        }
    }
    let deleted = conversation_repository::delete_conversation(&state.db, conversation_id).await?;
    if !deleted {
        return Err(AppError::NotFound("Conversation not found".to_string()));
    }
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

// 创建对话接口: 新建对话并启动执行, 返回自增 id
pub async fn create_conversation_work(
    State(state): State<AppState>,
    Json(req): Json<CreateWorkRequest>,
) -> Result<Json<i64>, AppError> {
    let mut system_prompt = String::new();
    let model_provider_id;
    let model;
    let thinking;

    if let Some(agent_id) = req.agent_id {
        // 指定 agent 时使用其 prompt 作为 system_prompt, 模型配置直接使用 agent 的配置, 忽略用户传入的参数
        let agent = agent_repository::get_agent(&state.db, agent_id).await?;
        let agent = match agent {
            Some(a) => a.agent,
            None => return Err(AppError::NotFound("Agent not found".to_string())),
        };
        system_prompt = agent.prompt.clone();
        model_provider_id = agent.model_provider_id;
        model = agent.model.clone();
        thinking = agent.thinking;
    } else {
        // 未指定 agent 时使用用户传入的模型配置, 模型配置必填
        match (req.model_provider_id, req.model, req.thinking) {
            (Some(p), Some(m), Some(t)) => {
                model_provider_id = p;
                model = m;
                thinking = t;
            }
            _ => {
                return Err(AppError::BadRequest(
                    "Model config is required when agent_id is not provided".to_string(),
                ))
            }
        }
        // 未指定 agent 时只从工作目录读取 AGENTS.md 作为系统提示词
        let agents_file = std::path::PathBuf::from(&req.work_dir).join("AGENTS.md");
        if agents_file.exists() && agents_file.is_file() {
            if let Ok(content) = tokio::fs::read_to_string(&agents_file).await {
                system_prompt = content;
            }
        }
    }

    // 先创建对话, 再根据对话ID开始任务
    let conversation_id = conversation_repository::add_conversation(
        &state.db,
        &req.task_content,
        &req.work_dir,
        &system_prompt,
        None,
        req.agent_id,
        None,
    )
    .await?;
    if !conversation_service::start_conversation(
        &state,
        conversation_id,
        req.task_content.clone(),
        model_provider_id,
        model,
        thinking,
        false,
    )
    .await
    {
        // 启动失败时清理刚创建的对话, 避免产生孤儿数据
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

// 启动历史对话执行接口, 已在运行返回 409
pub async fn start_conversation_work(
    State(state): State<AppState>,
    Path(conversation_id): Path<i64>,
    Json(req): Json<StartWorkRequest>,
) -> Result<(), AppError> {
    if !conversation_service::start_conversation(
        &state,
        conversation_id,
        req.task_content,
        req.model_provider_id,
        req.model,
        req.thinking,
        false,
    )
    .await
    {
        return Err(AppError::Conflict("Work already running".to_string()));
    }
    Ok(())
}

// 停止对话接口: 对话未在运行返回 409
pub async fn stop_conversation_work(
    State(state): State<AppState>,
    Path(conversation_id): Path<i64>,
) -> Result<(), AppError> {
    if !conversation_service::stop_conversation(&state, conversation_id).await {
        return Err(AppError::Conflict(
            "Conversation is not running".to_string(),
        ));
    }
    Ok(())
}

// SSE 流式订阅接口: 先回放历史 chunks, 再实时跟随新数据
pub async fn stream_conversation_work(
    State(state): State<AppState>,
    Path(conversation_id): Path<i64>,
) -> Result<Sse<impl futures_util::Stream<Item = Result<Event, Infallible>>>, AppError> {
    conversation_repository::get_conversation(&state.db, conversation_id)
        .await?
        .ok_or_else(|| AppError::NotFound("Conversation not found".to_string()))?;

    // 无内存状态说明对话未运行, 启动仅查询的回放会话
    if conversation_service::get_conversation_state(&state, conversation_id).is_none() {
        conversation_service::start_conversation(
            &state,
            conversation_id,
            String::new(),
            0,
            String::new(),
            false,
            true,
        )
        .await;
    }
    let conversation_state = conversation_service::get_conversation_state(&state, conversation_id)
        .ok_or_else(|| AppError::Internal(anyhow::anyhow!("Failed to get conversation state")))?;

    // 构造 SSE 流
    let stream = create_sse_stream(conversation_state);
    Ok(Sse::new(stream))
}

// 创建 SSE 流: 先回放历史 chunks, 再实时跟随新数据
fn create_sse_stream(
    conversation_state: Arc<RwLock<ConversationState>>,
) -> impl futures_util::Stream<Item = Result<Event, Infallible>> {
    let init_state = (0usize, None::<tokio::sync::watch::Receiver<u64>>);
    futures_util::stream::unfold(
        (conversation_state, init_state),
        |(conversation_state, (mut index, mut rx))| async move {
            loop {
                // 单次读锁取当前 chunk 数据与完成标记, 多个 SSE 读者可并发
                let (data, done) = {
                    let s = conversation_state.read().await;
                    let data = s
                        .chunks
                        .get(index)
                        .map(|chunk| serde_json::to_string(chunk).unwrap_or_default());
                    (data, s.done)
                };

                // 回放/输出新 chunks
                if let Some(data) = data {
                    index += 1;
                    return Some((
                        Ok(Event::default().data(data)),
                        (conversation_state, (index, rx)),
                    ));
                }

                if done {
                    return None;
                }

                // 初始化 watch receiver 后立即重查状态, 避免订阅发生在最后一次通知之后导致永久等待
                if rx.is_none() {
                    let s = conversation_state.read().await;
                    rx = Some(s.notify.subscribe());
                    continue;
                }

                // 异步等待新数据通知(rx 必已初始化)
                if rx.as_mut()?.changed().await.is_err() {
                    return None;
                }
            }
        },
    )
}
