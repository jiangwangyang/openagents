// 模型协议分发层: 以 Anthropic 协议为基准, 按 provider.protocol_type 路由到具体协议适配器
pub mod anthropic;
pub mod client;
pub mod openai_responses;
