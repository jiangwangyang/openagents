// OpenAI Responses 流式 API 类型建模(对齐 pi/packages/ai/src/api/openai-responses-shared.ts 处理的事件子集)
use serde::{Deserialize, Serialize};

// ========== 请求体 ==========

// 创建响应请求(input/tools 由 pi 基准协议消息转换而来, 直接用 Value 承载)
#[derive(Debug, Serialize)]
pub struct CreateResponseRequest {
    pub model: String,
    pub input: Vec<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tools: Option<Vec<serde_json::Value>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reasoning: Option<ReasoningConfig>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_output_tokens: Option<u32>,
    pub stream: bool,
    pub store: bool,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub include: Vec<String>,
}

// Reasoning 配置
#[derive(Debug, Serialize)]
pub struct ReasoningConfig {
    pub effort: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub summary: Option<String>,
}

// ========== 流事件 ==========

// 流式事件(仅建模转换所需的事件, 未建模的事件归为 Unknown 直接跳过)
#[derive(Debug, Clone, Deserialize)]
#[serde(tag = "type")]
pub enum ResponseStreamEvent {
    #[serde(rename = "response.created")]
    ResponseCreated { response: Response },
    #[serde(rename = "response.completed")]
    ResponseCompleted { response: Response },
    #[serde(rename = "response.incomplete")]
    ResponseIncomplete { response: Response },
    #[serde(rename = "response.failed")]
    ResponseFailed { response: Response },
    #[serde(rename = "response.output_item.added")]
    OutputItemAdded { output_index: u32, item: OutputItem },
    #[serde(rename = "response.output_item.done")]
    OutputItemDone { output_index: u32, item: OutputItem },
    #[serde(rename = "response.output_text.delta")]
    OutputTextDelta { output_index: u32, delta: String },
    #[serde(rename = "response.refusal.delta")]
    RefusalDelta { output_index: u32, delta: String },
    #[serde(rename = "response.reasoning_summary_text.delta")]
    ReasoningSummaryTextDelta { output_index: u32, delta: String },
    #[serde(rename = "response.reasoning_summary_part.done")]
    ReasoningSummaryPartDone { output_index: u32 },
    #[serde(rename = "response.reasoning_text.delta")]
    ReasoningTextDelta { output_index: u32, delta: String },
    #[serde(rename = "response.function_call_arguments.delta")]
    FunctionCallArgumentsDelta { output_index: u32, delta: String },
    #[serde(rename = "response.function_call_arguments.done")]
    FunctionCallArgumentsDone {
        output_index: u32,
        arguments: String,
    },
    #[serde(rename = "error")]
    Error {
        code: Option<String>,
        message: Option<String>,
    },
    // 协议中无需处理的生命周期事件(如 response.in_progress / content_part.added 等)
    #[serde(other)]
    Unknown,
}

// 响应对象(response.created/completed/incomplete/failed 中的 response)
#[derive(Debug, Clone, Deserialize)]
pub struct Response {
    pub id: Option<String>,
    pub status: Option<String>,
    pub usage: Option<ResponseUsage>,
    pub error: Option<ResponseError>,
    pub incomplete_details: Option<IncompleteDetails>,
    pub output: Option<Vec<OutputItem>>,
}

// 响应错误
#[derive(Debug, Clone, Deserialize)]
pub struct ResponseError {
    pub code: Option<String>,
    pub message: Option<String>,
}

// 不完整明细
#[derive(Debug, Clone, Deserialize)]
pub struct IncompleteDetails {
    pub reason: Option<String>,
}

// Token 使用量
// 对齐 pi 的 || 0 容错: 字段缺省回退 0(部分代理在终态响应中省略 usage 字段)
#[derive(Debug, Clone, Deserialize)]
pub struct ResponseUsage {
    #[serde(default)]
    pub input_tokens: u64,
    #[serde(default)]
    pub output_tokens: u64,
    pub total_tokens: Option<u64>,
    pub input_tokens_details: Option<InputTokensDetails>,
    pub output_tokens_details: Option<OutputTokensDetails>,
}

// 输入 token 明细
#[derive(Debug, Clone, Deserialize)]
pub struct InputTokensDetails {
    #[serde(default)]
    pub cached_tokens: u64,
    #[serde(default)]
    pub cache_write_tokens: u64,
}

// 输出 token 明细
#[derive(Debug, Clone, Deserialize)]
pub struct OutputTokensDetails {
    pub reasoning_tokens: Option<u64>,
}

// 输出项(output_item.added/done 与 response.output 中的 item, 未建模 custom_tool_call)
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum OutputItem {
    #[serde(rename = "reasoning")]
    Reasoning(ResponseReasoningItem),
    #[serde(rename = "message")]
    Message(ResponseOutputMessage),
    #[serde(rename = "function_call")]
    FunctionCall(ResponseFunctionCall),
    #[serde(other)]
    Unknown,
}

// reasoning 项(thinkingSignature 以其 JSON 序列化形式存储用于跨轮回放)
// 对齐 pi 的 JSON.stringify(item) 全量存储: 未建模字段经 extra 原样保留, 回放不丢失;
// id 容忍缺省(对齐 pi 不做校验, 缺省时回填/查找按空串处理)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResponseReasoningItem {
    #[serde(default)]
    pub id: String,
    #[serde(default)]
    pub summary: Vec<ReasoningTextPart>,
    #[serde(default)]
    pub content: Vec<ReasoningTextPart>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub encrypted_content: Option<String>,
    // 未建模字段原样保留(如 status 等)
    #[serde(flatten)]
    pub extra: serde_json::Map<String, serde_json::Value>,
}

// reasoning 摘要/文本部分
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReasoningTextPart {
    #[serde(rename = "type")]
    pub part_type: String,
    pub text: String,
}

// message 输出项
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResponseOutputMessage {
    pub id: String,
    #[serde(default)]
    pub content: Vec<ResponseOutputContent>,
    pub status: Option<String>,
    pub phase: Option<String>,
}

// message 输出内容
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum ResponseOutputContent {
    #[serde(rename = "output_text")]
    OutputText { text: String },
    #[serde(rename = "refusal")]
    Refusal { refusal: String },
    #[serde(other)]
    Unknown,
}

// function_call 输出项(未迁移字段: namespace, 本项目无 OpenAI 自定义工具)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResponseFunctionCall {
    pub id: Option<String>,
    pub call_id: String,
    pub name: String,
    pub arguments: Option<String>,
}

// ========== 模型列表 ==========

// 模型列表响应(GET /models)
#[derive(Debug, Deserialize)]
pub struct ListModelsResponse {
    pub data: Vec<ModelInfo>,
}

// 模型信息(仅取 id 字段)
#[derive(Debug, Deserialize)]
pub struct ModelInfo {
    pub id: String,
}
