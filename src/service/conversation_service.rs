// 对话服务: 启动对话、查询状态、发布 SSE chunk、后台 agent loop
use futures_util::{FutureExt, StreamExt};
use serde_json::{json, Value};
use std::sync::Arc;
use tokio::sync::RwLock;

use crate::ai;
use crate::ai::pi::types::{
    now_timestamp, AssistantContent, AssistantMessage, AssistantMessageEvent, Context, Message, TextContent, ToolCall,
    ToolResultMessage, UserContent, UserMessage, UserMessageContent,
};
use crate::repository::entity::NewMessageEntity;
use crate::repository::{conversation_repository, model_provider_repository};
use crate::state::{AppState, ConversationState};
use crate::service::tool::{self, ToolContext};

// 查询对话状态在内存中的保留时长(秒)
const QUERY_STATE_TTL_SECS: u64 = 300;

// 启动对话, 同一 conversation 不允许同时运行多个
pub async fn start_conversation(
    state: &AppState,
    conversation_id: i64,
    task_content: String,
    model_provider_id: i64,
    model: String,
    thinking: bool,
) -> bool {
    let (tx, _) = tokio::sync::watch::channel(0u64);
    let conv_state = Arc::new(RwLock::new(ConversationState {
        chunks: Vec::new(),
        done: false,
        notify: tx,
    }));
    // 防重入: entry 原子检查并替换, 持锁期间不 await, 状态锁被占用视为运行中
    match state.conversation_states.entry(conversation_id) {
        dashmap::mapref::entry::Entry::Occupied(mut entry) => {
            let finished = match entry.get().try_read() {
                Ok(s) => s.done,
                Err(_) => false,
            };
            if !finished {
                tracing::warn!("Conversation start rejected: conversation_id={} already running", conversation_id);
                return false;
            }
            entry.insert(conv_state);
        }
        dashmap::mapref::entry::Entry::Vacant(entry) => {
            entry.insert(conv_state);
        }
    }
    tracing::info!("Conversation started: conversation_id={} model={} thinking={}", conversation_id, model, thinking);
    tokio::spawn(run_conversation(state.clone(), conversation_id, task_content, model_provider_id, model, thinking));
    true
}

// 启动历史回放查询, 不执行模型调用
pub async fn start_conversation_query(
    state: &AppState,
    conversation_id: i64,
) -> bool {
    let (tx, _) = tokio::sync::watch::channel(0u64);
    let conv_state = Arc::new(RwLock::new(ConversationState {
        chunks: Vec::new(),
        done: false,
        notify: tx,
    }));
    // 防重入: entry 原子检查并替换, 持锁期间不 await, 状态锁被占用视为运行中
    match state.conversation_states.entry(conversation_id) {
        dashmap::mapref::entry::Entry::Occupied(mut entry) => {
            let finished = match entry.get().try_read() {
                Ok(s) => s.done,
                Err(_) => false,
            };
            if !finished {
                tracing::warn!("Conversation query rejected: conversation_id={} already running", conversation_id);
                return false;
            }
            entry.insert(conv_state);
        }
        dashmap::mapref::entry::Entry::Vacant(entry) => {
            entry.insert(conv_state);
        }
    }
    tracing::info!("Conversation query started: conversation_id={}", conversation_id);
    tokio::spawn(run_conversation(state.clone(), conversation_id, String::new(), 0, String::new(), false));
    true
}

// 查询对话状态
pub fn get_conversation_state(
    state: &AppState,
    conversation_id: i64,
) -> Option<Arc<RwLock<ConversationState>>> {
    state.conversation_states.get(&conversation_id).map(|r| r.clone())
}

// 发布 SSE chunk
pub async fn publish_chunk(
    state: &AppState,
    conversation_id: i64,
    msg_type: &str,
    text: &str,
    kwargs: Value,
) {
    if let Some(conv_state) = state.conversation_states.get(&conversation_id) {
        let mut s = conv_state.write().await;
        let mut chunk = json!({"type": msg_type, "text": text});
        if let Value::Object(map) = kwargs {
            for (k, v) in map {
                chunk[k] = v;
            }
        }
        s.chunks.push(chunk);
        let _ = s.notify.send(s.chunks.len() as u64);
    }
}

// 标记对话完成
pub async fn finish_conversation(state: &AppState, conversation_id: i64) {
    if let Some(conv_state) = state.conversation_states.get(&conversation_id) {
        let mut s = conv_state.write().await;
        s.done = true;
        let _ = s.notify.send(s.chunks.len() as u64);
    }
}

// 后台 agent loop
async fn run_conversation(
    state: AppState,
    conversation_id: i64,
    task_content: String,
    model_provider_id: i64,
    model: String,
    thinking: bool,
) {
    // 捕获 panic 兜底, 保证 finish_conversation 必执行, 避免对话被永久锁死
    let result = std::panic::AssertUnwindSafe(do_run_conversation(&state, conversation_id, &task_content, model_provider_id, &model, thinking))
        .catch_unwind()
        .await
        .unwrap_or_else(|e| {
            let msg = e.downcast_ref::<String>().cloned()
                .or_else(|| e.downcast_ref::<&str>().map(|s| s.to_string()))
                .unwrap_or_else(|| "unknown".to_string());
            Err(anyhow::anyhow!("conversation panicked: {}", msg))
        });
    if let Err(e) = result {
        publish_chunk(&state, conversation_id, "error", &format!("Conversation execution failed: {}", e), json!({})).await;
        tracing::error!("Conversation execution failed: conversation_id={} error={}", conversation_id, e);
    }
    finish_conversation(&state, conversation_id).await;
    // 执行任务存储的 chunk 太碎立即移除, 查询任务的状态则保留 5 分钟后再移除
    if task_content.is_empty() {
        // 仅当状态未被新启动的对话替换时才移除
        if let Some(conv_state) = state.conversation_states.get(&conversation_id).map(|r| r.clone()) {
            let conversations = state.conversation_states.clone();
            tokio::spawn(async move {
                tokio::time::sleep(std::time::Duration::from_secs(QUERY_STATE_TTL_SECS)).await;
                conversations.remove_if(&conversation_id, |_, current| Arc::ptr_eq(current, &conv_state));
            });
        }
    } else {
        state.conversation_states.remove(&conversation_id);
    }
}

// 实际对话逻辑
async fn do_run_conversation(
    state: &AppState,
    conversation_id: i64,
    task_content: &str,
    model_provider_id: i64,
    model: &str,
    thinking: bool,
) -> anyhow::Result<()> {
    // 查询历史消息
    let conversation = conversation_repository::get_conversation(&state.db, conversation_id)
        .await?
        .ok_or_else(|| anyhow::anyhow!("conversation not found"))?;

    // 流式数据开头发布系统提示词
    publish_chunk(state, conversation_id, "system", &conversation.conversation.system_prompt, json!({})).await;

    // 发布历史消息(content 列为 pi 消息协议 JSON)
    for msg in &conversation.messages {
        match serde_json::from_value::<Message>(msg.content.clone()) {
            Ok(Message::User(user)) => {
                let text = match &user.content {
                    UserMessageContent::Text(s) => s.clone(),
                    UserMessageContent::Blocks(blocks) => blocks
                        .iter()
                        .filter_map(|b| match b {
                            UserContent::Text(t) => Some(t.text.as_str()),
                            _ => None,
                        })
                        .collect::<Vec<_>>()
                        .join("\n"),
                };
                publish_chunk(state, conversation_id, "user", &text, json!({})).await;
            }
            Ok(Message::Assistant(assistant)) => {
                for block in &assistant.content {
                    match block {
                        AssistantContent::Thinking(t) => {
                            publish_chunk(state, conversation_id, "thinking", &t.thinking, json!({})).await;
                        }
                        AssistantContent::Text(t) => {
                            publish_chunk(state, conversation_id, "text", &t.text, json!({})).await;
                        }
                        AssistantContent::ToolCall(tc) => {
                            let input_str = serde_json::to_string(&tc.arguments).unwrap_or_default();
                            publish_chunk(state, conversation_id, "tool_use", &input_str, json!({"_id": tc.id, "name": tc.name})).await;
                        }
                    }
                }
                // 发布使用量消息(usage 在 pi 消息内, 仅 assistant 消息携带)
                publish_chunk(
                    state,
                    conversation_id,
                    "usage",
                    "",
                    json!({
                        "cache_read_input_tokens": assistant.usage.cache_read,
                        "input_tokens": assistant.usage.input,
                        "output_tokens": assistant.usage.output,
                    }),
                )
                    .await;
            }
            Ok(Message::ToolResult(tool_result)) => {
                let text = tool_result
                    .content
                    .iter()
                    .filter_map(|b| match b {
                        UserContent::Text(t) => Some(t.text.as_str()),
                        _ => None,
                    })
                    .collect::<Vec<_>>()
                    .join("\n");
                publish_chunk(state, conversation_id, "tool_result", &text, json!({"_id": tool_result.tool_call_id, "is_error": tool_result.is_error})).await;
            }
            Err(e) => {
                tracing::warn!("Skip unparsable history message: id={} error={}", msg.id, e);
            }
        }
    }
    if !task_content.is_empty() {
        publish_chunk(state, conversation_id, "user", task_content, json!({})).await;
    }

    // 没有任务直接结束
    if task_content.is_empty() {
        return Ok(());
    }

    // 模型调用数据
    let provider = model_provider_repository::get_model_provider(&state.db, model_provider_id)
        .await?
        .ok_or_else(|| anyhow::anyhow!("model provider not found"))?;
    let system_prompt = conversation.conversation.system_prompt.clone();
    let tools: Vec<crate::ai::pi::types::Tool> = tool::list_tools()
        .iter()
        .map(|t| crate::ai::pi::types::Tool {
            name: t.name.clone(),
            description: t.description.clone(),
            parameters: t.input_schema.clone(),
        })
        .collect();

    // 构造初始消息列表(content 列为 pi 消息协议 JSON, 无法解析的历史消息跳过)
    let mut messages: Vec<Message> = conversation
        .messages
        .iter()
        .filter_map(|msg| match serde_json::from_value::<Message>(msg.content.clone()) {
            Ok(m) => Some(m),
            Err(e) => {
                tracing::warn!("Skip unparsable history message: id={} error={}", msg.id, e);
                None
            }
        })
        .collect();
    let task_message = Message::User(UserMessage {
        content: UserMessageContent::Text(task_content.to_string()),
        timestamp: now_timestamp(),
    });
    messages.push(task_message.clone());
    // 本轮新增的消息(结束后统一持久化)
    let mut new_messages: Vec<Message> = vec![task_message];

    // 执行 agent loop
    loop {
        // 发送模型请求(按 provider 协议类型路由, 统一返回 pi 基准协议事件流)
        let context = Context {
            system_prompt: Some(system_prompt.clone()),
            messages: messages.clone(),
            tools: Some(tools.clone()),
        };
        let mut stream = ai::client::stream(&provider, model, thinking, 16000, &context)?;

        // pi 事件流: 每个事件携带完整 partial 消息, done/error 事件携带最终 assistant message
        let mut assistant_msg: Option<AssistantMessage> = None;
        while let Some(event) = stream.next().await {
            match event {
                AssistantMessageEvent::Start { .. } => {}
                AssistantMessageEvent::ThinkingStart { .. } => {
                    publish_chunk(state, conversation_id, "thinking", "", json!({})).await;
                }
                AssistantMessageEvent::ThinkingDelta { delta, .. } => {
                    publish_chunk(state, conversation_id, "delta", &delta, json!({})).await;
                }
                AssistantMessageEvent::ThinkingEnd { .. } => {}
                AssistantMessageEvent::TextStart { .. } => {
                    publish_chunk(state, conversation_id, "text", "", json!({})).await;
                }
                AssistantMessageEvent::TextDelta { delta, .. } => {
                    publish_chunk(state, conversation_id, "delta", &delta, json!({})).await;
                }
                AssistantMessageEvent::TextEnd { .. } => {}
                AssistantMessageEvent::ToolcallStart { content_index, partial } => {
                    if let Some(AssistantContent::ToolCall(tc)) = partial.content.get(content_index) {
                        publish_chunk(state, conversation_id, "tool_use", "", json!({"_id": tc.id, "name": tc.name})).await;
                    }
                }
                AssistantMessageEvent::ToolcallDelta { delta, .. } => {
                    publish_chunk(state, conversation_id, "delta", &delta, json!({})).await;
                }
                AssistantMessageEvent::ToolcallEnd { .. } => {}
                AssistantMessageEvent::Done { message, .. } => {
                    assistant_msg = Some(message);
                }
                AssistantMessageEvent::Error { error, .. } => {
                    // 错误 chunk 由外层 run_conversation 统一发布
                    let message = error.error_message.clone().unwrap_or_else(|| "unknown error".to_string());
                    return Err(anyhow::anyhow!("model stream error: {}", message));
                }
            }
        }

        let msg = assistant_msg.ok_or_else(|| anyhow::anyhow!("no assistant message received"))?;
        tracing::info!(
            "Model round completed: conversation_id={} stop_reason={:?} input_tokens={} output_tokens={} cache_read_input_tokens={}",
            conversation_id,
            msg.stop_reason,
            msg.usage.input,
            msg.usage.output,
            msg.usage.cache_read,
        );
        publish_chunk(
            state,
            conversation_id,
            "usage",
            "",
            json!({
                "cache_read_input_tokens": msg.usage.cache_read,
                "input_tokens": msg.usage.input,
                "output_tokens": msg.usage.output,
            }),
        )
            .await;
        let assistant_message = Message::Assistant(msg);
        messages.push(assistant_message.clone());
        new_messages.push(assistant_message);

        // 判断结束
        let tool_calls: Vec<ToolCall> = match messages.last() {
            Some(Message::Assistant(a)) => a
                .content
                .iter()
                .filter_map(|b| match b {
                    AssistantContent::ToolCall(tc) => Some(tc.clone()),
                    _ => None,
                })
                .collect(),
            _ => Vec::new(),
        };

        if tool_calls.is_empty() {
            // 本轮新增消息持久化(content 列存整条 pi 消息 JSON)
            let messages_to_save: Vec<NewMessageEntity> = new_messages
                .iter()
                .map(|m| NewMessageEntity { content: serde_json::to_value(m).unwrap_or_default() })
                .collect();
            conversation_repository::add_conversation_messages(&state.db, conversation_id, &messages_to_save).await?;
            tracing::info!("Conversation finished: conversation_id={}", conversation_id);
            return Ok(());
        }

        // 工具调用(每个 toolCall 对应一条独立的 toolResult 消息, 对齐 pi)
        for tool_call in &tool_calls {
            let ctx = ToolContext {
                db: state.db.clone(),
                work_dir: conversation.conversation.work_dir.clone(),
                task_id: conversation.conversation.task_id,
                skills: state.skills.clone(),
            };
            let (tool_content, is_error) = tool::execute_tool(&tool_call.name, &tool_call.arguments, &ctx).await;
            publish_chunk(state, conversation_id, "tool_result", &tool_content, json!({"_id": tool_call.id, "is_error": is_error})).await;
            let tool_result = Message::ToolResult(ToolResultMessage {
                tool_call_id: tool_call.id.clone(),
                tool_name: tool_call.name.clone(),
                content: vec![UserContent::Text(TextContent { text: tool_content, text_signature: None })],
                is_error,
                timestamp: now_timestamp(),
            });
            messages.push(tool_result.clone());
            new_messages.push(tool_result);
        }
    }
}
