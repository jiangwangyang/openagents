// Anthropic 流式 API 类型建模(请求侧对齐 pi/packages/ai/src/api/anthropic-messages.ts 的 convertMessages/convertTools 输出)
use serde::{Deserialize, Serialize};

// ========== 请求体 ==========

// 创建消息请求
#[derive(Debug, Serialize)]
pub struct CreateMessageRequest {
    pub model: String,
    pub messages: Vec<MessageParam>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub system: Option<Vec<SystemBlock>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tools: Option<Vec<ToolParam>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub thinking: Option<ThinkingConfig>,
    pub max_tokens: u32,
    pub stream: bool,
}

// 系统提示块
#[derive(Debug, Serialize)]
pub struct SystemBlock {
    #[serde(rename = "type")]
    pub block_type: String, // 固定 "text"
    pub text: String,
}

// 请求消息
#[derive(Debug, Serialize)]
pub struct MessageParam {
    pub role: String,
    pub content: MessageParamContent,
}

// 请求消息内容(纯文本或内容块数组)
#[derive(Debug, Serialize)]
#[serde(untagged)]
pub enum MessageParamContent {
    Text(String),
    Blocks(Vec<ContentBlockParam>),
}

// 请求内容块
#[derive(Debug, Serialize)]
#[serde(tag = "type")]
pub enum ContentBlockParam {
    #[serde(rename = "text")]
    Text { text: String },
    #[serde(rename = "thinking")]
    Thinking { thinking: String, signature: String },
    #[serde(rename = "redacted_thinking")]
    RedactedThinking { data: String },
    #[serde(rename = "tool_use")]
    ToolUse {
        id: String,
        name: String,
        input: serde_json::Value,
    },
    #[serde(rename = "tool_result")]
    ToolResult {
        tool_use_id: String,
        content: MessageParamContent,
        is_error: bool,
    },
    #[serde(rename = "image")]
    Image { source: ImageSource },
}

// 图片来源
#[derive(Debug, Serialize)]
pub struct ImageSource {
    #[serde(rename = "type")]
    pub source_type: String, // 固定 "base64"
    pub media_type: String,
    pub data: String,
}

// 工具定义
#[derive(Debug, Serialize)]
pub struct ToolParam {
    pub name: String,
    pub description: String,
    pub input_schema: serde_json::Value,
}

// Thinking 配置
#[derive(Debug, Clone, Serialize)]
#[serde(tag = "type")]
pub enum ThinkingConfig {
    #[serde(rename = "enabled")]
    Enabled { budget_tokens: u32, display: String },
    #[serde(rename = "disabled")]
    Disabled,
}

// ========== 响应消息 ==========

// 完整消息(message_start 事件中的 message)
// 对齐 pi 的容错: 仅读取 id/usage, 其余字段容忍缺省(部分代理返回非标准 message_start)
#[allow(dead_code)]
#[derive(Debug, Clone, Deserialize)]
pub struct Message {
    #[serde(default)]
    pub id: String,
    #[serde(rename = "type", default)]
    pub msg_type: String,
    #[serde(default)]
    pub role: String,
    #[serde(default)]
    pub model: String,
    #[serde(default)]
    pub content: Vec<ContentBlock>,
    pub stop_reason: Option<String>,
    pub stop_sequence: Option<String>,
    #[serde(default)]
    pub usage: Usage,
}

// 内容块(响应中的 content block)
// 对齐 pi 的 ?? "" 容错: 文本/思考/签名字段容忍缺省, input 缺省回退空对象
#[derive(Debug, Clone, Deserialize)]
#[serde(tag = "type")]
pub enum ContentBlock {
    #[serde(rename = "thinking")]
    Thinking {
        #[serde(default)]
        thinking: String,
        #[serde(default)]
        signature: String,
    },
    #[serde(rename = "redacted_thinking")]
    RedactedThinking { data: String },
    #[serde(rename = "text")]
    Text {
        #[serde(default)]
        text: String,
    },
    #[serde(rename = "tool_use")]
    ToolUse {
        id: String,
        name: String,
        #[serde(default)]
        input: serde_json::Value,
    },
}

// Token 使用量(message_start 中的 usage)
// 对齐 pi 的 || 0 容错: 字段缺省回退 0
#[allow(dead_code)]
#[derive(Debug, Clone, Default, Deserialize)]
pub struct Usage {
    #[serde(default)]
    pub input_tokens: u64,
    #[serde(default)]
    pub output_tokens: u64,
    pub cache_creation_input_tokens: Option<u64>,
    pub cache_read_input_tokens: Option<u64>,
}

// message_delta 中的 usage
// 对齐 pi 的 != null 容错: 所有字段可选(部分代理在 message_delta 中省略字段)
#[allow(dead_code)]
#[derive(Debug, Clone, Deserialize)]
pub struct MessageDeltaUsage {
    pub output_tokens: Option<u64>,
    pub cache_creation_input_tokens: Option<u64>,
    pub cache_read_input_tokens: Option<u64>,
    pub input_tokens: Option<u64>,
    // 输出 token 明细(对齐 pi: Anthropic 在终态 message_delta 上报 thinking_tokens, 为 output 的子集)
    pub output_tokens_details: Option<OutputTokensDetails>,
}

// 输出 token 明细
#[derive(Debug, Clone, Deserialize)]
pub struct OutputTokensDetails {
    pub thinking_tokens: Option<u64>,
}

// ========== 流事件 ==========

// 流式事件(对应 pi 中的 RawMessageStreamEvent)
#[allow(dead_code)]
#[derive(Debug, Clone, Deserialize)]
#[serde(tag = "type")]
pub enum MessageStreamEvent {
    #[serde(rename = "message_start")]
    MessageStart { message: Message },
    #[serde(rename = "message_delta")]
    MessageDelta {
        delta: MessageDelta,
        usage: Option<MessageDeltaUsage>,
    },
    #[serde(rename = "message_stop")]
    MessageStop,
    #[serde(rename = "content_block_start")]
    ContentBlockStart {
        index: u32,
        content_block: ContentBlock,
    },
    #[serde(rename = "content_block_delta")]
    ContentBlockDelta {
        index: u32,
        delta: ContentBlockDelta,
    },
    #[serde(rename = "content_block_stop")]
    ContentBlockStop { index: u32 },
    #[serde(rename = "ping")]
    Ping,
    #[serde(rename = "error")]
    Error { error: MessageStreamError },
    // 未知事件兜底: 容忍 API 新增事件类型, 静默跳过
    #[serde(other)]
    Unknown,
}

// error 事件中的错误明细
#[derive(Debug, Clone, Deserialize)]
pub struct MessageStreamError {
    #[serde(rename = "type")]
    pub error_type: String,
    pub message: String,
}

// message_delta 事件中的 delta
#[allow(dead_code)]
#[derive(Debug, Clone, Deserialize)]
pub struct MessageDelta {
    pub stop_reason: Option<String>,
    pub stop_sequence: Option<String>,
    pub stop_details: Option<RefusalStopDetails>,
}

// refusal 停止明细
#[allow(dead_code)]
#[derive(Debug, Clone, Deserialize)]
pub struct RefusalStopDetails {
    #[serde(rename = "type")]
    pub details_type: String,
    pub explanation: Option<String>,
}

// content_block_delta 事件中的 delta
// 变体名与 Anthropic 协议字段对齐, 允许统一的 Delta 后缀
#[allow(clippy::enum_variant_names)]
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
