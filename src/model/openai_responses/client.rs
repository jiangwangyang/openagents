// OpenAI Responses 流式客户端 + 基准协议(Anthropic)双向适配
use eventsource_stream::Eventsource;
use futures_util::{Stream, StreamExt};
use reqwest::Client;
use serde_json::{json, Value};
use std::pin::Pin;

use crate::model::anthropic::types::{ContentBlock, ContentBlockDelta, CreateMessageRequest, ListModelsResponse, Message, MessageDelta, MessageDeltaUsage, MessageStreamEvent, ThinkingConfig, Usage};

use super::types::{CreateResponseRequest, OutputItem, ReasoningConfig, ResponseStreamEvent};

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
    #[error("响应失败: {0}")]
    Failed(String),
}

// 转换后的基准协议(Anthropic)事件流
pub type CanonicalEventStream = Pin<Box<dyn Stream<Item = Result<MessageStreamEvent, OpenAiResponsesError>> + Send>>;

// 共享 HTTP 客户端(内部为 Arc，克隆仅复制引用，全应用复用同一连接池)
static HTTP_CLIENT: std::sync::LazyLock<Client> = std::sync::LazyLock::new(Client::new);

// 流式创建响应,输入输出均为基准协议(Anthropic)类型,协议差异在本函数内完成转换
pub async fn create_message_stream(
    base_url: &str,
    api_key: &str,
    request: &CreateMessageRequest,
) -> Result<CanonicalEventStream, OpenAiResponsesError> {
    // ===== 出站转换: 基准协议请求 -> Responses 请求 =====
    // 基准消息列表 -> Responses input items
    let mut input: Vec<Value> = Vec::new();
    for msg in &request.messages {
        let role = msg.role.as_str();
        let text_type = if role == "assistant" { "output_text" } else { "input_text" };
        let mut message_content: Vec<Value> = Vec::new();
        // 累积中的文本块先落为 message item,保证 item 顺序与块顺序一致
        let flush_message = |input: &mut Vec<Value>, message_content: &mut Vec<Value>| {
            if !message_content.is_empty() {
                input.push(json!({"type": "message", "role": role, "content": std::mem::take(message_content)}));
            }
        };
        match &msg.content {
            Value::String(text) => {
                message_content.push(json!({"type": text_type, "text": text}));
            }
            Value::Array(blocks) => {
                for block in blocks {
                    match block["type"].as_str() {
                        Some("text") => {
                            message_content.push(json!({"type": text_type, "text": block["text"].as_str().unwrap_or("")}));
                        }
                        Some("thinking") => {
                            // thinking 块 -> reasoning item, signature 回传为 encrypted_content
                            flush_message(&mut input, &mut message_content);
                            input.push(json!({
                                "type": "reasoning",
                                "summary": [],
                                "encrypted_content": block["signature"].as_str().unwrap_or(""),
                            }));
                        }
                        Some("tool_use") => {
                            flush_message(&mut input, &mut message_content);
                            input.push(json!({
                                "type": "function_call",
                                "call_id": block["id"].as_str().unwrap_or(""),
                                "name": block["name"].as_str().unwrap_or(""),
                                "arguments": serde_json::to_string(&block["input"]).unwrap_or_default(),
                            }));
                        }
                        Some("tool_result") => {
                            flush_message(&mut input, &mut message_content);
                            input.push(json!({
                                "type": "function_call_output",
                                "call_id": block["tool_use_id"].as_str().unwrap_or(""),
                                "output": block["content"].as_str().unwrap_or(""),
                            }));
                        }
                        // redacted_thinking 等无法表达的块直接丢弃
                        _ => {}
                    }
                }
            }
            _ => {}
        }
        flush_message(&mut input, &mut message_content);
    }
    // 工具定义: input_schema -> parameters
    let tools = request.tools.as_ref().map(|tools| {
        tools.iter().map(|t| json!({
            "type": "function",
            "name": t["name"],
            "description": t["description"],
            "parameters": t["input_schema"],
        })).collect::<Vec<Value>>()
    });
    let responses_request = CreateResponseRequest {
        model: request.model.clone(),
        instructions: request.system.clone(),
        input,
        tools,
        reasoning: match &request.thinking {
            Some(ThinkingConfig::Enabled { .. }) => Some(ReasoningConfig { effort: "high".to_string(), summary: "auto".to_string() }),
            _ => None,
        },
        max_output_tokens: request.max_tokens,
        stream: true,
        // stateless 模式: 历史由本地管理,每次全量发送; encrypted_content 用于思考内容跨轮回传
        store: false,
        include: vec!["reasoning.encrypted_content".to_string()],
    };

    let url = format!("{}/responses", base_url.trim_end_matches('/'));
    let response = HTTP_CLIENT
        .post(&url)
        .header("authorization", format!("Bearer {}", api_key))
        .header("content-type", "application/json")
        .json(&responses_request)
        .send()
        .await?;

    let status = response.status();
    if !status.is_success() {
        let body = response.text().await.unwrap_or_default();
        return Err(OpenAiResponsesError::Api { status: status.as_u16(), body });
    }

    // bytes_stream -> eventsource-stream -> 反序列化为 ResponseStreamEvent
    let byte_stream = response.bytes_stream();
    let event_stream = byte_stream.eventsource();
    let parsed_stream = event_stream.filter_map(|result| {
        std::future::ready(match result {
            Ok(event) => match serde_json::from_str::<ResponseStreamEvent>(&event.data) {
                Ok(evt) => Some(Ok(evt)),
                Err(e) => {
                    tracing::warn!("未知 SSE 事件: {} error={}", event.data, e);
                    None
                }
            },
            Err(e) => Some(Err(OpenAiResponsesError::Sse(e.to_string()))),
        })
    });

    // ===== 入站转换: Responses 事件 -> 基准协议事件(单个事件可能展开为多个基准事件) =====
    // 记录已收到增量参数的 function_call item,避免 done 事件重复补发完整参数
    let delta_items = std::sync::Arc::new(std::sync::Mutex::new(std::collections::HashSet::<String>::new()));
    let stream = parsed_stream.flat_map(move |result| {
        let events: Vec<Result<MessageStreamEvent, OpenAiResponsesError>> = match result {
            Err(e) => vec![Err(e)],
            Ok(evt) => match evt {
                ResponseStreamEvent::ResponseCreated { response } => vec![Ok(MessageStreamEvent::MessageStart {
                    message: Message {
                        id: response.id,
                        msg_type: "message".to_string(),
                        role: "assistant".to_string(),
                        model: response.model.unwrap_or_default(),
                        content: vec![],
                        stop_reason: None,
                        stop_sequence: None,
                        usage: Usage { input_tokens: 0, output_tokens: 0, cache_creation_input_tokens: None, cache_read_input_tokens: None },
                    },
                })],
                ResponseStreamEvent::OutputItemAdded { output_index, item } => match item {
                    OutputItem::Reasoning { .. } => vec![Ok(MessageStreamEvent::ContentBlockStart {
                        index: output_index,
                        content_block: ContentBlock::Thinking { thinking: String::new(), signature: String::new() },
                    })],
                    OutputItem::Message {} => vec![Ok(MessageStreamEvent::ContentBlockStart {
                        index: output_index,
                        content_block: ContentBlock::Text { text: String::new() },
                    })],
                    OutputItem::FunctionCall { call_id, name, .. } => vec![Ok(MessageStreamEvent::ContentBlockStart {
                        index: output_index,
                        content_block: ContentBlock::ToolUse { id: call_id, name, input: json!({}) },
                    })],
                },
                ResponseStreamEvent::OutputTextDelta { output_index, delta } => vec![Ok(MessageStreamEvent::ContentBlockDelta {
                    index: output_index,
                    delta: ContentBlockDelta::TextDelta { text: delta },
                })],
                // reasoning 摘要对齐基准协议的 thinking 内容
                ResponseStreamEvent::ReasoningSummaryTextDelta { output_index, delta } => vec![Ok(MessageStreamEvent::ContentBlockDelta {
                    index: output_index,
                    delta: ContentBlockDelta::ThinkingDelta { thinking: delta },
                })],
                ResponseStreamEvent::FunctionCallArgumentsDelta { item_id, output_index, delta } => {
                    delta_items.lock().unwrap().insert(item_id);
                    vec![Ok(MessageStreamEvent::ContentBlockDelta {
                        index: output_index,
                        delta: ContentBlockDelta::InputJsonDelta { partial_json: delta },
                    })]
                }
                ResponseStreamEvent::OutputItemDone { output_index, item } => match item {
                    // encrypted_content 在 done 时一次性下发,对齐为 signature 增量
                    OutputItem::Reasoning { encrypted_content } => {
                        let mut evts = Vec::new();
                        if let Some(encrypted) = encrypted_content {
                            evts.push(Ok(MessageStreamEvent::ContentBlockDelta {
                                index: output_index,
                                delta: ContentBlockDelta::SignatureDelta { signature: encrypted },
                            }));
                        }
                        evts.push(Ok(MessageStreamEvent::ContentBlockStop { index: output_index }));
                        evts
                    }
                    // 部分供应商不流式下发参数增量,仅在 done 携带完整参数,此时补发一条增量
                    OutputItem::FunctionCall { id, arguments, .. } => {
                        let mut evts = Vec::new();
                        let item_id = id.unwrap_or_default();
                        let has_delta = delta_items.lock().unwrap().contains(&item_id);
                        if !has_delta {
                            if let Some(arguments) = arguments {
                                if !arguments.is_empty() {
                                    evts.push(Ok(MessageStreamEvent::ContentBlockDelta {
                                        index: output_index,
                                        delta: ContentBlockDelta::InputJsonDelta { partial_json: arguments },
                                    }));
                                }
                            }
                        }
                        evts.push(Ok(MessageStreamEvent::ContentBlockStop { index: output_index }));
                        evts
                    }
                    OutputItem::Message {} => vec![Ok(MessageStreamEvent::ContentBlockStop { index: output_index })],
                },
                ResponseStreamEvent::ResponseCompleted { response } | ResponseStreamEvent::ResponseIncomplete { response } => {
                    let stop_reason = if response.status.as_deref() == Some("incomplete") { "max_tokens" } else { "end_turn" };
                    let (input_tokens, output_tokens) = response.usage.map(|u| (u.input_tokens, u.output_tokens)).unwrap_or((0, 0));
                    vec![
                        Ok(MessageStreamEvent::MessageDelta {
                            delta: MessageDelta { stop_reason: Some(stop_reason.to_string()), stop_sequence: None },
                            usage: MessageDeltaUsage {
                                output_tokens,
                                cache_creation_input_tokens: None,
                                cache_read_input_tokens: None,
                                input_tokens: Some(input_tokens),
                            },
                        }),
                        Ok(MessageStreamEvent::MessageStop),
                    ]
                }
                ResponseStreamEvent::ResponseFailed { response } => vec![Err(OpenAiResponsesError::Failed(response.error.map(|e| e.to_string()).unwrap_or_default()))],
                ResponseStreamEvent::Error { message } => vec![Err(OpenAiResponsesError::Failed(message.unwrap_or_default()))],
            },
        };
        futures_util::stream::iter(events)
    });

    Ok(Box::pin(stream))
}

// 获取可用模型列表(GET {base_url}/models),返回模型 id 列表
pub async fn list_models(base_url: &str, api_key: &str) -> Result<Vec<String>, OpenAiResponsesError> {
    let url = format!("{}/models", base_url.trim_end_matches('/'));
    let response = HTTP_CLIENT
        .get(&url)
        .header("authorization", format!("Bearer {}", api_key))
        .send()
        .await?;

    let status = response.status();
    if !status.is_success() {
        let body = response.text().await.unwrap_or_default();
        return Err(OpenAiResponsesError::Api { status: status.as_u16(), body });
    }

    let list = response.json::<ListModelsResponse>().await?;
    Ok(list.data.into_iter().map(|m| m.id).collect())
}
