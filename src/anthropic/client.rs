// Anthropic 流式客户端(手写,替代 Python anthropic SDK)
use eventsource_stream::Eventsource;
use futures_util::Stream;
use reqwest::Client;
use std::pin::Pin;

use super::types::{CreateMessageRequest, MessageStreamEvent};

// Anthropic API 客户端
pub struct AnthropicClient {
    base_url: String,
    api_key: String,
    http: Client,
}

// 流式响应类型
pub type EventStream = Pin<Box<dyn Stream<Item = Result<MessageStreamEvent, AnthropicError>> + Send>>;

// 共享 HTTP 客户端(内部为 Arc，克隆仅复制引用，全应用复用同一连接池)
static HTTP_CLIENT: std::sync::LazyLock<Client> = std::sync::LazyLock::new(Client::new);

// Anthropic 客户端错误
#[derive(Debug, thiserror::Error)]
pub enum AnthropicError {
    #[error("HTTP 请求失败: {0}")]
    Http(#[from] reqwest::Error),
    #[error("SSE 解析失败: {0}")]
    Sse(String),
    #[error("JSON 反序列化失败: {0}")]
    Json(#[from] serde_json::Error),
    #[error("API 错误 {status}: {body}")]
    Api { status: u16, body: String },
}

impl AnthropicClient {
    // 创建客户端
    pub fn new(base_url: &str, api_key: &str) -> Self {
        Self {
            base_url: base_url.trim_end_matches('/').to_string(),
            api_key: api_key.to_string(),
            http: HTTP_CLIENT.clone(),
        }
    }

    // 流式创建消息,返回事件流
    pub async fn create_message_stream(
        &self,
        request: &CreateMessageRequest,
    ) -> Result<EventStream, AnthropicError> {
        let url = format!("{}/v1/messages", self.base_url);
        let response = self
            .http
            .post(&url)
            .header("x-api-key", &self.api_key)
            .header("anthropic-version", "2023-06-01")
            .header("content-type", "application/json")
            .json(request)
            .send()
            .await?;

        let status = response.status().as_u16();
        if status != 200 {
            let body = response.text().await.unwrap_or_default();
            return Err(AnthropicError::Api { status, body });
        }

        // bytes_stream -> eventsource-stream -> 反序列化为 MessageStreamEvent
        let byte_stream = response.bytes_stream();
        let event_stream = byte_stream.eventsource();

        let stream = futures_util::stream::StreamExt::filter_map(event_stream, |result| {
            std::future::ready(match result {
                Ok(event) => {
                    // SSE data 字段反序列化为 MessageStreamEvent
                    match serde_json::from_str::<MessageStreamEvent>(&event.data) {
                        Ok(evt) => Some(Ok(evt)),
                        Err(e) => {
                            tracing::warn!("未知 SSE 事件: {} error={}", event.data, e);
                            None
                        }
                    }
                }
                Err(e) => Some(Err(AnthropicError::Sse(e.to_string()))),
            })
        });

        Ok(Box::pin(stream))
    }
}
