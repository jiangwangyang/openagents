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

// thinking 开关开启时映射的固定思考级别(pi ThinkingLevel)
const THINKING_LEVEL: &str = "medium";
// medium 级别对应的 Anthropic 思考预算(pi adjustMaxTokensForThinking 默认值)
const THINKING_BUDGET_TOKENS: u32 = 8192;

// 流式调用, 按 provider 协议类型路由, 返回统一的 pi 基准协议事件流
pub fn stream(
    provider: &ModelProviderEntity,
    model_id: &str,
    thinking: bool,
    max_tokens: u32,
    context: &Context,
) -> Result<AssistantMessageEventStream, ModelError> {
    // pi Model: provider 取实体 id(稳定, 用于跨供应商重放比较), api 对齐 pi 的 KnownApi 命名
    // reasoning 取对话的 thinking 开关: 关闭时不下发 thinking/reasoning 参数(对齐 pi 以 model.reasoning 为门)
    let model = |api: &str| Model {
        id: model_id.to_string(),
        api: api.to_string(),
        provider: provider.id.to_string(),
        base_url: provider.base_url.clone(),
        reasoning: thinking,
        input: vec!["text".to_string()],
        max_tokens,
    };
    match provider.protocol_type.as_str() {
        "anthropic" => Ok(anthropic_client::stream(
            &model("anthropic-messages"),
            context,
            &AnthropicOptions {
                api_key: Some(provider.api_key.clone()),
                max_tokens: Some(max_tokens),
                thinking_enabled: Some(thinking),
                thinking_budget_tokens: if thinking { Some(THINKING_BUDGET_TOKENS) } else { None },
            },
        )),
        "openai-responses" => Ok(responses_client::stream(
            &model("openai-responses"),
            context,
            &OpenAIResponsesOptions {
                api_key: Some(provider.api_key.clone()),
                max_tokens: Some(max_tokens),
                reasoning_effort: if thinking { Some(THINKING_LEVEL.to_string()) } else { None },
                reasoning_summary: None,
            },
        )),
        other => Err(ModelError::UnsupportedProtocol(other.to_string())),
    }
}

// 获取可用模型列表, 按 provider 协议类型路由
pub async fn list_models(provider: &ModelProviderEntity) -> Result<Vec<String>, ModelError> {
    match provider.protocol_type.as_str() {
        "anthropic" => anthropic_client::list_models(&provider.base_url, &provider.api_key).await.map_err(ModelError::from),
        "openai-responses" => responses_client::list_models(&provider.base_url, &provider.api_key).await.map_err(ModelError::from),
        other => Err(ModelError::UnsupportedProtocol(other.to_string())),
    }
}
