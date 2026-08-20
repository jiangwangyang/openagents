// Anthropic 流式客户端 + pi 基准协议双向适配(移植自 pi/packages/ai/src/api/anthropic-messages.ts)
// 裁剪说明: 不迁移 OAuth/Claude Code 伪装, cache_control, deferred/strict tools, adaptive thinking,
// 重试, abort/timeout, onPayload/onResponse 钩子, temperature/toolChoice/metadata, fallbacks, cacheWrite1h
use eventsource_stream::Eventsource;
use futures_util::StreamExt;
use reqwest::Client;

use super::types::{ContentBlock, ContentBlockDelta, ContentBlockParam, CreateMessageRequest, ImageSource, ListModelsResponse, MessageParam, MessageParamContent, MessageStreamEvent, RefusalStopDetails, SystemBlock, ThinkingConfig, ToolParam};
use crate::ai::pi::transform_messages::transform_messages;
use crate::ai::pi::types::{now_timestamp, AssistantContent, AssistantMessage, AssistantMessageEvent, Context, Message, Model, StopReason, TextContent, ThinkingContent, Tool, ToolCall, ToolResultMessage, Usage, UserContent, UserMessageContent};
use crate::ai::pi::utils::event_stream::AssistantMessageEventStream;
use crate::ai::pi::utils::json_parse::{parse_json_with_repair, parse_streaming_json};
use crate::ai::pi::utils::sanitize_unicode::sanitize_surrogates;
use crate::ai::truncate_str;

// Anthropic 客户端错误
#[derive(Debug, thiserror::Error)]
pub enum AnthropicError {
    #[error("HTTP 请求失败: {0}")]
    Http(#[from] reqwest::Error),
    #[error("JSON 反序列化失败: {0}")]
    Json(#[from] serde_json::Error),
    #[error("API 错误 {status}: {body}")]
    Api { status: u16, body: String },
}

// 调用选项(pi AnthropicOptions 的裁剪版)
#[derive(Debug, Clone, Default)]
pub struct AnthropicOptions {
    pub api_key: Option<String>,
    pub max_tokens: Option<u32>,
    pub thinking_enabled: Option<bool>,
    pub thinking_budget_tokens: Option<u32>,
}

// 共享 HTTP 客户端(内部为 Arc, 克隆仅复制引用, 全应用复用同一连接池)
static HTTP_CLIENT: std::sync::LazyLock<Client> = std::sync::LazyLock::new(Client::new);

// toolCallId 规范化, 匹配 Anthropic 要求的字符集与长度(对齐 pi normalizeToolCallId)
fn normalize_tool_call_id(id: &str) -> String {
    id.chars().map(|c| if c.is_ascii_alphanumeric() || c == '_' || c == '-' { c } else { '_' }).take(64).collect()
}

// 内容块转 Anthropic 格式(对齐 pi convertContentBlocks): 纯文本拼接为字符串, 含图片时转为块数组
fn convert_content_blocks(content: &[UserContent]) -> MessageParamContent {
    // 仅文本块时拼接为字符串
    let has_images = content.iter().any(|c| matches!(c, UserContent::Image(_)));
    if !has_images {
        let text = content
            .iter()
            .filter_map(|c| match c {
                UserContent::Text(t) => Some(t.text.as_str()),
                _ => None,
            })
            .collect::<Vec<_>>()
            .join("\n");
        // 对齐 pi convertContentBlocks: 空文本原样透传(Anthropic 接受空字符串 tool_result content, pi 不做占位)
        return MessageParamContent::Text(sanitize_surrogates(&text));
    }
    let mut blocks: Vec<ContentBlockParam> = content
        .iter()
        .map(|block| match block {
            UserContent::Text(t) => ContentBlockParam::Text { text: sanitize_surrogates(&t.text) },
            UserContent::Image(image) => ContentBlockParam::Image { source: ImageSource { source_type: "base64".to_string(), media_type: image.mime_type.clone(), data: image.data.clone() } },
        })
        .collect();
    // 仅图片无文本时补占位文本块
    let has_text = blocks.iter().any(|b| matches!(b, ContentBlockParam::Text { .. }));
    if !has_text {
        blocks.insert(0, ContentBlockParam::Text { text: "(see attached image)".to_string() });
    }
    MessageParamContent::Blocks(blocks)
}

// 工具结果消息 -> tool_result 块(对齐 pi convertToolResult 的裁剪版, 无 tool_reference)
fn convert_tool_result(msg: &ToolResultMessage) -> ContentBlockParam {
    ContentBlockParam::ToolResult { tool_use_id: msg.tool_call_id.clone(), content: convert_content_blocks(&msg.content), is_error: msg.is_error }
}

// pi 消息列表 -> Anthropic 请求消息(对齐 pi convertMessages 的裁剪版: 无 OAuth 工具名映射/cache_control/tool_reference)
fn convert_messages(transformed_messages: &[Message]) -> Vec<MessageParam> {
    let mut params: Vec<MessageParam> = Vec::new();
    let mut i = 0;
    while i < transformed_messages.len() {
        let msg = &transformed_messages[i];
        match msg {
            Message::User(user) => match &user.content {
                UserMessageContent::Text(text) => {
                    if !text.trim().is_empty() {
                        params.push(MessageParam { role: "user".to_string(), content: MessageParamContent::Text(sanitize_surrogates(text)) });
                    }
                }
                UserMessageContent::Blocks(blocks) => {
                    let converted: Vec<ContentBlockParam> = blocks
                        .iter()
                        .map(|item| match item {
                            UserContent::Text(t) => ContentBlockParam::Text { text: sanitize_surrogates(&t.text) },
                            UserContent::Image(image) => ContentBlockParam::Image { source: ImageSource { source_type: "base64".to_string(), media_type: image.mime_type.clone(), data: image.data.clone() } },
                        })
                        .filter(|b| match b {
                            ContentBlockParam::Text { text } => !text.trim().is_empty(),
                            _ => true,
                        })
                        .collect();
                    if converted.is_empty() {
                        i += 1;
                        continue;
                    }
                    params.push(MessageParam { role: "user".to_string(), content: MessageParamContent::Blocks(converted) });
                }
            },
            Message::Assistant(assistant) => {
                let mut blocks: Vec<ContentBlockParam> = Vec::new();
                for block in &assistant.content {
                    match block {
                        AssistantContent::Text(t) => {
                            if t.text.trim().is_empty() {
                                continue;
                            }
                            blocks.push(ContentBlockParam::Text { text: sanitize_surrogates(&t.text) });
                        }
                        AssistantContent::Thinking(t) => {
                            // redacted thinking: 不透明载荷原样回传
                            if t.redacted == Some(true) {
                                blocks.push(ContentBlockParam::RedactedThinking { data: t.thinking_signature.clone().unwrap_or_default() });
                                continue;
                            }
                            let signature = t.thinking_signature.clone().unwrap_or_default();
                            let has_signature = !signature.trim().is_empty();
                            if t.thinking.trim().is_empty() && !has_signature {
                                continue;
                            }
                            // 签名缺失/为空(如流被中断)时降级为纯文本(对齐 pi 默认 allowEmptySignature=false)
                            if !has_signature {
                                blocks.push(ContentBlockParam::Text { text: sanitize_surrogates(&t.thinking) });
                            } else {
                                blocks.push(ContentBlockParam::Thinking { thinking: sanitize_surrogates(&t.thinking), signature });
                            }
                        }
                        AssistantContent::ToolCall(tc) => {
                            blocks.push(ContentBlockParam::ToolUse { id: tc.id.clone(), name: tc.name.clone(), input: if tc.arguments.is_null() { serde_json::json!({}) } else { tc.arguments.clone() } });
                        }
                    }
                }
                if blocks.is_empty() {
                    i += 1;
                    continue;
                }
                params.push(MessageParam { role: "assistant".to_string(), content: MessageParamContent::Blocks(blocks) });
            }
            Message::ToolResult(_) => {
                // 收集连续的 toolResult 消息合并为一条 user 消息(对齐 pi)
                let mut tool_results: Vec<ContentBlockParam> = Vec::new();
                let mut j = i;
                while j < transformed_messages.len() {
                    if let Message::ToolResult(tool_result) = &transformed_messages[j] {
                        tool_results.push(convert_tool_result(tool_result));
                        j += 1;
                    } else {
                        break;
                    }
                }
                // 跳过已处理的消息
                i = j - 1;
                params.push(MessageParam { role: "user".to_string(), content: MessageParamContent::Blocks(tool_results) });
            }
        }
        i += 1;
    }
    params
}

// 工具定义转换(对齐 pi convertTools 的裁剪版: 无 strict/eager_input_streaming/cache_control)
fn convert_tools(tools: &[Tool]) -> Vec<ToolParam> {
    tools.iter().map(|tool| ToolParam { name: tool.name.clone(), description: tool.description.clone(), input_schema: tool.parameters.clone() }).collect()
}

// 停止原因映射(对齐 pi mapStopReason, 未知 reason 返回 Err 对齐 pi 的 throw)
fn map_stop_reason(reason: &str, stop_details: Option<&RefusalStopDetails>) -> Result<(StopReason, Option<String>), String> {
    match reason {
        "end_turn" => Ok((StopReason::Stop, None)),
        "max_tokens" => Ok((StopReason::Length, None)),
        "tool_use" => Ok((StopReason::ToolUse, None)),
        "refusal" => Ok((StopReason::Error, Some(stop_details.and_then(|d| d.explanation.clone()).unwrap_or_else(|| "The model refused to complete the request".to_string())))),
        // pause_turn 视为正常停止(对齐 pi 注释: Stop is good enough -> resubmit)
        "pause_turn" => Ok((StopReason::Stop, None)),
        // 本项目不提供 stop sequences, 理论上不会触发
        "stop_sequence" => Ok((StopReason::Stop, None)),
        // 内容被安全过滤标记(SDK 类型中尚未收录)
        "sensitive" => Ok((StopReason::Error, Some("Provider stopped with: sensitive".to_string()))),
        // 未知停止原因(API 可能新增取值)
        other => Err(format!("Unhandled stop reason: {}", other)),
    }
}

// 请求参数构建(对齐 pi buildParams 的裁剪版)
fn build_params(model: &Model, context: &Context, options: &AnthropicOptions) -> CreateMessageRequest {
    let transformed_messages = transform_messages(&context.messages, model, Some(&|id: &str, _model: &Model, _source: &AssistantMessage| normalize_tool_call_id(id)));
    CreateMessageRequest {
        model: model.id.clone(),
        messages: convert_messages(&transformed_messages),
        // 空 system prompt 不传(对齐 pi 的 if (context.systemPrompt) 真值判断), 空文本会触发 400 text content is empty
        system: context.system_prompt.as_ref().filter(|s| !s.trim().is_empty()).map(|s| vec![SystemBlock { block_type: "text".to_string(), text: sanitize_surrogates(s) }]),
        tools: context.tools.as_ref().filter(|t| !t.is_empty()).map(|t| convert_tools(t)),
        // 思考配置: 开关映射固定级别 medium(预算在分发层给定), 关闭时显式 disabled
        thinking: if model.reasoning {
            match options.thinking_enabled {
                Some(true) => Some(ThinkingConfig::Enabled { budget_tokens: options.thinking_budget_tokens.unwrap_or(1024), display: "summarized".to_string() }),
                Some(false) => Some(ThinkingConfig::Disabled),
                None => None,
            }
        } else {
            None
        },
        max_tokens: options.max_tokens.unwrap_or(model.max_tokens),
        stream: true,
    }
}

// 流式创建消息(对齐 pi stream): 输入 pi Context, 输出 pi AssistantMessageEvent 流,
// 请求/运行时错误对齐 pi 编码为 error 事件而非抛出
pub fn stream(model: &Model, context: &Context, options: &AnthropicOptions) -> AssistantMessageEventStream {
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
        let url = format!("{}/v1/messages", model.base_url.trim_end_matches('/'));
        tracing::info!("Model request: POST {} model={} messages={}", url, params.model, params.messages.len());
        let response = match HTTP_CLIENT.post(&url).header("x-api-key", options.api_key.clone().unwrap_or_default()).header("anthropic-version", "2023-06-01").header("content-type", "application/json").json(&params).send().await {
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

        // 累积中的内容块(对齐 pi 的 Block: 内容块 + index + partialJson 暂存)
        struct Block {
            index: u32,
            content: AssistantContent,
            partial_json: String,
        }
        let mut blocks: Vec<Block> = Vec::new();
        // partial 快照(对齐 pi: output.content 与 blocks 为同一引用, 这里重建等价快照)
        let partial = |output: &AssistantMessage, blocks: &[Block]| -> AssistantMessage {
            let mut p = output.clone();
            p.content = blocks.iter().map(|b| b.content.clone()).collect();
            p
        };
        // 推入新内容块并发送对应 Start 事件(对齐 pi: 块创建即推流)
        let push_block = |output: &AssistantMessage, blocks: &mut Vec<Block>, index: u32, content: AssistantContent| {
            blocks.push(Block { index, content, partial_json: String::new() });
            let content_index = blocks.len() - 1;
            let snapshot = partial(output, blocks);
            let event = match &blocks[content_index].content {
                AssistantContent::Text(_) => AssistantMessageEvent::TextStart { content_index, partial: snapshot },
                AssistantContent::Thinking(_) => AssistantMessageEvent::ThinkingStart { content_index, partial: snapshot },
                AssistantContent::ToolCall(_) => AssistantMessageEvent::ToolcallStart { content_index, partial: snapshot },
            };
            let _ = tx.send(event);
        };

        let byte_stream = response.bytes_stream();
        let mut event_stream = Box::pin(byte_stream.eventsource());
        while let Some(result) = event_stream.next().await {
            let sse = match result {
                Ok(e) => e,
                Err(e) => fail!(format!("SSE 解析失败: {}", e)),
            };
            // 对齐 pi: 事件数据经 repair 后解析, 仍失败则终止流(不静默跳过, 避免丢失内容块)
            let event = match parse_json_with_repair(&sse.data).and_then(serde_json::from_value::<MessageStreamEvent>) {
                Ok(evt) => evt,
                Err(e) => fail!(format!("无法解析 Anthropic SSE 事件: {}; data={}", e, sse.data)),
            };
            match event {
                MessageStreamEvent::MessageStart { message } => {
                    output.response_id = Some(message.id);
                    // 回填实际响应模型(对齐 pi: output.model = event.message.model), 影响跨模型重放的同模型判断
                    if !message.model.is_empty() {
                        output.model = message.model;
                    }
                    // 捕获初始 token 用量, 保证流提前中断时也有输入 token 计数
                    output.usage.input = message.usage.input_tokens;
                    output.usage.output = message.usage.output_tokens;
                    output.usage.cache_read = message.usage.cache_read_input_tokens.unwrap_or(0);
                    output.usage.cache_write = message.usage.cache_creation_input_tokens.unwrap_or(0);
                    // Anthropic 不提供 total_tokens, 由各分量计算
                    output.usage.total_tokens = output.usage.input + output.usage.output + output.usage.cache_read + output.usage.cache_write;
                }
                MessageStreamEvent::ContentBlockStart { index, content_block } => {
                    let content = match content_block {
                        ContentBlock::Text { text } => AssistantContent::Text(TextContent { text, text_signature: None }),
                        ContentBlock::Thinking { thinking, signature } => AssistantContent::Thinking(ThinkingContent { thinking, thinking_signature: Some(signature), redacted: None }),
                        ContentBlock::RedactedThinking { data } => AssistantContent::Thinking(ThinkingContent { thinking: "[Reasoning redacted]".to_string(), thinking_signature: Some(data), redacted: Some(true) }),
                        ContentBlock::ToolUse { id, name, input } => AssistantContent::ToolCall(ToolCall { id, name, arguments: input, thought_signature: None }),
                    };
                    push_block(&output, &mut blocks, index, content);
                }
                MessageStreamEvent::ContentBlockDelta { index, delta } => {
                    let Some(pos) = blocks.iter().position(|b| b.index == index) else {
                        continue;
                    };
                    match delta {
                        ContentBlockDelta::TextDelta { text } => {
                            if let AssistantContent::Text(t) = &mut blocks[pos].content {
                                t.text.push_str(&text);
                                let _ = tx.send(AssistantMessageEvent::TextDelta { content_index: pos, delta: text, partial: partial(&output, &blocks) });
                            }
                        }
                        ContentBlockDelta::ThinkingDelta { thinking } => {
                            if let AssistantContent::Thinking(t) = &mut blocks[pos].content {
                                t.thinking.push_str(&thinking);
                                let _ = tx.send(AssistantMessageEvent::ThinkingDelta { content_index: pos, delta: thinking, partial: partial(&output, &blocks) });
                            }
                        }
                        ContentBlockDelta::InputJsonDelta { partial_json } => {
                            if matches!(blocks[pos].content, AssistantContent::ToolCall(_)) {
                                blocks[pos].partial_json.push_str(&partial_json);
                                let arguments = parse_streaming_json(&blocks[pos].partial_json);
                                if let AssistantContent::ToolCall(tc) = &mut blocks[pos].content {
                                    tc.arguments = arguments;
                                }
                                let _ = tx.send(AssistantMessageEvent::ToolcallDelta { content_index: pos, delta: partial_json, partial: partial(&output, &blocks) });
                            }
                        }
                        ContentBlockDelta::SignatureDelta { signature } => {
                            if let AssistantContent::Thinking(t) = &mut blocks[pos].content {
                                t.thinking_signature.get_or_insert_with(String::new).push_str(&signature);
                            }
                        }
                    }
                }
                MessageStreamEvent::ContentBlockStop { index } => {
                    let Some(pos) = blocks.iter().position(|b| b.index == index) else {
                        continue;
                    };
                    // toolCall 结束时以完整 partialJson 重解析参数(对齐 pi)
                    if matches!(blocks[pos].content, AssistantContent::ToolCall(_)) {
                        let arguments = parse_streaming_json(&blocks[pos].partial_json);
                        if let AssistantContent::ToolCall(tc) = &mut blocks[pos].content {
                            tc.arguments = arguments;
                        }
                    }
                    let snapshot = partial(&output, &blocks);
                    match &blocks[pos].content {
                        AssistantContent::Text(t) => {
                            let _ = tx.send(AssistantMessageEvent::TextEnd { content_index: pos, content: t.text.clone(), partial: snapshot });
                        }
                        AssistantContent::Thinking(t) => {
                            let _ = tx.send(AssistantMessageEvent::ThinkingEnd { content_index: pos, content: t.thinking.clone(), partial: snapshot });
                        }
                        AssistantContent::ToolCall(tc) => {
                            let _ = tx.send(AssistantMessageEvent::ToolcallEnd { content_index: pos, tool_call: tc.clone(), partial: snapshot });
                        }
                    }
                }
                MessageStreamEvent::MessageDelta { delta, usage } => {
                    if let Some(stop_reason) = delta.stop_reason {
                        output.raw_stop_reason = Some(stop_reason.clone());
                        match map_stop_reason(&stop_reason, delta.stop_details.as_ref()) {
                            Ok((mapped, error_message)) => {
                                output.stop_reason = mapped;
                                output.error_message = error_message;
                            }
                            Err(e) => fail!(e),
                        }
                    }
                    // 仅更新存在的字段(对齐 pi 的 != null 检查: 部分代理在 message_delta 中省略字段)
                    if let Some(usage) = usage {
                        if let Some(input_tokens) = usage.input_tokens {
                            output.usage.input = input_tokens;
                        }
                        if let Some(output_tokens) = usage.output_tokens {
                            output.usage.output = output_tokens;
                        }
                        if let Some(cache_read) = usage.cache_read_input_tokens {
                            output.usage.cache_read = cache_read;
                        }
                        if let Some(cache_write) = usage.cache_creation_input_tokens {
                            output.usage.cache_write = cache_write;
                        }
                        // 对齐 pi: 终态 message_delta 上报的 thinking_tokens 计入 reasoning(output 的子集)
                        if let Some(thinking_tokens) = usage.output_tokens_details.as_ref().and_then(|d| d.thinking_tokens) {
                            output.usage.reasoning = Some(thinking_tokens);
                        }
                    }
                    // Anthropic 不提供 total_tokens, 由各分量计算
                    output.usage.total_tokens = output.usage.input + output.usage.output + output.usage.cache_read + output.usage.cache_write;
                }
                MessageStreamEvent::MessageStop => {}
                // ping 保活与未知事件静默忽略(对齐 pi: 未知事件不中断流)
                MessageStreamEvent::Ping | MessageStreamEvent::Unknown => {}
                // 流中途 error 事件: 以真实错误信息终止, 避免笼统的 stop reason 缺失报错
                MessageStreamEvent::Error { error } => {
                    fail!(format!("{}: {}", error.error_type, error.message));
                }
            }
        }

        // 对齐 pi: 流结束后校验停止原因
        if output.stop_reason == StopReason::Pending {
            fail!("Anthropic stream ended without a stop reason".to_string());
        }
        if output.stop_reason == StopReason::Aborted || output.stop_reason == StopReason::Error {
            fail!(output.error_message.clone().unwrap_or_else(|| "An unknown error occurred".to_string()));
        }
        // Done 事件携带完整消息(对齐 pi: blocks 即 output.content 同一引用, 此处用快照补齐 content)
        let _ = tx.send(AssistantMessageEvent::Done { reason: output.stop_reason, message: partial(&output, &blocks) });
    });
    // tokio mpsc 接收端包装为 Stream
    Box::pin(futures_util::stream::poll_fn(move |cx| rx.poll_recv(cx)))
}

// 获取可用模型列表, 返回模型 id 列表
pub async fn list_models(base_url: &str, api_key: &str) -> Result<Vec<String>, AnthropicError> {
    let url = format!("{}/v1/models?limit=100", base_url.trim_end_matches('/'));
    tracing::info!("Model request: GET {}", url);
    let response = HTTP_CLIENT.get(&url).header("x-api-key", api_key).header("anthropic-version", "2023-06-01").send().await?;

    let status = response.status();
    if !status.is_success() {
        let body = response.text().await.unwrap_or_default();
        // 日志中的 body 截断至 500 字符, 错误对象保留完整内容
        tracing::error!("Model API error: status={} body={}", status.as_u16(), truncate_str(&body, 500));
        return Err(AnthropicError::Api { status: status.as_u16(), body });
    }

    let list = response.json::<ListModelsResponse>().await?;
    Ok(list.data.into_iter().map(|m| m.id).collect())
}
