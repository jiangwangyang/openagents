// OpenAI Responses 流式 API 类型建模
use serde::{Deserialize, Serialize};

// ========== 请求体 ==========

// 创建响应请求(input/tools 由基准协议消息转换而来,直接用 Value 承载)
#[derive(Debug, Serialize)]
pub struct CreateResponseRequest {
    pub model: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub instructions: Option<String>,
    pub input: Vec<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tools: Option<Vec<serde_json::Value>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reasoning: Option<ReasoningConfig>,
    pub max_output_tokens: u32,
    pub stream: bool,
    pub store: bool,
    pub include: Vec<String>,
}

// Reasoning 配置(对应基准协议的 thinking 开关)
#[derive(Debug, Serialize)]
pub struct ReasoningConfig {
    pub effort: String,
    pub summary: String,
}

// ========== 流事件 ==========

// 流式事件(仅建模转换所需的事件,未知事件解析失败即跳过)
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
    #[serde(rename = "response.reasoning_summary_text.delta")]
    ReasoningSummaryTextDelta { output_index: u32, delta: String },
    #[serde(rename = "response.function_call_arguments.delta")]
    FunctionCallArgumentsDelta { item_id: String, output_index: u32, delta: String },
    #[serde(rename = "error")]
    Error { message: Option<String> },
}

// 响应对象(response.created/completed 中的 response)
#[derive(Debug, Clone, Deserialize)]
pub struct Response {
    pub id: String,
    pub model: Option<String>,
    pub status: Option<String>,
    pub usage: Option<ResponseUsage>,
    pub error: Option<serde_json::Value>,
}

// Token 使用量
#[derive(Debug, Clone, Deserialize)]
pub struct ResponseUsage {
    pub input_tokens: u64,
    pub output_tokens: u64,
    // 输入 token 明细(缓存命中量,部分供应商不返回该字段)
    pub input_tokens_details: Option<InputTokensDetails>,
}

// 输入 token 明细
#[derive(Debug, Clone, Deserialize)]
pub struct InputTokensDetails {
    #[serde(default)]
    pub cached_tokens: u64,
}

// 输出项(output_item.added/done 中的 item,仅建模转换所需的三种)
#[derive(Debug, Clone, Deserialize)]
#[serde(tag = "type")]
pub enum OutputItem {
    #[serde(rename = "reasoning")]
    Reasoning { encrypted_content: Option<String> },
    #[serde(rename = "message")]
    Message {},
    #[serde(rename = "function_call")]
    FunctionCall {
        id: Option<String>,
        call_id: String,
        name: String,
        arguments: Option<String>,
    },
}
