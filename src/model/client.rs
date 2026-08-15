// 模型协议分发: 对外仅暴露基准协议(Anthropic)类型的统一接口
use futures_util::{Stream, StreamExt};
use std::pin::Pin;

use crate::model::anthropic::client::{self as anthropic_client, AnthropicError};
use crate::model::anthropic::types::{CreateMessageRequest, MessageStreamEvent};
use crate::model::openai_responses::client::{self as responses_client, OpenAiResponsesError};
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

// 统一基准协议事件流
pub type ModelEventStream = Pin<Box<dyn Stream<Item = Result<MessageStreamEvent, ModelError>> + Send>>;

// 流式创建消息, 按 provider 协议类型路由, 返回统一的基准协议事件流
pub async fn create_message_stream(
    provider: &ModelProviderEntity,
    request: &CreateMessageRequest,
) -> Result<ModelEventStream, ModelError> {
    match provider.protocol_type.as_str() {
        "anthropic" => {
            let stream = anthropic_client::create_message_stream(&provider.base_url, &provider.api_key, request).await?;
            Ok(Box::pin(stream.map(|r| r.map_err(ModelError::from))))
        }
        "openai-responses" => {
            let stream = responses_client::create_message_stream(&provider.base_url, &provider.api_key, request).await?;
            Ok(Box::pin(stream.map(|r| r.map_err(ModelError::from))))
        }
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
