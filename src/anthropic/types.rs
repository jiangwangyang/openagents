// Anthropic 流式 API 类型建模
use serde::{Deserialize, Serialize};

// ========== 请求体 ==========

// 创建消息请求
#[derive(Debug, Serialize)]
pub struct CreateMessageRequest {
    pub model: String,
    pub messages: Vec<RequestMessage>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub system: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tools: Option<Vec<serde_json::Value>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub thinking: Option<ThinkingConfig>,
    pub max_tokens: u32,
    pub stream: bool,
}

// 请求消息
#[derive(Debug, Serialize)]
pub struct RequestMessage {
    pub role: String,
    pub content: serde_json::Value,
}

// Thinking 配置
#[derive(Debug, Clone, Serialize)]
#[serde(tag = "type")]
pub enum ThinkingConfig {
    #[serde(rename = "enabled")]
    Enabled { display: String },
    #[serde(rename = "disabled")]
    Disabled,
}

// ========== 响应消息 ==========

// 完整消息(message_start 事件中的 message)
#[allow(dead_code)]
#[derive(Debug, Clone, Deserialize)]
pub struct Message {
    pub id: String,
    #[serde(rename = "type")]
    pub msg_type: String,
    pub role: String,
    pub model: String,
    pub content: Vec<ContentBlock>,
    pub stop_reason: Option<String>,
    pub stop_sequence: Option<String>,
    pub usage: Usage,
}

// 内容块(响应中的 content block)
#[derive(Debug, Clone, Deserialize)]
#[serde(tag = "type")]
pub enum ContentBlock {
    #[serde(rename = "thinking")]
    Thinking { thinking: String, signature: String },
    #[serde(rename = "redacted_thinking")]
    RedactedThinking { data: String },
    #[serde(rename = "text")]
    Text { text: String },
    #[serde(rename = "tool_use")]
    ToolUse {
        id: String,
        name: String,
        input: serde_json::Value,
    },
}

// Token 使用量(message_start 中的 usage)
#[allow(dead_code)]
#[derive(Debug, Clone, Deserialize)]
pub struct Usage {
    pub input_tokens: u64,
    pub output_tokens: u64,
    pub cache_creation_input_tokens: Option<u64>,
    pub cache_read_input_tokens: Option<u64>,
}

// message_delta 中的 usage
#[allow(dead_code)]
#[derive(Debug, Clone, Deserialize)]
pub struct MessageDeltaUsage {
    pub output_tokens: u64,
    pub cache_creation_input_tokens: Option<u64>,
    pub cache_read_input_tokens: Option<u64>,
    pub input_tokens: Option<u64>,
}

// ========== 流事件 ==========

// 流式事件(对应 Python RawMessageStreamEvent)
#[allow(dead_code)]
#[derive(Debug, Clone, Deserialize)]
#[serde(tag = "type")]
pub enum MessageStreamEvent {
    #[serde(rename = "message_start")]
    MessageStart { message: Message },
    #[serde(rename = "message_delta")]
    MessageDelta { delta: MessageDelta, usage: MessageDeltaUsage },
    #[serde(rename = "message_stop")]
    MessageStop,
    #[serde(rename = "content_block_start")]
    ContentBlockStart {
        index: u32,
        content_block: ContentBlock,
    },
    #[serde(rename = "content_block_delta")]
    ContentBlockDelta { index: u32, delta: ContentBlockDelta },
    #[serde(rename = "content_block_stop")]
    ContentBlockStop { index: u32 },
}

// message_delta 事件中的 delta
#[allow(dead_code)]
#[derive(Debug, Clone, Deserialize)]
pub struct MessageDelta {
    pub stop_reason: Option<String>,
    pub stop_sequence: Option<String>,
}

// content_block_delta 事件中的 delta
#[derive(Debug, Clone, Deserialize)]
#[serde(tag = "type")]
pub enum ContentBlockDelta {
    #[serde(rename = "thinking_delta")]
    ThinkingDelta { thinking: String },
    #[serde(rename = "signature_delta")]
    SignatureDelta { signature: String },
    #[serde(rename = "text_delta")]
    TextDelta { text: String },
    #[serde(rename = "input_json_delta")]
    InputJsonDelta { partial_json: String },
}

// ========== 模型列表 ==========

// 模型列表响应(GET /v1/models)
#[derive(Debug, Deserialize)]
pub struct ListModelsResponse {
    pub data: Vec<ModelInfo>,
}

// 模型信息(仅取 id 字段)
#[derive(Debug, Deserialize)]
pub struct ModelInfo {
    pub id: String,
}
