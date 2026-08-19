// 对话服务: 启动对话、查询状态、发布 SSE chunk、后台 agent loop
use std::sync::Arc;

use futures_util::{FutureExt, StreamExt};
use serde_json::{json, Value};
use tokio::sync::RwLock;

use crate::ai;
use crate::ai::pi::types::{
    now_timestamp, AssistantContent, AssistantMessage, AssistantMessageEvent, Context, Message,
    TextContent, ToolCall, ToolResultMessage, UserContent, UserMessage, UserMessageContent,
};
use crate::repository::entity::NewMessageEntity;
use crate::repository::{conversation_repository, model_provider_repository};
use crate::service::tool::{self, ToolContext};
use crate::state::{AppState, ConversationState};

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
    let (stop_tx, _) = tokio::sync::watch::channel(false);
    let conv_state = Arc::new(RwLock::new(ConversationState {
        chunks: Vec::new(),
        done: false,
        notify: tx,
        stop: stop_tx,
        query: false,
    }));
    // 防重入: entry 原子检查并替换, 持锁期间不 await, 状态锁被占用视为运行中
    match state.conversation_states.entry(conversation_id) {
        dashmap::mapref::entry::Entry::Occupied(mut entry) => {
            let finished = match entry.get().try_read() {
                Ok(s) => s.done,
                Err(_) => false,
            };
            if !finished {
                tracing::warn!(
                    "Conversation start rejected: conversation_id={} already running",
                    conversation_id
                );
                return false;
            }
            entry.insert(conv_state);
        }
        dashmap::mapref::entry::Entry::Vacant(entry) => {
            entry.insert(conv_state);
        }
    }
    tracing::info!(
        "Conversation started: conversation_id={} model={} thinking={}",
        conversation_id,
        model,
        thinking
    );
    tokio::spawn(run_conversation(
        state.clone(),
        conversation_id,
        task_content,
        model_provider_id,
        model,
        thinking,
    ));
    true
}

// 启动历史回放查询, 不执行模型调用
pub async fn start_conversation_query(state: &AppState, conversation_id: i64) -> bool {
    let (tx, _) = tokio::sync::watch::channel(0u64);
    let (stop_tx, _) = tokio::sync::watch::channel(false);
    let conv_state = Arc::new(RwLock::new(ConversationState {
        chunks: Vec::new(),
        done: false,
        notify: tx,
        stop: stop_tx,
        query: true,
    }));
    // 防重入: entry 原子检查并替换, 持锁期间不 await, 状态锁被占用视为运行中
    match state.conversation_states.entry(conversation_id) {
        dashmap::mapref::entry::Entry::Occupied(mut entry) => {
            let finished = match entry.get().try_read() {
                Ok(s) => s.done,
                Err(_) => false,
            };
            if !finished {
                tracing::warn!(
                    "Conversation query rejected: conversation_id={} already running",
                    conversation_id
                );
                return false;
            }
            entry.insert(conv_state);
        }
        dashmap::mapref::entry::Entry::Vacant(entry) => {
            entry.insert(conv_state);
        }
    }
    tracing::info!(
        "Conversation query started: conversation_id={}",
        conversation_id
    );
    tokio::spawn(run_conversation(
        state.clone(),
        conversation_id,
        String::new(),
        0,
        String::new(),
        false,
    ));
    true
}

// 查询对话状态
pub fn get_conversation_state(
    state: &AppState,
    conversation_id: i64,
) -> Option<Arc<RwLock<ConversationState>>> {
    state
        .conversation_states
        .get(&conversation_id)
        .map(|r| r.clone())
}

// 查询对话是否正在执行: 存在内存状态且未结束且非回放查询会话视为运行中
pub fn is_conversation_running(state: &AppState, conversation_id: i64) -> bool {
    match state.conversation_states.get(&conversation_id) {
        Some(conv_state) => match conv_state.try_read() {
            Ok(s) => !s.done && !s.query,
            // 状态锁被占用(正在写入)视为运行中, 与 start_conversation 的占用判定一致
            Err(_) => true,
        },
        None => false,
    }
}

// 停止对话: 发送停止信号, 对话未在运行(无内存状态或已结束)返回 false, 幂等无副作用
pub async fn stop_conversation(state: &AppState, conversation_id: i64) -> bool {
    if let Some(conv_state) = state.conversation_states.get(&conversation_id) {
        let s = conv_state.read().await;
        if s.done {
            return false;
        }
        let _ = s.stop.send(true);
        tracing::info!(
            "Conversation stop requested: conversation_id={}",
            conversation_id
        );
        return true;
    }
    false
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
    let result = std::panic::AssertUnwindSafe(do_run_conversation(
        &state,
        conversation_id,
        &task_content,
        model_provider_id,
        &model,
        thinking,
    ))
    .catch_unwind()
    .await
    .unwrap_or_else(|e| {
        let msg = e
            .downcast_ref::<String>()
            .cloned()
            .or_else(|| e.downcast_ref::<&str>().map(|s| s.to_string()))
            .unwrap_or_else(|| "unknown".to_string());
        Err(anyhow::anyhow!("conversation panicked: {}", msg))
    });
    if let Err(e) = result {
        publish_chunk(
            &state,
            conversation_id,
            "error",
            &format!("Conversation execution failed: {}", e),
            json!({}),
        )
        .await;
        tracing::error!(
            "Conversation execution failed: conversation_id={} error={}",
            conversation_id,
            e
        );
    }
    finish_conversation(&state, conversation_id).await;
    // 执行任务存储的 chunk 太碎立即移除, 查询任务的状态则保留 5 分钟后再移除
    if task_content.is_empty() {
        // 仅当状态未被新启动的对话替换时才移除
        if let Some(conv_state) = state
            .conversation_states
            .get(&conversation_id)
            .map(|r| r.clone())
        {
            let conversations = state.conversation_states.clone();
            tokio::spawn(async move {
                tokio::time::sleep(std::time::Duration::from_secs(QUERY_STATE_TTL_SECS)).await;
                conversations.remove_if(&conversation_id, |_, current| {
                    Arc::ptr_eq(current, &conv_state)
                });
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
    publish_chunk(
        state,
        conversation_id,
        "system",
        &conversation.conversation.system_prompt,
        json!({}),
    )
    .await;

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
                            publish_chunk(
                                state,
                                conversation_id,
                                "thinking",
                                &t.thinking,
                                json!({}),
                            )
                            .await;
                        }
                        AssistantContent::Text(t) => {
                            publish_chunk(state, conversation_id, "text", &t.text, json!({})).await;
                        }
                        AssistantContent::ToolCall(tc) => {
                            let input_str =
                                serde_json::to_string(&tc.arguments).unwrap_or_default();
                            publish_chunk(
                                state,
                                conversation_id,
                                "tool_use",
                                &input_str,
                                json!({"_id": tc.id, "name": tc.name}),
                            )
                            .await;
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
                publish_chunk(
                    state,
                    conversation_id,
                    "tool_result",
                    &text,
                    json!({"_id": tool_result.tool_call_id, "is_error": tool_result.is_error}),
                )
                .await;
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
    let tools: Vec<crate::ai::pi::types::Tool> = tool::list_tools(
        conversation.conversation.task_id.is_some(),
        conversation.conversation.schedule_id.is_some(),
    )
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
        .filter_map(
            |msg| match serde_json::from_value::<Message>(msg.content.clone()) {
                Ok(m) => Some(m),
                Err(e) => {
                    tracing::warn!("Skip unparsable history message: id={} error={}", msg.id, e);
                    None
                }
            },
        )
        .collect();
    let task_message = Message::User(UserMessage {
        content: UserMessageContent::Text(task_content.to_string()),
        timestamp: now_timestamp(),
    });
    messages.push(task_message.clone());
    // 本轮新增的消息(结束后统一持久化)
    let mut new_messages: Vec<Message> = vec![task_message];

    // 订阅停止信号(内存状态必存在, 取不到时退化为永不停机的空信号)
    let mut stop_rx = match state.conversation_states.get(&conversation_id) {
        Some(conv_state) => conv_state.read().await.stop.subscribe(),
        None => tokio::sync::watch::channel(false).1,
    };

    // 流错误记录: 报错时不直接返回, 跳出循环走统一收尾, 已完整消息入库后再返回错误
    let mut stream_error: Option<anyhow::Error> = None;

    // 执行 agent loop
    'agent_loop: loop {
        // 每轮开头检测停止信号, 覆盖工具执行期间触发暂停的场景(当前工具执行完入库后再退出, 不强制杀工具子进程)
        if *stop_rx.borrow() {
            publish_chunk(state, conversation_id, "stopped", "", json!({})).await;
            tracing::info!("Conversation stopped: conversation_id={}", conversation_id);
            break 'agent_loop;
        }

        // 发送模型请求(按 provider 协议类型路由, 统一返回 pi 基准协议事件流)
        let context = Context {
            system_prompt: Some(system_prompt.clone()),
            messages: messages.clone(),
            tools: Some(tools.clone()),
        };
        let mut stream = match ai::client::stream(&provider, model, thinking, 16000, &context) {
            Ok(s) => s,
            Err(e) => {
                // 流发起失败同样走统一收尾, 已完整消息入库后再返回错误
                stream_error = Some(e.into());
                break 'agent_loop;
            }
        };

        // pi 事件流: 每个事件携带完整 partial 消息, done/error 事件携带最终 assistant message
        let mut assistant_msg: Option<AssistantMessage> = None;
        // 停止标记: 流读取中被停止信号打断时置位
        let mut stopped = false;
        loop {
            // 同时监听模型流事件与停止信号, 停止信号触发时跳出循环优雅收尾
            tokio::select! {
                event = stream.next() => {
                    let event = match event {
                        Some(e) => e,
                        None => break,
                    };
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
                        AssistantMessageEvent::ToolcallStart {
                            content_index,
                            partial,
                        } => {
                            if let Some(AssistantContent::ToolCall(tc)) = partial.content.get(content_index)
                            {
                                publish_chunk(
                                    state,
                                    conversation_id,
                                    "tool_use",
                                    "",
                                    json!({"_id": tc.id, "name": tc.name}),
                                )
                                .await;
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
                            // 记录流错误并跳出流读取循环, 错误 chunk 由外层 run_conversation 统一发布
                            let message = error
                                .error_message
                                .clone()
                                .unwrap_or_else(|| "unknown error".to_string());
                            stream_error = Some(anyhow::anyhow!("model stream error: {}", message));
                            break;
                        }
                    }
                }
                _ = stop_rx.changed() => {
                    // 停止信号触发, 跳出流读取循环优雅收尾
                    stopped = true;
                    break;
                }
            }
        }
        // 停止时 drop stream, reqwest 的 HTTP 连接随流 drop 自动取消, 即真正中断模型 API 调用
        drop(stream);

        if stopped {
            publish_chunk(state, conversation_id, "stopped", "", json!({})).await;
            tracing::info!("Conversation stopped: conversation_id={}", conversation_id);
            break 'agent_loop;
        }

        // 流中途报错: 跳出 agent loop 走统一收尾, 本轮未完成的 partial assistant 消息丢弃不入库
        if stream_error.is_some() {
            break 'agent_loop;
        }

        let mut msg =
            assistant_msg.ok_or_else(|| anyhow::anyhow!("no assistant message received"))?;
        // 工具参数归一化: 模型可能返回非对象 arguments(数组/字符串等), 回放给 API 会被拒绝且入库后永久毒化历史, 统一回退空对象
        for block in &mut msg.content {
            if let AssistantContent::ToolCall(tc) = block {
                if !tc.arguments.is_object() {
                    tracing::warn!(
                        "Tool call arguments normalized to object: conversation_id={} tool={} raw_arguments={}",
                        conversation_id,
                        tc.name,
                        tc.arguments
                    );
                    tc.arguments = json!({});
                }
            }
        }
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
            break 'agent_loop;
        }

        // 工具调用(每个 toolCall 对应一条独立的 toolResult 消息, 对齐 pi)
        for tool_call in &tool_calls {
            let ctx = ToolContext {
                state: state.clone(),
                work_dir: conversation.conversation.work_dir.clone(),
                task_id: conversation.conversation.task_id,
            };
            let (tool_content, is_error) =
                tool::execute_tool(&tool_call.name, &tool_call.arguments, &ctx).await;
            publish_chunk(
                state,
                conversation_id,
                "tool_result",
                &tool_content,
                json!({"_id": tool_call.id, "is_error": is_error}),
            )
            .await;
            let tool_result = Message::ToolResult(ToolResultMessage {
                tool_call_id: tool_call.id.clone(),
                tool_name: tool_call.name.clone(),
                content: vec![UserContent::Text(TextContent {
                    text: tool_content,
                    text_signature: None,
                })],
                is_error,
                timestamp: now_timestamp(),
            });
            messages.push(tool_result.clone());
            new_messages.push(tool_result);
        }
    }

    // 对话结束(正常结束/手动停止/模型报错)统一持久化本轮新增的完整消息(content 列存整条 pi 消息 JSON)
    // 停止或报错时未完成的 partial assistant 消息不入库, 避免半截 tool_call 破坏 pi 消息协议导致续跑上下文损坏
    let messages_to_save: Vec<NewMessageEntity> = new_messages
        .iter()
        .map(|m| NewMessageEntity {
            content: serde_json::to_value(m).unwrap_or_default(),
        })
        .collect();
    conversation_repository::add_conversation_messages(
        &state.db,
        conversation_id,
        &messages_to_save,
    )
    .await?;
    // 已完整消息入库后再返回流错误, 由 run_conversation 统一发布 error chunk 并收尾
    if let Some(e) = stream_error {
        return Err(e);
    }
    tracing::info!("Conversation finished: conversation_id={}", conversation_id);
    Ok(())
}
