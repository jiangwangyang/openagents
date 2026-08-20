// 对话服务: 启动对话, 查询状态, 发布 SSE chunk, 后台 agent loop
use std::sync::Arc;

use futures_util::{FutureExt, StreamExt};
use serde_json::{json, Value};
use tokio::sync::RwLock;

use crate::ai;
use crate::ai::pi::types::{now_timestamp, AssistantContent, AssistantMessage, AssistantMessageEvent, Context, Message, TextContent, Tool, ToolCall, ToolResultMessage, UserContent, UserMessage, UserMessageContent};
use crate::repository::{conversation_repository, model_provider_repository};
use crate::service::tool::{self, ToolContext};
use crate::state::{AppState, ConversationState};

// 构造纯文本用户消息
pub fn user_text_message(text: &str) -> Message {
    Message::User(UserMessage { content: UserMessageContent::Text(text.to_string()), timestamp: now_timestamp() })
}

// 拼接用户内容块中的文本(忽略图片块)
pub fn user_blocks_text(blocks: &[UserContent]) -> String {
    blocks
        .iter()
        .filter_map(|b| match b {
            UserContent::Text(t) => Some(t.text.as_str()),
            _ => None,
        })
        .collect::<Vec<_>>()
        .join("\n")
}

// 提取用户消息纯文本(纯文本直取, 内容块拼接)
pub fn user_message_text(content: &UserMessageContent) -> String {
    match content {
        UserMessageContent::Text(s) => s.clone(),
        UserMessageContent::Blocks(blocks) => user_blocks_text(blocks),
    }
}

// 提取助手消息最后一个文本块内容(无文本块返回空串)
pub fn assistant_message_text(message: &AssistantMessage) -> String {
    message
        .content
        .iter()
        .rev()
        .find_map(|b| match b {
            AssistantContent::Text(t) => Some(t.text.clone()),
            _ => None,
        })
        .unwrap_or_default()
}

// 启动对话, 同一 conversation 不允许同时运行多个; query 为 true 时仅回放历史, 不执行模型调用
pub async fn start_conversation(state: &AppState, conversation_id: i64, task_content: String, model_provider_id: i64, model: String, thinking: bool, query: bool) -> bool {
    let (tx, _) = tokio::sync::watch::channel(0u64);
    let (stop_tx, _) = tokio::sync::watch::channel(false);
    let conv_state = Arc::new(RwLock::new(ConversationState { chunks: Vec::new(), query, notify: Some(tx), stop: stop_tx }));
    // 防重入: entry 原子检查并插入, 持锁期间不 await; 已存在(运行中)则拒绝
    // entry 存在即运行中: run_conversation 先从 map 移除再标记 done, 不会出现存在但已结束的残留
    match state.conversation_states.entry(conversation_id) {
        dashmap::mapref::entry::Entry::Occupied(_) => {
            tracing::warn!("Conversation start rejected: conversation_id={} already running query={}", conversation_id, query);
            return false;
        }
        dashmap::mapref::entry::Entry::Vacant(entry) => {
            entry.insert(conv_state.clone());
        }
    }
    tracing::info!("Conversation started: conversation_id={} model={} thinking={} query={}", conversation_id, model, thinking, query);
    tokio::spawn(run_conversation(state.clone(), conv_state, conversation_id, task_content, model_provider_id, model, thinking));
    true
}

// 查询对话状态
pub fn get_conversation_state(state: &AppState, conversation_id: i64) -> Option<Arc<RwLock<ConversationState>>> {
    state.conversation_states.get(&conversation_id).map(|r| r.clone())
}

// 查询对话是否正在执行: entry 存在即运行中, 仅需排除回放查询会话
pub fn is_conversation_running(state: &AppState, conversation_id: i64) -> bool {
    match state.conversation_states.get(&conversation_id) {
        Some(conv_state) => match conv_state.try_read() {
            Ok(s) => !s.query,
            // 状态锁被占用(正在写入)视为运行中
            Err(_) => true,
        },
        None => false,
    }
}

// 停止对话: 发送停止信号, 对话未在运行返回 false, 幂等无副作用
pub async fn stop_conversation(state: &AppState, conversation_id: i64) -> bool {
    // entry 存在即运行中, 直接发送停止信号; 停止信号幂等, 重复停止无副作用
    if let Some(conv_state) = state.conversation_states.get(&conversation_id) {
        let _ = conv_state.read().await.stop.send(true);
        tracing::info!("Conversation stop requested: conversation_id={}", conversation_id);
        return true;
    }
    false
}

// 发布 SSE chunk
pub async fn publish_chunk(state: &AppState, conversation_id: i64, msg_type: &str, text: &str, kwargs: Value) {
    if let Some(conv_state) = state.conversation_states.get(&conversation_id) {
        let mut s = conv_state.write().await;
        let mut chunk = json!({"type": msg_type, "text": text});
        if let Value::Object(map) = kwargs {
            for (k, v) in map {
                chunk[k] = v;
            }
        }
        s.chunks.push(chunk);
        // 通道已关闭(对话已结束)时无需通知
        if let Some(notify) = &s.notify {
            let _ = notify.send(s.chunks.len() as u64);
        }
    }
}

// 后台 agent loop, conv_state 为本次启动插入的状态, 收尾时据此避免误删替换后的新状态
async fn run_conversation(state: AppState, conv_state: Arc<RwLock<ConversationState>>, conversation_id: i64, task_content: String, model_provider_id: i64, model: String, thinking: bool) {
    // 捕获 panic 兜底, 保证对话必然标记完成, 避免对话被永久锁死
    let result = std::panic::AssertUnwindSafe(do_run_conversation(&state, conversation_id, &task_content, model_provider_id, &model, thinking)).catch_unwind().await.unwrap_or_else(|e| {
        let msg = e.downcast_ref::<String>().cloned().or_else(|| e.downcast_ref::<&str>().map(|s| s.to_string())).unwrap_or_else(|| "unknown".to_string());
        Err(anyhow::anyhow!("conversation panicked: {}", msg))
    });
    if let Err(e) = result {
        publish_chunk(&state, conversation_id, "error", &format!("Conversation execution failed: {}", e), json!({})).await;
        tracing::error!("Conversation execution failed: conversation_id={} error={}", conversation_id, e);
    }
    // 先从 map 移除, 保证 entry 存在即运行中, 等待方看到 done 时 entry 必然已移除, 新启动必然走 Vacant 分支
    // 仅当状态未被替换时才移除(ptr_eq 防御性校验); 已连接的 SSE 流持有自身 Arc 不受影响
    state.conversation_states.remove_if(&conversation_id, |_, current| Arc::ptr_eq(current, &conv_state));
    // 关闭通知通道标记完成: 等待方与 SSE 流的 changed() 返回 Err 或发现通道关闭后自行结束, 其持有自身 Arc 与订阅不依赖 map
    conv_state.write().await.notify = None;
}

// 实际对话逻辑
async fn do_run_conversation(state: &AppState, conversation_id: i64, task_content: &str, model_provider_id: i64, model: &str, thinking: bool) -> anyhow::Result<()> {
    // 查询历史消息
    let conversation = conversation_repository::get_conversation_with_messages(&state.db, conversation_id).await?.ok_or_else(|| anyhow::anyhow!("conversation not found"))?;

    // 流式数据开头发布系统提示词
    publish_chunk(state, conversation_id, "system", &conversation.conversation.system_prompt, json!({})).await;

    // 解析历史消息(content 列为 pi 消息协议 JSON): 单趟完成 chunk 回放与模型上下文构造, 无法解析的消息跳过
    let mut messages: Vec<Message> = Vec::new();
    for msg in &conversation.messages {
        let parsed = match serde_json::from_value::<Message>(msg.content.clone()) {
            Ok(m) => m,
            Err(e) => {
                tracing::warn!("Skip unparsable history message: id={} error={}", msg.id, e);
                continue;
            }
        };
        match &parsed {
            Message::User(user) => {
                let text = user_message_text(&user.content);
                publish_chunk(state, conversation_id, "user", &text, json!({})).await;
            }
            Message::Assistant(assistant) => {
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
                publish_chunk(state, conversation_id, "usage", "", json!({"cache_read_input_tokens": assistant.usage.cache_read, "input_tokens": assistant.usage.input, "output_tokens": assistant.usage.output})).await;
            }
            Message::ToolResult(tool_result) => {
                let text = user_blocks_text(&tool_result.content);
                publish_chunk(state, conversation_id, "tool_result", &text, json!({"_id": tool_result.tool_call_id, "is_error": tool_result.is_error})).await;
            }
        }
        messages.push(parsed);
    }
    if !task_content.is_empty() {
        publish_chunk(state, conversation_id, "user", task_content, json!({})).await;
    }

    // 没有任务直接结束
    if task_content.is_empty() {
        return Ok(());
    }

    // 模型调用数据
    let provider = model_provider_repository::get_model_provider(&state.db, model_provider_id).await?.ok_or_else(|| anyhow::anyhow!("model provider not found"))?;
    let system_prompt = conversation.conversation.system_prompt.clone();
    let tools: Vec<Tool> = tool::list_tools(conversation.conversation.task_id.is_some(), conversation.conversation.schedule_id.is_some()).iter().map(|t| Tool { name: t.name.clone(), description: t.description.clone(), parameters: t.input_schema.clone() }).collect();

    // 本轮任务消息并入上下文
    let task_message = user_text_message(task_content);
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
        let context = Context { system_prompt: Some(system_prompt.clone()), messages: messages.clone(), tools: Some(tools.clone()) };
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
                            // 记录流错误并跳出流读取循环, 错误 chunk 由外层 run_conversation 统一发布
                            let message = error.error_message.clone().unwrap_or_else(|| "unknown error".to_string());
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

        let mut msg = assistant_msg.ok_or_else(|| anyhow::anyhow!("no assistant message received"))?;
        // 工具参数归一化: 模型可能返回非对象 arguments(数组/字符串等), 回放给 API 会被拒绝且入库后永久毒化历史, 统一回退空对象
        for block in &mut msg.content {
            if let AssistantContent::ToolCall(tc) = block {
                if !tc.arguments.is_object() {
                    tracing::warn!("Tool call arguments normalized to object: conversation_id={} tool={} raw_arguments={}", conversation_id, tc.name, tc.arguments);
                    tc.arguments = json!({});
                }
            }
        }
        tracing::info!("Model round completed: conversation_id={} stop_reason={:?} input_tokens={} output_tokens={} cache_read_input_tokens={}", conversation_id, msg.stop_reason, msg.usage.input, msg.usage.output, msg.usage.cache_read);
        publish_chunk(state, conversation_id, "usage", "", json!({"cache_read_input_tokens": msg.usage.cache_read, "input_tokens": msg.usage.input, "output_tokens": msg.usage.output})).await;
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
            let ctx = ToolContext { state: state.clone(), work_dir: conversation.conversation.work_dir.clone(), task_id: conversation.conversation.task_id };
            let (tool_content, is_error) = tool::execute_tool(&tool_call.name, &tool_call.arguments, &ctx).await;
            publish_chunk(state, conversation_id, "tool_result", &tool_content, json!({"_id": tool_call.id, "is_error": is_error})).await;
            let tool_result = Message::ToolResult(ToolResultMessage { tool_call_id: tool_call.id.clone(), tool_name: tool_call.name.clone(), content: vec![UserContent::Text(TextContent { text: tool_content, text_signature: None })], is_error, timestamp: now_timestamp() });
            messages.push(tool_result.clone());
            new_messages.push(tool_result);
        }
    }

    // 对话结束(正常结束/手动停止/模型报错)统一持久化本轮新增的完整消息(content 列存整条 pi 消息 JSON)
    // 停止或报错时未完成的 partial assistant 消息不入库, 避免半截 tool_call 破坏 pi 消息协议导致续跑上下文损坏
    let messages_to_save: Vec<Value> = new_messages.iter().map(|m| serde_json::to_value(m).unwrap_or_default()).collect();
    conversation_repository::add_conversation_messages(&state.db, conversation_id, &messages_to_save).await?;
    // 已完整消息入库后再返回流错误, 由 run_conversation 统一发布 error chunk 并收尾
    if let Some(e) = stream_error {
        return Err(e);
    }
    tracing::info!("Conversation finished: conversation_id={}", conversation_id);
    Ok(())
}
