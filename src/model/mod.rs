// 模型协议分发层: 以 Anthropic 协议为基准, 按 provider.protocol_type 路由到具体协议适配器
pub mod anthropic;
pub mod client;
pub mod openai_responses;

// 日志内容截断: 超过 max 字符时截断并追加省略号, 避免长内容刷爆日志
pub(crate) fn truncate_str(s: &str, max: usize) -> String {
    if s.chars().count() > max {
        format!("{}...", s.chars().take(max).collect::<String>())
    } else {
        s.to_string()
    }
}
