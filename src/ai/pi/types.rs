// pi 消息协议类型(移植自 pi/packages/ai/src/types.ts 的消息相关子集)
// 对齐说明: 结构体/字段与 pi 一一对应, Rust 侧 snake_case 字段经 serde camelCase 序列化后与 pi 的 JSON 完全一致,
// 未迁移字段(diagnostics/deferred/endTurn/namespace/addedToolNames 等)在对应结构体注释中说明
use serde::{Deserialize, Serialize};

// Unix 毫秒时间戳(对齐 pi 的 Date.now())
pub fn now_timestamp() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}

// ========== 内容块 ==========

// 文本内容块
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TextContent {
    pub text: String,
    // OpenAI Responses 消息元数据(旧版 id 字符串或 TextSignatureV1 JSON)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub text_signature: Option<String>,
}

// 思考内容块
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ThinkingContent {
    pub thinking: String,
    // OpenAI Responses 的 reasoning item JSON / Anthropic 的 signature
    #[serde(skip_serializing_if = "Option::is_none")]
    pub thinking_signature: Option<String>,
    // 被安全过滤打码的思考内容, 加密载荷存于 thinkingSignature 用于跨轮回传
    #[serde(skip_serializing_if = "Option::is_none")]
    pub redacted: Option<bool>,
}

// 图片内容块(仅保留类型对齐, 当前流程不使用)
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ImageContent {
    pub data: String,      // base64 编码的图片数据
    pub mime_type: String, // 如 "image/jpeg", "image/png"
}

// 工具调用块(未迁移字段: namespace, 本项目无 OpenAI 自定义工具)
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ToolCall {
    pub id: String,
    pub name: String,
    pub arguments: serde_json::Value,
    // Google 专用思考签名(仅类型对齐, 当前流程不使用)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub thought_signature: Option<String>,
}

// 用户/工具结果内容块(TextContent | ImageContent)
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum UserContent {
    #[serde(rename = "text")]
    Text(TextContent),
    #[serde(rename = "image")]
    Image(ImageContent),
}

// 助手内容块(TextContent | ThinkingContent | ToolCall)
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum AssistantContent {
    #[serde(rename = "text")]
    Text(TextContent),
    #[serde(rename = "thinking")]
    Thinking(ThinkingContent),
    #[serde(rename = "toolCall")]
    ToolCall(ToolCall),
}

// ========== 使用量 ==========

// 费用(pi 按模型费率计算, 本项目不维护费率表, 恒为 0)
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UsageCost {
    pub input: f64,
    pub output: f64,
    pub cache_read: f64,
    pub cache_write: f64,
    pub total: f64,
}

// Token 使用量(未迁移字段: cacheWrite1h, 仅 Anthropic 上报的 1h 缓存细分)
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Usage {
    pub input: u64,
    pub output: u64,
    pub cache_read: u64,
    pub cache_write: u64,
    // 思考 token(部分供应商上报, 为 output 的子集)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reasoning: Option<u64>,
    pub total_tokens: u64,
    pub cost: UsageCost,
}

// 停止原因(未迁移值: deferred, 本项目无延迟响应)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum StopReason {
    #[serde(rename = "pending")]
    Pending,
    #[serde(rename = "stop")]
    Stop,
    #[serde(rename = "length")]
    Length,
    #[serde(rename = "toolUse")]
    ToolUse,
    #[serde(rename = "error")]
    Error,
    #[serde(rename = "aborted")]
    Aborted,
}

// ========== 消息 ==========

// 用户消息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserMessage {
    pub content: UserMessageContent,
    pub timestamp: u64, // Unix 毫秒时间戳
}

// 用户消息内容(string | (TextContent | ImageContent)[])
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum UserMessageContent {
    Text(String),
    Blocks(Vec<UserContent>),
}

// 助手消息(未迁移字段: diagnostics/deferred/endTurn)
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AssistantMessage {
    pub content: Vec<AssistantContent>,
    pub api: String,
    pub provider: String,
    pub model: String,
    // 实际响应模型(与请求模型不同时上报, 如 OpenRouter auto)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub response_model: Option<String>,
    // 供应商响应/消息标识
    #[serde(skip_serializing_if = "Option::is_none")]
    pub response_id: Option<String>,
    pub usage: Usage,
    pub stop_reason: StopReason,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error_message: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub raw_stop_reason: Option<String>,
    pub timestamp: u64, // Unix 毫秒时间戳
}

// 工具结果消息(未迁移字段: details/usage/addedToolNames, 本项目无延迟加载工具)
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ToolResultMessage {
    pub tool_call_id: String,
    pub tool_name: String,
    pub content: Vec<UserContent>, // 支持文本与图片
    pub is_error: bool,
    pub timestamp: u64, // Unix 毫秒时间戳
}

// 消息(UserMessage | AssistantMessage | ToolResultMessage)
#[allow(clippy::large_enum_variant)]
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "role")]
pub enum Message {
    #[serde(rename = "user")]
    User(UserMessage),
    #[serde(rename = "assistant")]
    Assistant(AssistantMessage),
    #[serde(rename = "toolResult")]
    ToolResult(ToolResultMessage),
}

// ========== 上下文 ==========

// 工具定义(未迁移字段: constrainedSampling, 本项目无约束采样)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Tool {
    pub name: String,
    pub description: String,
    pub parameters: serde_json::Value,
}

// 模型调用上下文
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Context {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub system_prompt: Option<String>,
    pub messages: Vec<Message>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tools: Option<Vec<Tool>>,
}

// 模型(pi Model 的裁剪版, 仅保留消息转换所需字段)
#[derive(Debug, Clone)]
pub struct Model {
    pub id: String,
    pub api: String,
    pub provider: String,
    pub base_url: String,
    pub reasoning: bool,
    pub input: Vec<String>, // "text" | "image"
    pub max_tokens: u32,
}

// ========== 流事件 ==========

// 助手消息流事件(pi AssistantMessageEvent, 变体名对应 start/text_delta/toolcall_end 等 type 值)
// 对齐说明: 与 pi 一致, 每个事件携带完整 partial 消息; 流以 done(成功)或 error(失败)事件终止;
// 消费方按需读取字段, 未读字段保留用于与 pi 对齐
#[allow(dead_code)]
#[derive(Debug, Clone)]
pub enum AssistantMessageEvent {
    Start {
        partial: AssistantMessage,
    },
    TextStart {
        content_index: usize,
        partial: AssistantMessage,
    },
    TextDelta {
        content_index: usize,
        delta: String,
        partial: AssistantMessage,
    },
    TextEnd {
        content_index: usize,
        content: String,
        partial: AssistantMessage,
    },
    ThinkingStart {
        content_index: usize,
        partial: AssistantMessage,
    },
    ThinkingDelta {
        content_index: usize,
        delta: String,
        partial: AssistantMessage,
    },
    ThinkingEnd {
        content_index: usize,
        content: String,
        partial: AssistantMessage,
    },
    ToolcallStart {
        content_index: usize,
        partial: AssistantMessage,
    },
    ToolcallDelta {
        content_index: usize,
        delta: String,
        partial: AssistantMessage,
    },
    ToolcallEnd {
        content_index: usize,
        tool_call: ToolCall,
        partial: AssistantMessage,
    },
    Done {
        reason: StopReason,
        message: AssistantMessage,
    },
    Error {
        reason: StopReason,
        error: AssistantMessage,
    },
}
