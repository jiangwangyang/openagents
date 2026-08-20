// 助手消息事件流(移植自 pi/packages/ai/src/utils/event-stream.ts)
// 对齐说明: pi 的 AssistantMessageEventStream 类支持 push/异步迭代双端;
// Rust 侧由适配器经 mpsc 发送, 消费端统一为 Stream, 因此仅保留类型别名
use std::pin::Pin;

use futures_util::Stream;

use crate::ai::pi::types::AssistantMessageEvent;

// 统一事件流类型
pub type AssistantMessageEventStream = Pin<Box<dyn Stream<Item = AssistantMessageEvent> + Send>>;
