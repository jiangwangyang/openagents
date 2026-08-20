// OpenAI Responses 流式客户端 + pi 基准协议双向适配
// (移植自 pi/packages/ai/src/api/openai-responses.ts 与 openai-responses-shared.ts)
// 裁剪说明: 不迁移 grammar 自定义工具, deferred tools, serviceTier 计费, prompt cache, compat 选项,
// 重试, abort/timeout, onPayload/onResponse 钩子, temperature/toolChoice
use eventsource_stream::Eventsource;
use futures_util::{Stream, StreamExt};
use reqwest::Client;
use serde_json::{json, Value};

use super::types::{CreateResponseRequest, ListModelsResponse, OutputItem, ReasoningConfig, ResponseOutputContent, ResponseStreamEvent};
use crate::ai::pi::transform_messages::transform_messages;
use crate::ai::pi::types::{now_timestamp, AssistantContent, AssistantMessage, AssistantMessageEvent, Context, Message, Model, StopReason, TextContent, ThinkingContent, Tool, ToolCall, Usage, UserContent, UserMessageContent};
use crate::ai::pi::utils::event_stream::AssistantMessageEventStream;
use crate::ai::pi::utils::hash::short_hash;
use crate::ai::pi::utils::json_parse::parse_streaming_json;
use crate::ai::pi::utils::sanitize_unicode::sanitize_surrogates;
use crate::ai::truncate_str;

// OpenAI Responses 客户端错误
#[derive(Debug, thiserror::Error)]
pub enum OpenAiResponsesError {
    #[error("HTTP 请求失败: {0}")]
    Http(#[from] reqwest::Error),
    #[error("SSE 解析失败: {0}")]
    Sse(String),
    #[error("JSON 反序列化失败: {0}")]
    Json(#[from] serde_json::Error),
    #[error("API 错误 {status}: {body}")]
    Api { status: u16, body: String },
}

// 调用选项(pi OpenAIResponsesOptions 的裁剪版, reasoning_effort 由开关映射为固定级别)
#[derive(Debug, Clone, Default)]
pub struct OpenAIResponsesOptions {
    pub api_key: Option<String>,
    pub max_tokens: Option<u32>,
    pub reasoning_effort: Option<String>,
    pub reasoning_summary: Option<String>,
}

// 共享 HTTP 客户端(内部为 Arc, 克隆仅复制引用, 全应用复用同一连接池)
static HTTP_CLIENT: std::sync::LazyLock<Client> = std::sync::LazyLock::new(Client::new);

// ========== 工具函数 ==========

// TextSignatureV1 编码(对齐 pi encodeTextSignatureV1)
fn encode_text_signature_v1(id: &str, phase: Option<&str>) -> String {
    let mut payload = json!({ "v": 1, "id": id });
    if let Some(phase) = phase {
        payload["phase"] = json!(phase);
    }
    payload.to_string()
}

// textSignature 解析(对齐 pi parseTextSignature): 返回 (id, phase)
fn parse_text_signature(signature: Option<&str>) -> Option<(String, Option<String>)> {
    let signature = signature?;
    if signature.starts_with('{') {
        if let Ok(parsed) = serde_json::from_str::<Value>(signature) {
            if parsed["v"].as_i64() == Some(1) {
                if let Some(id) = parsed["id"].as_str() {
                    let phase = match parsed["phase"].as_str() {
                        Some("commentary") => Some("commentary".to_string()),
                        Some("final_answer") => Some("final_answer".to_string()),
                        _ => None,
                    };
                    return Some((id.to_string(), phase));
                }
            }
        }
    }
    // 旧版纯字符串签名
    Some((signature.to_string(), None))
}

// 工具结果输出转换(对齐 pi convertToolResultOutput): 无图片或模型不支持图片时输出纯文本
fn convert_tool_result_output(model: &Model, content: &[UserContent]) -> Value {
    let text_result = content
        .iter()
        .filter_map(|c| match c {
            UserContent::Text(t) => Some(t.text.as_str()),
            _ => None,
        })
        .collect::<Vec<_>>()
        .join("\n");
    let images: Vec<&crate::ai::pi::types::ImageContent> = content
        .iter()
        .filter_map(|c| match c {
            UserContent::Image(i) => Some(i),
            _ => None,
        })
        .collect();
    let has_text = !text_result.is_empty();
    if images.is_empty() || !model.input.iter().any(|i| i == "image") {
        let text = if has_text {
            text_result
        } else if !images.is_empty() {
            "(see attached image)".to_string()
        } else {
            "(no tool output)".to_string()
        };
        return json!(sanitize_surrogates(&text));
    }
    let mut output: Vec<Value> = Vec::new();
    if has_text {
        output.push(json!({ "type": "input_text", "text": sanitize_surrogates(&text_result) }));
    }
    for image in images {
        output.push(json!({
            "type": "input_image",
            "detail": "auto",
            "image_url": format!("data:{};base64,{}", image.mime_type, image.data),
        }));
    }
    json!(output)
}

// ========== 消息转换 ==========

// convert_responses_messages 选项(pi ConvertResponsesMessagesOptions 裁剪版)
#[derive(Debug, Default)]
pub struct ConvertResponsesMessagesOptions {
    pub include_system_prompt: Option<bool>,
}

// pi 消息列表 -> Responses input(对齐 pi convertResponsesMessages 的裁剪版: 无 grammar/deferred tools)
// 对齐说明: pi 以固定供应商集合判断是否允许 "callId|itemId" 拆分; 本项目 toolCall id 均由本客户端
// 按本协议格式生成(provider 为用户配置的实体 id, 非 pi KnownApi), 恒允许拆分, 故裁剪该参数
pub fn convert_responses_messages(model: &Model, context: &Context, options: &ConvertResponsesMessagesOptions) -> Vec<Value> {
    let mut messages: Vec<Value> = Vec::new();

    // id 片段规范化: 非法字符替换为 '_', 截断 64, 去掉尾部 '_'
    let normalize_id_part = |part: &str| -> String {
        let sanitized: String = part.chars().map(|c| if c.is_ascii_alphanumeric() || c == '_' || c == '-' { c } else { '_' }).collect();
        let normalized: String = sanitized.chars().take(64).collect();
        normalized.trim_end_matches('_').to_string()
    };

    // 外部供应商 item id: fc_ + 短哈希
    let build_foreign_responses_item_id = |item_id: &str| -> String {
        let normalized = format!("fc_{}", short_hash(item_id));
        normalized.chars().take(64).collect()
    };

    let normalize_tool_call_id = |id: &str, _target_model: &Model, source: &AssistantMessage| -> String {
        if !id.contains('|') {
            return normalize_id_part(id);
        }
        let mut parts = id.splitn(2, '|');
        let call_id = parts.next().unwrap_or("");
        let item_id = parts.next().unwrap_or("");
        let normalized_call_id = normalize_id_part(call_id);
        let is_foreign_tool_call = source.provider != model.provider || source.api != model.api;
        let mut normalized_item_id = if is_foreign_tool_call { build_foreign_responses_item_id(item_id) } else { normalize_id_part(item_id) };
        // OpenAI Responses API 要求 item id 以 "fc" 开头
        if !normalized_item_id.starts_with("fc_") {
            normalized_item_id = normalize_id_part(&format!("fc_{}", normalized_item_id));
        }
        format!("{}|{}", normalized_call_id, normalized_item_id)
    };

    let transformed_messages = transform_messages(&context.messages, model, Some(&normalize_tool_call_id));

    let include_system_prompt = options.include_system_prompt.unwrap_or(true);
    if include_system_prompt {
        // 空 system prompt 跳过(对齐 pi 的真值判断), 部分兼容端点同样拒绝空 content
        if let Some(system_prompt) = context.system_prompt.as_ref().filter(|s| !s.trim().is_empty()) {
            // 对齐 pi: reasoning 模型使用 developer 角色(compat.supportsDeveloperRole 默认 true)
            let role = if model.reasoning { "developer" } else { "system" };
            messages.push(json!({ "role": role, "content": sanitize_surrogates(system_prompt) }));
        }
    }

    let mut msg_index = 0;
    for msg in &transformed_messages {
        match msg {
            Message::User(user) => match &user.content {
                UserMessageContent::Text(text) => {
                    messages.push(json!({
                        "role": "user",
                        "content": [{ "type": "input_text", "text": sanitize_surrogates(text) }],
                    }));
                }
                UserMessageContent::Blocks(blocks) => {
                    let content: Vec<Value> = blocks
                        .iter()
                        .map(|item| match item {
                            UserContent::Text(t) => json!({ "type": "input_text", "text": sanitize_surrogates(&t.text) }),
                            UserContent::Image(image) => json!({
                                "type": "input_image",
                                "detail": "auto",
                                "image_url": format!("data:{};base64,{}", image.mime_type, image.data),
                            }),
                        })
                        .collect();
                    if content.is_empty() {
                        msg_index += 1;
                        continue;
                    }
                    messages.push(json!({ "role": "user", "content": content }));
                }
            },
            Message::Assistant(assistant_msg) => {
                let mut output: Vec<Value> = Vec::new();
                let is_same_provider_and_api = assistant_msg.provider == model.provider && assistant_msg.api == model.api;
                let is_different_model = is_same_provider_and_api && assistant_msg.model != model.id;
                let mut text_block_index = 0;
                for block in &assistant_msg.content {
                    match block {
                        AssistantContent::Thinking(t) => {
                            // thinkingSignature 存 reasoning item JSON, 原样回放
                            if let Some(signature) = &t.thinking_signature {
                                if let Ok(reasoning_item) = serde_json::from_str::<Value>(signature) {
                                    output.push(reasoning_item);
                                }
                            }
                        }
                        AssistantContent::Text(t) => {
                            let parsed_signature = parse_text_signature(t.text_signature.as_deref());
                            let fallback_message_id = if text_block_index == 0 { format!("msg_pi_{}", msg_index) } else { format!("msg_pi_{}_{}", msg_index, text_block_index) };
                            text_block_index += 1;
                            // OpenAI 要求 id 最长 64 字符
                            let (msg_id, phase) = match parsed_signature {
                                None => (fallback_message_id, None),
                                Some((id, phase)) if id.len() > 64 => (format!("msg_{}", short_hash(&id)), phase),
                                Some((id, phase)) => (id, phase),
                            };
                            let mut item = json!({
                                "type": "message",
                                "role": "assistant",
                                "content": [{ "type": "output_text", "text": sanitize_surrogates(&t.text), "annotations": [] }],
                                "status": "completed",
                                "id": msg_id,
                            });
                            if let Some(phase) = phase {
                                item["phase"] = json!(phase);
                            }
                            output.push(item);
                        }
                        AssistantContent::ToolCall(tool_call) => {
                            let mut parts = tool_call.id.splitn(2, '|');
                            let call_id = parts.next().unwrap_or("");
                            let item_id_raw = parts.next();
                            // 跨模型消息丢弃 fc_ id 避免配对校验; 非 fc_ 开头的 id 一并丢弃
                            // (function_call item id 必须为 fc_*; 对齐 pi, 本项目无自定义工具分支)
                            let starts_with_fc = item_id_raw.map(|i| i.starts_with("fc_")).unwrap_or(false);
                            let item_id = if !starts_with_fc || is_different_model { None } else { item_id_raw };
                            let mut item = json!({
                                "type": "function_call",
                                "call_id": call_id,
                                "name": tool_call.name,
                                "arguments": serde_json::to_string(&tool_call.arguments).unwrap_or_default(),
                            });
                            if let Some(item_id) = item_id {
                                item["id"] = json!(item_id);
                            }
                            output.push(item);
                        }
                    }
                }
                if output.is_empty() {
                    msg_index += 1;
                    continue;
                }
                messages.extend(output);
            }
            Message::ToolResult(tool_result) => {
                let call_id = tool_result.tool_call_id.split('|').next().unwrap_or("");
                let output = convert_tool_result_output(model, &tool_result.content);
                messages.push(json!({
                    "type": "function_call_output",
                    "call_id": call_id,
                    "output": output,
                }));
            }
        }
        msg_index += 1;
    }
    messages
}

// ========== 工具转换 ==========

// 工具定义转换(对齐 pi convertResponsesTools 的裁剪版: 仅 function 工具, 无 strict/grammar)
pub fn convert_responses_tools(tools: &[Tool]) -> Vec<Value> {
    tools
        .iter()
        .map(|tool| {
            json!({
                "type": "function",
                "name": tool.name,
                "description": tool.description,
                "parameters": tool.parameters,
            })
        })
        .collect()
}

// ========== 流处理 ==========

// 输出槽位(对齐 pi ResponsesOutputSlot; toolCall 的 partialJson 为流式暂存, 结束后丢弃)
enum ResponsesOutputSlot {
    Thinking { content_index: usize },
    Text { content_index: usize },
    ToolCall { content_index: usize, partial_json: String },
}

// 槽位类别
enum SlotKind {
    Thinking,
    Text,
    ToolCall,
}

// 停止原因映射(对齐 pi mapStopReason, 未知 status 返回 Err 对齐 pi 的 throw)
fn map_stop_reason(status: Option<&str>, incomplete_reason: Option<&str>) -> Result<(StopReason, Option<String>), String> {
    let Some(status) = status else {
        return Ok((StopReason::Stop, None));
    };
    match status {
        "completed" => Ok((StopReason::Stop, None)),
        "incomplete" => {
            if incomplete_reason == Some("max_output_tokens") {
                Ok((StopReason::Length, None))
            } else {
                Ok((
                    StopReason::Error,
                    Some(match incomplete_reason {
                        Some(reason) => format!("Response incomplete: {}", reason),
                        None => "Response incomplete without a provider reason".to_string(),
                    }),
                ))
            }
        }
        "failed" | "cancelled" => Ok((StopReason::Error, None)),
        // 这两个状态比较特殊, 视为正常
        "in_progress" | "queued" => Ok((StopReason::Stop, None)),
        other => Err(format!("Unhandled stop reason: {}", other)),
    }
}

// 流事件处理(移植自 pi processResponsesStream, 裁剪 custom_tool_call/serviceTier)
async fn process_responses_stream(mut event_stream: impl Stream<Item = Result<ResponseStreamEvent, OpenAiResponsesError>> + Unpin, output: &mut AssistantMessage, tx: &tokio::sync::mpsc::UnboundedSender<AssistantMessageEvent>) -> Result<(), String> {
    let mut saw_terminal_response_event = false;
    let mut output_slots: std::collections::HashMap<u32, ResponsesOutputSlot> = std::collections::HashMap::new();
    // reasoning item id -> output.content 下标(用于终态响应回填签名)
    let mut reasoning_blocks_by_id: std::collections::HashMap<String, usize> = std::collections::HashMap::new();

    // 事件发送(对齐 pi 的 stream.push)
    let push = |event: AssistantMessageEvent| {
        let _ = tx.send(event);
    };

    // 创建输出槽位(对齐 pi createSlot)
    macro_rules! create_slot {
        ($output_index:expr, $item:expr) => {
            match $item {
                OutputItem::Reasoning(_) => {
                    output.content.push(AssistantContent::Thinking(ThinkingContent { thinking: String::new(), thinking_signature: None, redacted: None }));
                    let content_index = output.content.len() - 1;
                    output_slots.insert($output_index, ResponsesOutputSlot::Thinking { content_index });
                    push(AssistantMessageEvent::ThinkingStart { content_index, partial: output.clone() });
                }
                OutputItem::Message(m) => {
                    if m.phase.as_deref() == Some("final_answer") {
                        output.stop_reason = StopReason::Stop;
                    }
                    output.content.push(AssistantContent::Text(TextContent { text: String::new(), text_signature: None }));
                    let content_index = output.content.len() - 1;
                    output_slots.insert($output_index, ResponsesOutputSlot::Text { content_index });
                    push(AssistantMessageEvent::TextStart { content_index, partial: output.clone() });
                }
                OutputItem::FunctionCall(fc) => {
                    output.content.push(AssistantContent::ToolCall(ToolCall { id: format!("{}|{}", fc.call_id, fc.id.clone().unwrap_or_default()), name: fc.name.clone(), arguments: json!({}), thought_signature: None }));
                    let content_index = output.content.len() - 1;
                    output_slots.insert($output_index, ResponsesOutputSlot::ToolCall { content_index, partial_json: fc.arguments.clone().unwrap_or_default() });
                    push(AssistantMessageEvent::ToolcallStart { content_index, partial: output.clone() });
                }
                OutputItem::Unknown => {}
            }
        };
    }

    // 终态响应处理(对齐 pi finalizeResponse)
    macro_rules! finalize_response {
        ($response:expr) => {{
            let response = $response;
            saw_terminal_response_event = true;
            // Azure 可能仅在 response.completed.response.output 携带 encrypted_content,
            // 回填 reasoning 签名保证 store:false 多轮回放(对齐 pi backfillReasoningSignatures)
            if let Some(items) = &response.output {
                for item in items {
                    let OutputItem::Reasoning(reasoning) = item else { continue };
                    let Some(encrypted) = &reasoning.encrypted_content else { continue };
                    let Some(&content_index) = reasoning_blocks_by_id.get(&reasoning.id) else { continue };
                    if let AssistantContent::Thinking(t) = &mut output.content[content_index] {
                        let Some(signature) = &t.thinking_signature else { continue };
                        let Ok(mut stored) = serde_json::from_str::<Value>(signature) else { continue };
                        if stored["encrypted_content"].as_str().map(|s| !s.is_empty()).unwrap_or(false) {
                            continue;
                        }
                        stored["encrypted_content"] = json!(encrypted);
                        t.thinking_signature = Some(stored.to_string());
                    }
                }
            }
            if let Some(id) = &response.id {
                output.response_id = Some(id.clone());
            }
            if let Some(usage) = &response.usage {
                let cached_tokens = usage.input_tokens_details.as_ref().map(|d| d.cached_tokens).unwrap_or(0);
                let cache_write_tokens = usage.input_tokens_details.as_ref().map(|d| d.cache_write_tokens).unwrap_or(0);
                // OpenAI 的 input_tokens 含缓存读写 token, 需减去
                output.usage.input = usage.input_tokens.saturating_sub(cached_tokens).saturating_sub(cache_write_tokens);
                output.usage.output = usage.output_tokens;
                output.usage.cache_read = cached_tokens;
                output.usage.cache_write = cache_write_tokens;
                output.usage.reasoning = usage.output_tokens_details.as_ref().and_then(|d| d.reasoning_tokens);
                output.usage.total_tokens = usage.total_tokens.unwrap_or(0);
            }
            // 状态映射停止原因; incomplete 保留供应商具体原因
            let status = response.status.as_deref();
            let incomplete_reason = response.incomplete_details.as_ref().and_then(|d| d.reason.clone());
            output.raw_stop_reason = Some(match &incomplete_reason {
                Some(reason) => format!("{}.{}", status.unwrap_or(""), reason),
                None => status.unwrap_or("").to_string(),
            });
            let (mapped, error_message) = map_stop_reason(status, incomplete_reason.as_deref())?;
            output.stop_reason = mapped;
            output.error_message = error_message;
            if output.content.iter().any(|b| matches!(b, AssistantContent::ToolCall(_))) && output.stop_reason == StopReason::Stop {
                output.stop_reason = StopReason::ToolUse;
            }
        }};
    }

    while let Some(result) = event_stream.next().await {
        let event = result.map_err(|e| e.to_string())?;
        match event {
            ResponseStreamEvent::ResponseCreated { response } => {
                output.response_id = response.id;
            }
            ResponseStreamEvent::OutputItemAdded { output_index, item } => {
                create_slot!(output_index, item);
            }
            ResponseStreamEvent::ReasoningSummaryTextDelta { output_index, delta } | ResponseStreamEvent::ReasoningTextDelta { output_index, delta } => {
                if let Some(ResponsesOutputSlot::Thinking { content_index }) = output_slots.get(&output_index) {
                    let content_index = *content_index;
                    if let AssistantContent::Thinking(t) = &mut output.content[content_index] {
                        t.thinking.push_str(&delta);
                    }
                    push(AssistantMessageEvent::ThinkingDelta { content_index, delta, partial: output.clone() });
                }
            }
            ResponseStreamEvent::ReasoningSummaryPartDone { output_index } => {
                if let Some(ResponsesOutputSlot::Thinking { content_index }) = output_slots.get(&output_index) {
                    let content_index = *content_index;
                    if let AssistantContent::Thinking(t) = &mut output.content[content_index] {
                        t.thinking.push_str("\n\n");
                    }
                    push(AssistantMessageEvent::ThinkingDelta { content_index, delta: "\n\n".to_string(), partial: output.clone() });
                }
            }
            ResponseStreamEvent::OutputTextDelta { output_index, delta } | ResponseStreamEvent::RefusalDelta { output_index, delta } => {
                if let Some(ResponsesOutputSlot::Text { content_index }) = output_slots.get(&output_index) {
                    let content_index = *content_index;
                    if let AssistantContent::Text(t) = &mut output.content[content_index] {
                        t.text.push_str(&delta);
                    }
                    push(AssistantMessageEvent::TextDelta { content_index, delta, partial: output.clone() });
                }
            }
            ResponseStreamEvent::FunctionCallArgumentsDelta { output_index, delta } => {
                if let Some(ResponsesOutputSlot::ToolCall { content_index, partial_json }) = output_slots.get_mut(&output_index) {
                    let content_index = *content_index;
                    partial_json.push_str(&delta);
                    let arguments = parse_streaming_json(partial_json);
                    if let AssistantContent::ToolCall(tc) = &mut output.content[content_index] {
                        tc.arguments = arguments;
                    }
                    push(AssistantMessageEvent::ToolcallDelta { content_index, delta, partial: output.clone() });
                }
            }
            ResponseStreamEvent::FunctionCallArgumentsDone { output_index, arguments } => {
                if let Some(ResponsesOutputSlot::ToolCall { content_index, partial_json }) = output_slots.get_mut(&output_index) {
                    let content_index = *content_index;
                    let previous_partial_json = std::mem::replace(partial_json, arguments.clone());
                    let parsed = parse_streaming_json(partial_json);
                    if let AssistantContent::ToolCall(tc) = &mut output.content[content_index] {
                        tc.arguments = parsed;
                    }
                    // 完整参数以增量形式补发差额(对齐 pi)
                    if let Some(delta) = arguments.strip_prefix(previous_partial_json.as_str()) {
                        if !delta.is_empty() {
                            push(AssistantMessageEvent::ToolcallDelta { content_index, delta: delta.to_string(), partial: output.clone() });
                        }
                    }
                }
            }
            ResponseStreamEvent::OutputItemDone { output_index, item } => {
                // message 的 phase 停止原因(对齐 pi applyMessagePhaseStopReason)
                if let OutputItem::Message(m) = &item {
                    if m.phase.as_deref() == Some("final_answer") {
                        output.stop_reason = StopReason::Stop;
                    }
                }
                // 对齐 pi getOrCreateSlot
                if !output_slots.contains_key(&output_index) {
                    create_slot!(output_index, item.clone());
                }
                let slot_info = output_slots.get(&output_index).map(|s| match s {
                    ResponsesOutputSlot::Thinking { content_index } => (SlotKind::Thinking, *content_index),
                    ResponsesOutputSlot::Text { content_index } => (SlotKind::Text, *content_index),
                    ResponsesOutputSlot::ToolCall { content_index, .. } => (SlotKind::ToolCall, *content_index),
                });
                match (slot_info, item) {
                    (Some((SlotKind::Thinking, content_index)), OutputItem::Reasoning(reasoning)) => {
                        let summary_text = reasoning.summary.iter().map(|s| s.text.clone()).collect::<Vec<_>>().join("\n\n");
                        let content_text = reasoning.content.iter().map(|c| c.text.clone()).collect::<Vec<_>>().join("\n\n");
                        // thinkingSignature 存 reasoning item 的 JSON 序列化(对齐 pi JSON.stringify(item))
                        let signature = serde_json::to_value(OutputItem::Reasoning(reasoning.clone())).ok().map(|v| v.to_string());
                        if let AssistantContent::Thinking(t) = &mut output.content[content_index] {
                            if !summary_text.is_empty() {
                                t.thinking = summary_text;
                            } else if !content_text.is_empty() {
                                t.thinking = content_text;
                            }
                            t.thinking_signature = signature;
                        }
                        reasoning_blocks_by_id.insert(reasoning.id.clone(), content_index);
                        let content = match &output.content[content_index] {
                            AssistantContent::Thinking(t) => t.thinking.clone(),
                            _ => String::new(),
                        };
                        push(AssistantMessageEvent::ThinkingEnd { content_index, content, partial: output.clone() });
                        output_slots.remove(&output_index);
                    }
                    (Some((SlotKind::Text, content_index)), OutputItem::Message(m)) => {
                        let text: String = m
                            .content
                            .iter()
                            .map(|c| match c {
                                ResponseOutputContent::OutputText { text } => text.clone(),
                                ResponseOutputContent::Refusal { refusal } => refusal.clone(),
                                ResponseOutputContent::Unknown => String::new(),
                            })
                            .collect();
                        if let AssistantContent::Text(t) = &mut output.content[content_index] {
                            t.text = text;
                            t.text_signature = Some(encode_text_signature_v1(&m.id, m.phase.as_deref()));
                        }
                        let content = match &output.content[content_index] {
                            AssistantContent::Text(t) => t.text.clone(),
                            _ => String::new(),
                        };
                        push(AssistantMessageEvent::TextEnd { content_index, content, partial: output.clone() });
                        output_slots.remove(&output_index);
                    }
                    (Some((SlotKind::ToolCall, content_index)), OutputItem::FunctionCall(fc)) => {
                        let partial_json = match output_slots.get(&output_index) {
                            Some(ResponsesOutputSlot::ToolCall { partial_json, .. }) => partial_json.clone(),
                            _ => String::new(),
                        };
                        let raw = match &fc.arguments {
                            Some(a) if !a.is_empty() => a.clone(),
                            _ if !partial_json.is_empty() => partial_json,
                            _ => "{}".to_string(),
                        };
                        let arguments = parse_streaming_json(&raw);
                        if let AssistantContent::ToolCall(tc) = &mut output.content[content_index] {
                            tc.arguments = arguments;
                        }
                        let tool_call = match &output.content[content_index] {
                            AssistantContent::ToolCall(tc) => tc.clone(),
                            _ => continue,
                        };
                        push(AssistantMessageEvent::ToolcallEnd { content_index, tool_call, partial: output.clone() });
                        output_slots.remove(&output_index);
                    }
                    _ => {}
                }
            }
            ResponseStreamEvent::ResponseCompleted { response } | ResponseStreamEvent::ResponseIncomplete { response } => {
                finalize_response!(response);
            }
            ResponseStreamEvent::ResponseFailed { response } => {
                output.raw_stop_reason = response.status.clone();
                let msg = match &response.error {
                    Some(error) => format!("{}: {}", error.code.clone().unwrap_or_else(|| "unknown".to_string()), error.message.clone().unwrap_or_else(|| "no message".to_string())),
                    None => match response.incomplete_details.as_ref().and_then(|d| d.reason.clone()) {
                        Some(reason) => format!("incomplete: {}", reason),
                        None => "Unknown error (no error details in response)".to_string(),
                    },
                };
                return Err(msg);
            }
            ResponseStreamEvent::Error { code, message } => {
                return Err(format!("Error Code {}: {}", code.unwrap_or_default(), message.unwrap_or_default()));
            }
            ResponseStreamEvent::Unknown => {}
        }
    }
    if !saw_terminal_response_event {
        return Err("OpenAI Responses stream ended before a terminal response event".to_string());
    }
    Ok(())
}

// OpenAI Responses 拒绝 max_output_tokens 低于 16(对齐 pi OPENAI_RESPONSES_MIN_OUTPUT_TOKENS, pi issue #6265)
const MIN_OUTPUT_TOKENS: u32 = 16;

// 请求参数构建(对齐 pi openai-responses.ts buildParams 的裁剪版)
fn build_params(model: &Model, context: &Context, options: &OpenAIResponsesOptions) -> CreateResponseRequest {
    let input = convert_responses_messages(model, context, &ConvertResponsesMessagesOptions::default());
    let mut include: Vec<String> = Vec::new();
    let reasoning = if model.reasoning {
        if options.reasoning_effort.is_some() || options.reasoning_summary.is_some() {
            include.push("reasoning.encrypted_content".to_string());
            Some(ReasoningConfig { effort: options.reasoning_effort.clone().unwrap_or_else(|| "medium".to_string()), summary: Some(options.reasoning_summary.clone().unwrap_or_else(|| "auto".to_string())) })
        } else {
            Some(ReasoningConfig { effort: "none".to_string(), summary: None })
        }
    } else {
        None
    };
    CreateResponseRequest {
        model: model.id.clone(),
        input,
        tools: context.tools.as_ref().filter(|t| !t.is_empty()).map(|t| convert_responses_tools(t)),
        reasoning,
        max_output_tokens: options.max_tokens.map(|t| t.max(MIN_OUTPUT_TOKENS)),
        stream: true,
        // stateless 模式: 历史由本地管理, 每次全量发送; encrypted_content 用于思考内容跨轮回传
        store: false,
        include,
    }
}

// 流式创建响应(对齐 pi stream): 输入 pi Context, 输出 pi AssistantMessageEvent 流,
// 请求/运行时错误对齐 pi 编码为 error 事件而非抛出
pub fn stream(model: &Model, context: &Context, options: &OpenAIResponsesOptions) -> AssistantMessageEventStream {
    let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel::<AssistantMessageEvent>();
    let model = model.clone();
    let context = context.clone();
    let options = options.clone();
    tokio::spawn(async move {
        // 输出消息(对齐 pi 的 output)
        let mut output = AssistantMessage { content: vec![], api: model.api.clone(), provider: model.provider.clone(), model: model.id.clone(), response_model: None, response_id: None, usage: Usage::default(), stop_reason: StopReason::Pending, error_message: None, raw_stop_reason: None, timestamp: now_timestamp() };
        // 失败收尾(对齐 pi 的 catch: 错误编码为 error 事件)
        macro_rules! fail {
            ($message:expr) => {{
                output.stop_reason = StopReason::Error;
                output.error_message = Some($message);
                let _ = tx.send(AssistantMessageEvent::Error { reason: output.stop_reason, error: output });
                return;
            }};
        }

        let params = build_params(&model, &context, &options);
        let url = format!("{}/responses", model.base_url.trim_end_matches('/'));
        tracing::info!("Model request: POST {} model={} input={}", url, params.model, params.input.len());
        let response = match HTTP_CLIENT.post(&url).header("authorization", format!("Bearer {}", options.api_key.clone().unwrap_or_default())).header("content-type", "application/json").json(&params).send().await {
            Ok(r) => r,
            Err(e) => fail!(format!("HTTP 请求失败: {}", e)),
        };
        let status = response.status();
        if !status.is_success() {
            let body = response.text().await.unwrap_or_default();
            // 日志中的 body 截断至 500 字符, 错误事件保留完整内容
            tracing::error!("Model API error: status={} body={}", status.as_u16(), truncate_str(&body, 500));
            fail!(format!("API 错误 {}: {}", status.as_u16(), body));
        }
        let _ = tx.send(AssistantMessageEvent::Start { partial: output.clone() });

        // bytes_stream -> eventsource-stream -> 反序列化为 ResponseStreamEvent
        // 对齐 pi: 解析失败即终止流(不静默跳过, 避免丢失内容块)
        let byte_stream = response.bytes_stream();
        let parsed_stream = byte_stream.eventsource().map(|result| match result {
            Ok(event) => serde_json::from_str::<ResponseStreamEvent>(&event.data).map_err(|e| OpenAiResponsesError::Sse(format!("无法解析 OpenAI Responses SSE 事件: {}; data={}", e, event.data))),
            Err(e) => Err(OpenAiResponsesError::Sse(e.to_string())),
        });
        if let Err(e) = process_responses_stream(Box::pin(parsed_stream), &mut output, &tx).await {
            fail!(e);
        }

        // 对齐 pi: 流结束后校验停止原因
        if output.stop_reason == StopReason::Pending {
            fail!("OpenAI Responses stream ended without a stop reason".to_string());
        }
        if output.stop_reason == StopReason::Aborted || output.stop_reason == StopReason::Error {
            fail!(output.error_message.clone().unwrap_or_else(|| "An unknown error occurred".to_string()));
        }
        let _ = tx.send(AssistantMessageEvent::Done { reason: output.stop_reason, message: output });
    });
    // tokio mpsc 接收端包装为 Stream
    Box::pin(futures_util::stream::poll_fn(move |cx| rx.poll_recv(cx)))
}

// 获取可用模型列表(GET {base_url}/models), 返回模型 id 列表
pub async fn list_models(base_url: &str, api_key: &str) -> Result<Vec<String>, OpenAiResponsesError> {
    let url = format!("{}/models", base_url.trim_end_matches('/'));
    tracing::info!("Model request: GET {}", url);
    let response = HTTP_CLIENT.get(&url).header("authorization", format!("Bearer {}", api_key)).send().await?;

    let status = response.status();
    if !status.is_success() {
        let body = response.text().await.unwrap_or_default();
        // 日志中的 body 截断至 500 字符, 错误对象保留完整内容
        tracing::error!("Model API error: status={} body={}", status.as_u16(), truncate_str(&body, 500));
        return Err(OpenAiResponsesError::Api { status: status.as_u16(), body });
    }

    let list = response.json::<ListModelsResponse>().await?;
    Ok(list.data.into_iter().map(|m| m.id).collect())
}
