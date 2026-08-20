// 消息跨模型转换(移植自 pi/packages/ai/src/api/transform-messages.ts)
// 裁剪说明: 本项目不使用图片, 未移植 downgradeUnsupportedImages 图片降级与 replaceImagesWithPlaceholder
use std::collections::{HashMap, HashSet};

use super::types::{now_timestamp, AssistantContent, AssistantMessage, Message, Model, StopReason, TextContent, ToolCall, ToolResultMessage, UserContent};

// toolCallId 规范化回调(由各协议适配器注入, 对齐 pi 的 normalizeToolCallId 参数)
pub type NormalizeToolCallId<'a> = Option<&'a dyn Fn(&str, &Model, &AssistantMessage) -> String>;

// 跨模型重放转换: thinking 降级/丢弃、toolCallId 规范化、孤儿 toolCall 补合成 toolResult、跳过失败消息
pub fn transform_messages(messages: &[Message], model: &Model, normalize_tool_call_id: NormalizeToolCallId) -> Vec<Message> {
    // 原始 toolCallId -> 规范化 id 的映射
    let mut tool_call_id_map: HashMap<String, String> = HashMap::new();

    // 第一遍: 转换消息(thinking 块处理、toolCallId 规范化)
    let transformed: Vec<Message> = messages
        .iter()
        .map(|msg| match msg {
            // 用户消息原样透传
            Message::User(_) => msg.clone(),
            Message::ToolResult(tool_result) => {
                // 存在映射时规范化 toolCallId
                if let Some(normalized_id) = tool_call_id_map.get(&tool_result.tool_call_id) {
                    if normalized_id != &tool_result.tool_call_id {
                        let mut mapped = tool_result.clone();
                        mapped.tool_call_id = normalized_id.clone();
                        return Message::ToolResult(mapped);
                    }
                }
                msg.clone()
            }
            Message::Assistant(assistant_msg) => {
                let is_same_model = assistant_msg.provider == model.provider && assistant_msg.api == model.api && assistant_msg.model == model.id;
                let mut transformed_content: Vec<AssistantContent> = Vec::new();
                for block in &assistant_msg.content {
                    match block {
                        AssistantContent::Thinking(thinking) => {
                            // redacted thinking 为不透明的加密内容, 仅对同一模型有效, 跨模型丢弃避免 API 报错
                            if thinking.redacted == Some(true) {
                                if is_same_model {
                                    transformed_content.push(block.clone());
                                }
                                continue;
                            }
                            // 同模型保留带签名的 thinking 块(重放需要), 即使思考文本为空(OpenAI 加密 reasoning)
                            if is_same_model && thinking.thinking_signature.is_some() {
                                transformed_content.push(block.clone());
                                continue;
                            }
                            // 空 thinking 块跳过, 跨模型的非空 thinking 降级为纯文本
                            if thinking.thinking.trim().is_empty() {
                                continue;
                            }
                            if is_same_model {
                                transformed_content.push(block.clone());
                                continue;
                            }
                            transformed_content.push(AssistantContent::Text(TextContent { text: thinking.thinking.clone(), text_signature: None }));
                        }
                        AssistantContent::Text(t) => {
                            // 对齐 pi: 同模型原样保留, 跨模型重建文本块丢弃 textSignature(旧模型的 msg id 不回放)
                            if is_same_model {
                                transformed_content.push(block.clone());
                            } else {
                                transformed_content.push(AssistantContent::Text(TextContent { text: t.text.clone(), text_signature: None }));
                            }
                        }
                        AssistantContent::ToolCall(tool_call) => {
                            let mut normalized_tool_call = tool_call.clone();
                            if !is_same_model && normalized_tool_call.thought_signature.is_some() {
                                normalized_tool_call.thought_signature = None;
                            }
                            if !is_same_model {
                                if let Some(normalize) = normalize_tool_call_id {
                                    let normalized_id = normalize(&tool_call.id, model, assistant_msg);
                                    if normalized_id != tool_call.id {
                                        tool_call_id_map.insert(tool_call.id.clone(), normalized_id.clone());
                                        normalized_tool_call.id = normalized_id;
                                    }
                                }
                            }
                            transformed_content.push(AssistantContent::ToolCall(normalized_tool_call));
                        }
                    }
                }
                let mut mapped = assistant_msg.clone();
                mapped.content = transformed_content;
                Message::Assistant(mapped)
            }
        })
        .collect();

    // 第二遍: 为孤儿 toolCall 插入合成空 toolResult(保留 thinking 签名并满足 API 配对要求)
    let mut result: Vec<Message> = Vec::new();
    let mut pending_tool_calls: Vec<ToolCall> = Vec::new();
    let mut existing_tool_result_ids: HashSet<String> = HashSet::new();
    // 对齐 pi 的 insertSyntheticToolResults 闭包
    macro_rules! insert_synthetic_tool_results {
        () => {
            if !pending_tool_calls.is_empty() {
                for tc in pending_tool_calls.drain(..) {
                    if !existing_tool_result_ids.contains(&tc.id) {
                        result.push(Message::ToolResult(ToolResultMessage { tool_call_id: tc.id.clone(), tool_name: tc.name.clone(), content: vec![UserContent::Text(TextContent { text: "No result provided".to_string(), text_signature: None })], is_error: true, timestamp: now_timestamp() }));
                    }
                }
                pending_tool_calls.clear();
                existing_tool_result_ids.clear();
            }
        };
    }

    for msg in transformed {
        match &msg {
            Message::Assistant(assistant_msg) => {
                // 前一条 assistant 遗留孤儿 toolCall 时先补合成结果
                insert_synthetic_tool_results!();
                // 跳过 error/aborted 的 assistant 消息: 不完整轮次不应重放(可能含半截内容导致 API 报错)
                if assistant_msg.stop_reason == StopReason::Error || assistant_msg.stop_reason == StopReason::Aborted {
                    continue;
                }
                // 跟踪该 assistant 消息的 toolCall
                let tool_calls: Vec<ToolCall> = assistant_msg
                    .content
                    .iter()
                    .filter_map(|b| match b {
                        AssistantContent::ToolCall(tc) => Some(tc.clone()),
                        _ => None,
                    })
                    .collect();
                if !tool_calls.is_empty() {
                    pending_tool_calls = tool_calls;
                    existing_tool_result_ids.clear();
                }
                result.push(msg.clone());
            }
            Message::ToolResult(tool_result) => {
                existing_tool_result_ids.insert(tool_result.tool_call_id.clone());
                result.push(msg.clone());
            }
            Message::User(_) => {
                // 用户消息打断工具流程, 先补孤儿 toolCall 的合成结果
                insert_synthetic_tool_results!();
                result.push(msg.clone());
            }
        }
    }

    // 对话以未闭环的 toolCall 结尾时, 立即补合成结果
    insert_synthetic_tool_results!();

    result
}
