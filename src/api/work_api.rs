// Work API: 启动 work / 历史回放 / SSE 流式订阅
use axum::extract::{Path, State};
use axum::response::sse::{Event, Sse};
use axum::Json;
use serde::Deserialize;
use std::convert::Infallible;
use std::sync::Arc;
use tokio::sync::RwLock;

use crate::error::AppError;
use crate::repository::{agent_repository, conversation_repository};
use crate::service::work_service;
use crate::state::{AppState, WorkState};

// 创建 work 请求体
#[derive(Debug, Deserialize)]
pub struct CreateWorkRequest {
    pub task_content: String,
    pub work_dir: String,
    pub model_provider_id: Option<i64>,
    pub model: Option<String>,
    pub thinking: Option<bool>,
    pub agent_id: Option<i64>,
}

// POST /work/start 创建 work
pub async fn create_work(State(state): State<AppState>, Json(req): Json<CreateWorkRequest>) -> Result<Json<i64>, AppError> {
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
    if !work_service::start_work(conversation_id, req.task_content.clone(), model_provider_id.unwrap(), model.unwrap(), thinking.unwrap(), &state.works, &state.db).await {
        // 启动失败时清理刚创建的对话，避免产生孤儿数据
        let _ = conversation_repository::delete_conversation(&state.db, conversation_id).await;
        return Err(AppError::Conflict("Work already running".to_string()));
    }
    Ok(Json(conversation_id))
}

// 启动历史 work 请求体
#[derive(Debug, Deserialize)]
pub struct StartWorkRequest {
    pub task_content: String,
    pub model_provider_id: i64,
    pub model: String,
    pub thinking: bool,
}

// POST /work/{conversation_id}/start 启动历史回放
pub async fn start_work(State(state): State<AppState>, Path(conversation_id): Path<i64>, Json(req): Json<StartWorkRequest>) -> Result<(), AppError> {
    if !work_service::start_work(conversation_id, req.task_content, req.model_provider_id, req.model, req.thinking, &state.works, &state.db).await {
        return Err(AppError::Conflict("Work already running".to_string()));
    }
    Ok(())
}

// GET /work/{conversation_id}/stream SSE 流式订阅
pub async fn stream_work(State(state): State<AppState>, Path(conversation_id): Path<i64>) -> Result<Sse<impl futures_util::Stream<Item = Result<Event, Infallible>>>, AppError> {
    let conversation = conversation_repository::get_conversation(&state.db, conversation_id).await?;
    if conversation.is_none() {
        return Err(AppError::NotFound("Work not found".to_string()));
    }

    // 查询 work_state
    let work_state = work_service::get_work_state(conversation_id, &state.works);
    if work_state.is_none() {
        // 如果没有 work 启动一个查询 work 不执行业务
        work_service::start_query(conversation_id, &state.works, &state.db).await;
    }
    let work_state = work_service::get_work_state(conversation_id, &state.works)
        .ok_or_else(|| AppError::Internal(anyhow::anyhow!("Failed to get work state")))?;

    // 构造 SSE 流
    let stream = create_sse_stream(work_state);
    Ok(Sse::new(stream))
}

// 创建 SSE 流: 先回放历史 chunks，再实时跟随新数据
fn create_sse_stream(work_state: Arc<RwLock<WorkState>>) -> impl futures_util::Stream<Item = Result<Event, Infallible>> {
    let init_state = (0usize, None::<tokio::sync::watch::Receiver<u64>>);
    futures_util::stream::unfold((work_state, init_state), |(work_state, (mut index, mut rx))| async move {
        loop {
            // 读锁获取 chunks 快照，多个 SSE 读者可并发
            let (chunks_len, done) = {
                let s = work_state.read().await;
                (s.chunks.len(), s.done)
            };

            // 回放/输出新 chunks
            if index < chunks_len {
                let data = {
                    let s = work_state.read().await;
                    serde_json::to_string(&s.chunks[index]).unwrap_or_default()
                };
                index += 1;
                return Some((Ok(Event::default().data(data)), (work_state, (index, rx))));
            }

            if done {
                return None;
            }

            // 初始化 watch receiver
            if rx.is_none() {
                let s = work_state.read().await;
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
