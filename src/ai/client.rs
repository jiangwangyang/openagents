// 模型协议分发: 以 pi 协议为基准, 输入 pi Context, 输出 pi AssistantMessageEvent 流
use crate::ai::anthropic_messages::client::{self as anthropic_client, AnthropicError, AnthropicOptions};
use crate::ai::openai_responses::client::{self as responses_client, OpenAIResponsesOptions, OpenAiResponsesError};
use crate::ai::pi::types::{Context, Model};
use crate::ai::pi::utils::event_stream::AssistantMessageEventStream;
use crate::repository::entity::ModelProviderEntity;

// 统一模型调用错误
#[derive(Debug, thiserror::Error)]
pub enum ModelError {
    #[error(transparent)]
    Anthropic(#[from] AnthropicError),
    #[error(transparent)]
    OpenAiResponses(#[from] OpenAiResponsesError),
    #[error("unsupported protocol type: {0}")]
    UnsupportedProtocol(String),
}

// thinking 级别(pi ModelThinkingLevel): off 为关闭, 其余为 pi ThinkingLevel(minimal/low/medium/high/xhigh/max)
// Anthropic 思考预算表(对齐 pi DEFAULT_THINKING_BUDGETS), xhigh/max 钳制到 high(对齐 pi clampReasoning)
fn thinking_budget_tokens(thinking: &str) -> u32 {
    match thinking {
        "minimal" => 1024,
        "low" => 2048,
        "high" | "xhigh" | "max" => 16384,
        // medium 及未知级别兜底
        _ => 8192,
    }
}

// 流式调用, 按 provider 协议类型路由, 返回统一的 pi 基准协议事件流
pub fn stream(provider: &ModelProviderEntity, model_id: &str, thinking: &str, max_tokens: u32, context: &Context) -> Result<AssistantMessageEventStream, ModelError> {
    // pi Model: provider 取实体 id(稳定, 用于跨供应商重放比较), api 对齐 pi 的 KnownApi 命名
    // reasoning 为模型能力门(对齐 pi model.reasoning): thinking 级别经 options 传递(对齐 pi streamSimple),
    // off 时由各协议显式下发禁用参数(OpenAI effort: "none", Anthropic thinking: disabled)
    let model = |api: &str| Model { id: model_id.to_string(), api: api.to_string(), provider: provider.id.to_string(), base_url: provider.base_url.clone(), reasoning: true, input: vec!["text".to_string()], max_tokens };
    let thinking_enabled = thinking != "off";
    match provider.protocol_type.as_str() {
        "anthropic-messages" => {
            // 对齐 pi adjustMaxTokensForThinking: thinking 开启时请求 max_tokens 叠加思考预算,
            // 保证 max_tokens > budget_tokens(否则 Anthropic 返回 400); 本项目无模型上限目录, 省略 pi 的模型上限 clamp
            let request_max_tokens = if thinking_enabled { max_tokens.saturating_add(thinking_budget_tokens(thinking)) } else { max_tokens };
            Ok(anthropic_client::stream(&model("anthropic-messages"), context, &AnthropicOptions { api_key: Some(provider.api_key.clone()), max_tokens: Some(request_max_tokens), thinking_enabled: Some(thinking_enabled), thinking_budget_tokens: if thinking_enabled { Some(thinking_budget_tokens(thinking)) } else { None } }))
        }
        "openai-responses" => Ok(responses_client::stream(&model("openai-responses"), context, &OpenAIResponsesOptions { api_key: Some(provider.api_key.clone()), max_tokens: Some(max_tokens), reasoning_effort: if thinking_enabled { Some(thinking.to_string()) } else { None }, reasoning_summary: None })),
        other => Err(ModelError::UnsupportedProtocol(other.to_string())),
    }
}

// 获取可用模型列表, 按 provider 协议类型路由
pub async fn list_models(provider: &ModelProviderEntity) -> Result<Vec<String>, ModelError> {
    match provider.protocol_type.as_str() {
        "anthropic-messages" => anthropic_client::list_models(&provider.base_url, &provider.api_key).await.map_err(ModelError::from),
        "openai-responses" => responses_client::list_models(&provider.base_url, &provider.api_key).await.map_err(ModelError::from),
        other => Err(ModelError::UnsupportedProtocol(other.to_string())),
    }
}
