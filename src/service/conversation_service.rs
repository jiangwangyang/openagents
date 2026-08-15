use futures_util::{FutureExt, StreamExt};
use serde_json::{json, Value};
// agent loop + 流式状态发布
use std::sync::Arc;
use tokio::sync::RwLock;

use crate::model::anthropic::types::{ContentBlock, ContentBlockDelta, CreateMessageRequest, MessageStreamEvent, RequestMessage, ThinkingConfig};
use crate::model;
use crate::repository::entity::NewMessageEntity;
use crate::repository::{conversation_repository, model_provider_repository};
use crate::state::{AppState, ConversationState};
use crate::tool::{self, ToolContext};

// 查询对话状态在内存中的保留时长(秒)
const QUERY_STATE_TTL_SECS: u64 = 300;

// 启动对话,同一 conversation 不允许同时运行多个
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
    // 防重入: entry 原子检查并替换,持锁期间不 await,状态锁被占用视为运行中
    match state.conversation_states.entry(conversation_id) {
        dashmap::mapref::entry::Entry::Occupied(mut entry) => {
            let finished = match entry.get().try_read() {
                Ok(s) => s.done,
                Err(_) => false,
            };
            if !finished {
                return false;
            }
            entry.insert(conv_state);
        }
        dashmap::mapref::entry::Entry::Vacant(entry) => {
            entry.insert(conv_state);
        }
    }
    tokio::spawn(run_conversation(state.clone(), conversation_id, task_content, model_provider_id, model, thinking));
    true
}

// 启动历史回放查询,不执行模型调用
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
    // 防重入: entry 原子检查并替换,持锁期间不 await,状态锁被占用视为运行中
    match state.conversation_states.entry(conversation_id) {
        dashmap::mapref::entry::Entry::Occupied(mut entry) => {
            let finished = match entry.get().try_read() {
                Ok(s) => s.done,
                Err(_) => false,
            };
            if !finished {
                return false;
            }
            entry.insert(conv_state);
        }
        dashmap::mapref::entry::Entry::Vacant(entry) => {
            entry.insert(conv_state);
        }
    }
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
    // 捕获 panic 兜底,保证 finish_conversation 必执行,避免对话被永久锁死
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
        tracing::error!("Conversation execution failed: {}", e);
    }
    finish_conversation(&state, conversation_id).await;
    // 执行任务存储的 chunk 太碎立即移除,查询任务的状态则保留 5 分钟后再移除
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

    // 发布历史消息
    for msg in &conversation.messages {
        if let Value::String(s) = &msg.content {
            publish_chunk(state, conversation_id, "user", s, json!({})).await;
        } else if let Value::Array(blocks) = &msg.content {
            for block in blocks {
                match block["type"].as_str() {
                    Some("thinking") => {
                        publish_chunk(state, conversation_id, "thinking", block["thinking"].as_str().unwrap_or(""), json!({})).await;
                    }
                    Some("text") => {
                        publish_chunk(state, conversation_id, "text", block["text"].as_str().unwrap_or(""), json!({})).await;
                    }
                    Some("tool_use") => {
                        let input_str = serde_json::to_string(&block["input"]).unwrap_or_default();
                        publish_chunk(state, conversation_id, "tool_use", &input_str, json!({"_id": block["id"], "name": block["name"]})).await;
                    }
                    Some("tool_result") => {
                        publish_chunk(state, conversation_id, "tool_result", block["content"].as_str().unwrap_or(""), json!({"_id": block["tool_use_id"], "is_error": block["is_error"]})).await;
                    }
                    _ => {}
                }
            }
        }
        // 发布使用量消息
        publish_chunk(
            state,
            conversation_id,
            "usage",
            "",
            json!({
                "cache_read_input_tokens": msg.cache_read_input_tokens,
                "input_tokens": msg.input_tokens,
                "output_tokens": msg.output_tokens,
            }),
        )
            .await;
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
    let thinking_config = if thinking {
        ThinkingConfig::Enabled { display: "summarized".to_string() }
    } else {
        ThinkingConfig::Disabled
    };
    let system_prompt = conversation.conversation.system_prompt.clone();
    let tools = tool::list_tools();
    let tools_json: Vec<Value> = tools.iter().map(|t| serde_json::to_value(t).unwrap_or_default()).collect();

    // 构造初始消息列表
    let mut messages: Vec<Value> = conversation
        .messages
        .iter()
        .map(|msg| json!({"role": msg.role, "content": msg.content}))
        .collect();
    messages.push(json!({"role": "user", "content": task_content, "time": chrono::Local::now().to_rfc3339()}));

    // 执行 agent loop
    loop {
        // 1. 发送模型请求(按 provider 协议类型路由,统一返回基准协议事件流)
        let request = CreateMessageRequest {
            model: model.to_string(),
            messages: messages.iter().map(|m| RequestMessage {
                role: m["role"].as_str().unwrap_or("").to_string(),
                content: m["content"].clone(),
            }).collect(),
            system: Some(system_prompt.clone()),
            tools: Some(tools_json.clone()),
            thinking: Some(thinking_config.clone()),
            max_tokens: 16000,
            stream: true,
        };
        let mut stream = model::client::create_message_stream(&provider, &request).await?;

        // 累积完整 assistant message
        let mut assistant_msg: Option<Value> = None;
        let mut input_json = String::new();

        while let Some(event_result) = stream.next().await {
            let event = event_result.map_err(|e| anyhow::anyhow!("stream error: {}", e))?;
            match event {
                MessageStreamEvent::MessageStart { message } => {
                    assistant_msg = Some(json!({
                        "id": message.id,
                        "type": message.msg_type,
                        "role": message.role,
                        "model": message.model,
                        "content": [],
                        "stop_reason": Value::Null,
                        "stop_sequence": Value::Null,
                        "usage": {
                            "input_tokens": message.usage.input_tokens,
                            "output_tokens": message.usage.output_tokens,
                            "cache_creation_input_tokens": message.usage.cache_creation_input_tokens,
                            "cache_read_input_tokens": message.usage.cache_read_input_tokens,
                        }
                    }));
                }
                MessageStreamEvent::ContentBlockStart { content_block, .. } => {
                    if let Some(ref mut msg) = assistant_msg {
                        let block_json = match &content_block {
                            ContentBlock::Thinking { thinking, signature } => {
                                publish_chunk(state, conversation_id, "thinking", "", json!({})).await;
                                json!({"type": "thinking", "thinking": thinking, "signature": signature})
                            }
                            ContentBlock::Text { text } => {
                                publish_chunk(state, conversation_id, "text", "", json!({})).await;
                                json!({"type": "text", "text": text})
                            }
                            ContentBlock::ToolUse { id, name, input } => {
                                publish_chunk(state, conversation_id, "tool_use", "", json!({"_id": id, "name": name})).await;
                                json!({"type": "tool_use", "id": id, "name": name, "input": input})
                            }
                            ContentBlock::RedactedThinking { data } => {
                                json!({"type": "redacted_thinking", "data": data})
                            }
                        };
                        if let Some(content) = msg["content"].as_array_mut() {
                            content.push(block_json);
                        }
                    }
                }
                MessageStreamEvent::ContentBlockDelta { delta, .. } => {
                    if let Some(ref mut msg) = assistant_msg {
                        let content = msg["content"].as_array_mut();
                        if let Some(last) = content.and_then(|c| c.last_mut()) {
                            match delta {
                                ContentBlockDelta::ThinkingDelta { thinking } => {
                                    last["thinking"] = json!(last["thinking"].as_str().unwrap_or("").to_string() + &thinking);
                                    publish_chunk(state, conversation_id, "delta", &thinking, json!({})).await;
                                }
                                ContentBlockDelta::SignatureDelta { signature } => {
                                    last["signature"] = json!(last["signature"].as_str().unwrap_or("").to_string() + &signature);
                                }
                                ContentBlockDelta::TextDelta { text } => {
                                    last["text"] = json!(last["text"].as_str().unwrap_or("").to_string() + &text);
                                    publish_chunk(state, conversation_id, "delta", &text, json!({})).await;
                                }
                                ContentBlockDelta::InputJsonDelta { partial_json } => {
                                    input_json.push_str(&partial_json);
                                    publish_chunk(state, conversation_id, "delta", &partial_json, json!({})).await;
                                }
                            }
                        }
                    }
                }
                MessageStreamEvent::ContentBlockStop { .. } => {
                    if let Some(ref mut msg) = assistant_msg {
                        let content = msg["content"].as_array_mut();
                        if let Some(last) = content.and_then(|c| c.last_mut()) {
                            if last["type"].as_str() == Some("tool_use") {
                                match serde_json::from_str::<Value>(&input_json) {
                                    Ok(parsed) => last["input"] = parsed,
                                    Err(e) => last["input"] = json!({"error": e.to_string()}),
                                }
                                input_json.clear();
                            }
                        }
                    }
                }
                MessageStreamEvent::MessageDelta { delta, usage } => {
                    if let Some(ref mut msg) = assistant_msg {
                        msg["stop_reason"] = json!(delta.stop_reason);
                        msg["usage"]["output_tokens"] = json!(usage.output_tokens);
                        // Responses 协议的 usage 仅在完成事件下发,input_tokens 在此处补齐
                        if let Some(input_tokens) = usage.input_tokens {
                            msg["usage"]["input_tokens"] = json!(input_tokens);
                        }
                        // 缓存命中 token 一并落库,避免持久化后丢失
                        if let Some(cache_read_input_tokens) = usage.cache_read_input_tokens {
                            msg["usage"]["cache_read_input_tokens"] = json!(cache_read_input_tokens);
                        }
                        publish_chunk(
                            state,
                            conversation_id,
                            "usage",
                            "",
                            json!({
                                "cache_read_input_tokens": usage.cache_read_input_tokens,
                                "input_tokens": usage.input_tokens,
                                "output_tokens": usage.output_tokens,
                            }),
                        )
                            .await;
                    }
                }
                MessageStreamEvent::MessageStop => {}
            }
        }

        let msg = assistant_msg.ok_or_else(|| anyhow::anyhow!("no assistant message received"))?;
        let msg_time = chrono::Local::now().to_rfc3339();
        let mut msg_with_time = msg.clone();
        msg_with_time["time"] = json!(msg_time);
        messages.push(msg_with_time.clone());

        // 2. 判断结束
        let tool_use_list: Vec<&Value> = msg["content"]
            .as_array()
            .map(|arr| arr.iter().filter(|b| b["type"].as_str() == Some("tool_use")).collect())
            .unwrap_or_default();

        if tool_use_list.is_empty() {
            // assistant 消息持久化
            let messages_to_save: Vec<NewMessageEntity> = messages
                .iter()
                .filter(|m| m.get("time").is_some())
                .map(|m| NewMessageEntity {
                    role: m["role"].as_str().unwrap_or("").to_string(),
                    content: m["content"].clone(),
                    stop_reason: m["stop_reason"].as_str().unwrap_or("").to_string(),
                    cache_read_input_tokens: m["usage"]["cache_read_input_tokens"].as_i64().unwrap_or(0),
                    input_tokens: m["usage"]["input_tokens"].as_i64().unwrap_or(0),
                    output_tokens: m["usage"]["output_tokens"].as_i64().unwrap_or(0),
                    time: m["time"].as_str().unwrap_or("").to_string(),
                })
                .collect();
            conversation_repository::add_conversation_messages(&state.db, conversation_id, &messages_to_save).await?;
            return Ok(());
        }

        // 3. 工具调用
        let mut tool_result_content = Vec::new();
        for tool_use in &tool_use_list {
            let tool_name = tool_use["name"].as_str().unwrap_or("");
            let tool_input = &tool_use["input"];
            let tool_use_id = tool_use["id"].as_str().unwrap_or("");
            let ctx = ToolContext {
                db: state.db.clone(),
                work_dir: conversation.conversation.work_dir.clone(),
                task_id: conversation.conversation.task_id,
                skills: state.skills.clone(),
            };
            let (tool_content, is_error) = tool::execute_tool(tool_name, tool_input, &ctx).await;
            tool_result_content.push(json!({
                "type": "tool_result",
                "tool_use_id": tool_use_id,
                "content": tool_content,
                "is_error": is_error,
            }));
            publish_chunk(state, conversation_id, "tool_result", &tool_content, json!({"_id": tool_use_id, "is_error": is_error})).await;
        }
        let tool_result_time = chrono::Local::now().to_rfc3339();
        messages.push(json!({
            "role": "user",
            "content": tool_result_content,
            "time": tool_result_time,
        }));
    }
}
