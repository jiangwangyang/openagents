// agent loop + 流式状态发布
use std::sync::Arc;
use dashmap::DashMap;
use futures_util::StreamExt;
use serde_json::{json, Value};
use sqlx::SqlitePool;
use tokio::sync::RwLock;

use crate::anthropic::client::AnthropicClient;
use crate::anthropic::types::{ContentBlock, ContentBlockDelta, CreateMessageRequest, MessageStreamEvent, RequestMessage, ThinkingConfig};
use crate::repository::{conversation_repository, model_provider_repository};
use crate::state::ConversationState;
use crate::tool::{self, ToolContext};

// 启动对话,同一 conversation 不允许同时运行多个
pub async fn start_conversation(
    conversation_id: i64,
    task_content: String,
    model_provider_id: i64,
    model: String,
    thinking: bool,
    conversations: &Arc<DashMap<i64, Arc<RwLock<ConversationState>>>>,
    db: &SqlitePool,
) -> bool {
    // 防重入
    if let Some(state) = conversations.get(&conversation_id) {
        if !state.read().await.done {
            return false;
        }
    }
    let (tx, _) = tokio::sync::watch::channel(0u64);
    let state = Arc::new(RwLock::new(ConversationState {
        chunks: Vec::new(),
        done: false,
        notify: tx,
    }));
    conversations.insert(conversation_id, state.clone());
    let conversations = conversations.clone();
    let db = db.clone();
    tokio::spawn(run_conversation(conversation_id, task_content, model_provider_id, model, thinking, conversations, db));
    true
}

// 启动历史回放查询,不执行模型调用
pub async fn start_conversation_query(
    conversation_id: i64,
    conversations: &Arc<DashMap<i64, Arc<RwLock<ConversationState>>>>,
    db: &SqlitePool,
) -> bool {
    if let Some(state) = conversations.get(&conversation_id) {
        if !state.read().await.done {
            return false;
        }
    }
    let (tx, _) = tokio::sync::watch::channel(0u64);
    let state = Arc::new(RwLock::new(ConversationState {
        chunks: Vec::new(),
        done: false,
        notify: tx,
    }));
    conversations.insert(conversation_id, state.clone());
    let conversations = conversations.clone();
    let db = db.clone();
    tokio::spawn(run_conversation(conversation_id, String::new(), 0, String::new(), false, conversations, db));
    true
}

// 查询对话状态
pub fn get_conversation_state(
    conversation_id: i64,
    conversations: &Arc<DashMap<i64, Arc<RwLock<ConversationState>>>>,
) -> Option<Arc<RwLock<ConversationState>>> {
    conversations.get(&conversation_id).map(|r| r.clone())
}

// 发布 SSE chunk
pub async fn publish_chunk(
    conversation_id: i64,
    msg_type: &str,
    text: &str,
    kwargs: Value,
    conversations: &Arc<DashMap<i64, Arc<RwLock<ConversationState>>>>,
) {
    if let Some(state) = conversations.get(&conversation_id) {
        let mut s = state.write().await;
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
pub async fn finish_conversation(conversation_id: i64, conversations: &Arc<DashMap<i64, Arc<RwLock<ConversationState>>>>) {
    if let Some(state) = conversations.get(&conversation_id) {
        let mut s = state.write().await;
        s.done = true;
        let _ = s.notify.send(s.chunks.len() as u64);
    }
}

// 后台 agent loop
async fn run_conversation(
    conversation_id: i64,
    task_content: String,
    model_provider_id: i64,
    model: String,
    thinking: bool,
    conversations: Arc<DashMap<i64, Arc<RwLock<ConversationState>>>>,
    db: SqlitePool,
) {
    let result = do_run_conversation(conversation_id, &task_content, model_provider_id, &model, thinking, &conversations, &db).await;
    if let Err(e) = result {
        publish_chunk(conversation_id, "error", &format!("Conversation execution failed: {}", e), json!({}), &conversations).await;
        tracing::error!("Conversation execution failed: {}", e);
    }
    finish_conversation(conversation_id, &conversations).await;
    // 执行任务存储的 chunk 太碎需要清理,查询任务状态可以保留
    if !task_content.is_empty() {
        conversations.remove(&conversation_id);
    }
}

// 实际对话逻辑
async fn do_run_conversation(
    conversation_id: i64,
    task_content: &str,
    model_provider_id: i64,
    model: &str,
    thinking: bool,
    conversations: &Arc<DashMap<i64, Arc<RwLock<ConversationState>>>>,
    db: &SqlitePool,
) -> anyhow::Result<()> {
    // 查询历史消息
    let conversation = conversation_repository::get_conversation(db, conversation_id)
        .await?
        .ok_or_else(|| anyhow::anyhow!("conversation not found"))?;

    // 发布历史消息
    for msg in &conversation.messages {
        if let Value::String(s) = &msg.content {
            publish_chunk(conversation_id, "user", s, json!({}), conversations).await;
        } else if let Value::Array(blocks) = &msg.content {
            for block in blocks {
                match block["type"].as_str() {
                    Some("thinking") => {
                        publish_chunk(conversation_id, "thinking", block["thinking"].as_str().unwrap_or(""), json!({}), conversations).await;
                    }
                    Some("text") => {
                        publish_chunk(conversation_id, "text", block["text"].as_str().unwrap_or(""), json!({}), conversations).await;
                    }
                    Some("tool_use") => {
                        let input_str = serde_json::to_string(&block["input"]).unwrap_or_default();
                        publish_chunk(conversation_id, "tool_use", &input_str, json!({"_id": block["id"], "name": block["name"]}), conversations).await;
                    }
                    Some("tool_result") => {
                        publish_chunk(conversation_id, "tool_result", block["content"].as_str().unwrap_or(""), json!({"_id": block["tool_use_id"], "is_error": block["is_error"]}), conversations).await;
                    }
                    _ => {}
                }
            }
        }
        // 发布使用量消息
        publish_chunk(
            conversation_id,
            "usage",
            "",
            json!({
                "cache_read_input_tokens": msg.cache_read_input_tokens,
                "input_tokens": msg.input_tokens,
                "output_tokens": msg.output_tokens,
            }),
            conversations,
        )
        .await;
    }
    if !task_content.is_empty() {
        publish_chunk(conversation_id, "user", task_content, json!({}), conversations).await;
    }

    // 没有任务直接结束
    if task_content.is_empty() {
        return Ok(());
    }

    // 模型调用数据
    let provider = model_provider_repository::get_model_provider(db, model_provider_id)
        .await?
        .ok_or_else(|| anyhow::anyhow!("model provider not found"))?;
    let base_url = provider.base_url.as_str();
    let api_key = provider.api_key.as_str();
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
    let anthropic_client = AnthropicClient::new(base_url, api_key);
    loop {
        // 1. 发送 anthropic 请求
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
        let mut stream = anthropic_client.create_message_stream(&request).await?;

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
                                publish_chunk(conversation_id, "thinking", "", json!({}), conversations).await;
                                json!({"type": "thinking", "thinking": thinking, "signature": signature})
                            }
                            ContentBlock::Text { text } => {
                                publish_chunk(conversation_id, "text", "", json!({}), conversations).await;
                                json!({"type": "text", "text": text})
                            }
                            ContentBlock::ToolUse { id, name, input } => {
                                publish_chunk(conversation_id, "tool_use", "", json!({"_id": id, "name": name}), conversations).await;
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
                                    publish_chunk(conversation_id, "delta", &thinking, json!({}), conversations).await;
                                }
                                ContentBlockDelta::SignatureDelta { signature } => {
                                    last["signature"] = json!(last["signature"].as_str().unwrap_or("").to_string() + &signature);
                                }
                                ContentBlockDelta::TextDelta { text } => {
                                    last["text"] = json!(last["text"].as_str().unwrap_or("").to_string() + &text);
                                    publish_chunk(conversation_id, "delta", &text, json!({}), conversations).await;
                                }
                                ContentBlockDelta::InputJsonDelta { partial_json } => {
                                    input_json.push_str(&partial_json);
                                    publish_chunk(conversation_id, "delta", &partial_json, json!({}), conversations).await;
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
                        publish_chunk(
                            conversation_id,
                            "usage",
                            "",
                            json!({
                                "cache_read_input_tokens": usage.cache_read_input_tokens,
                                "input_tokens": usage.input_tokens,
                                "output_tokens": usage.output_tokens,
                            }),
                            conversations,
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
            let messages_to_save: Vec<(String, Value, String, i64, i64, i64, String)> = messages
                .iter()
                .filter(|m| m.get("time").is_some())
                .map(|m| {
                    (
                        m["role"].as_str().unwrap_or("").to_string(),
                        m["content"].clone(),
                        m["stop_reason"].as_str().unwrap_or("").to_string(),
                        m["usage"]["cache_read_input_tokens"].as_i64().unwrap_or(0),
                        m["usage"]["input_tokens"].as_i64().unwrap_or(0),
                        m["usage"]["output_tokens"].as_i64().unwrap_or(0),
                        m["time"].as_str().unwrap_or("").to_string(),
                    )
                })
                .collect();
            conversation_repository::add_conversation_messages(db, conversation_id, &messages_to_save).await?;
            return Ok(());
        }

        // 3. 工具调用
        let mut tool_result_content = Vec::new();
        for tool_use in &tool_use_list {
            let tool_name = tool_use["name"].as_str().unwrap_or("");
            let tool_input = &tool_use["input"];
            let tool_use_id = tool_use["id"].as_str().unwrap_or("");
            let ctx = ToolContext {
                db: db.clone(),
                work_dir: conversation.conversation.work_dir.clone(),
                task_id: conversation.conversation.task_id,
            };
            let (tool_content, is_error) = tool::execute_tool(tool_name, tool_input, &ctx).await;
            tool_result_content.push(json!({
                "type": "tool_result",
                "tool_use_id": tool_use_id,
                "content": tool_content,
                "is_error": is_error,
            }));
            publish_chunk(conversation_id, "tool_result", &tool_content, json!({"_id": tool_use_id, "is_error": is_error}), conversations).await;
        }
        let tool_result_time = chrono::Local::now().to_rfc3339();
        messages.push(json!({
            "role": "user",
            "content": tool_result_content,
            "time": tool_result_time,
        }));
    }
}
