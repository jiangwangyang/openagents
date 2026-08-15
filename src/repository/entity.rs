// 表结构实体(FromRow)
use sqlx::FromRow;

// 模型提供商
#[derive(Debug, Clone, FromRow, serde::Serialize)]
pub struct ModelProviderEntity {
    pub id: i64,
    pub name: String,
    pub protocol_type: String,
    pub base_url: String,
    pub api_key: String,
    pub create_time: String,
    pub update_time: String,
}

// Agent 定义
#[derive(Debug, Clone, FromRow, serde::Serialize)]
pub struct AgentEntity {
    pub id: i64,
    pub name: String,
    pub description: String,
    pub prompt: String,
    pub model_provider_id: i64,
    pub model: String,
    pub thinking: bool,
    pub create_time: String,
    pub update_time: String,
}

// 任务
#[derive(Debug, Clone, FromRow, serde::Serialize)]
pub struct TaskEntity {
    pub id: i64,
    pub title: String,
    pub content: String,
    pub agent_ids: sqlx::types::Json<Vec<i64>>,
    pub work_dir: String,
    pub create_time: String,
    pub update_time: String,
}

// 定时任务
#[derive(Debug, Clone, FromRow, serde::Serialize)]
pub struct ScheduleEntity {
    pub id: i64,
    pub name: String,
    pub content: String,
    pub work_dir: String,
    pub cron_expr: String,
    pub agent_id: i64,
    pub enabled: bool,
    pub create_time: String,
    pub update_time: String,
}

// 对话
#[derive(Debug, Clone, FromRow, serde::Serialize)]
pub struct ConversationEntity {
    pub id: i64,
    pub task_id: Option<i64>,
    pub schedule_id: Option<i64>,
    pub agent_id: Option<i64>,
    pub title: String,
    pub work_dir: String,
    pub system_prompt: String,
    pub create_time: String,
    pub update_time: String,
}

// 消息
#[derive(Debug, Clone, FromRow, serde::Serialize)]
pub struct MessageEntity {
    pub id: i64,
    pub conversation_id: i64,
    pub role: String,
    pub content: serde_json::Value,
    pub stop_reason: String,
    pub cache_read_input_tokens: i64,
    pub input_tokens: i64,
    pub output_tokens: i64,
    pub time: String,
}

// MCP 服务
#[derive(Debug, Clone, FromRow, serde::Serialize)]
pub struct McpServerEntity {
    pub id: i64,
    pub name: String,
    pub description: String,
    pub protocol_type: String,
    pub url: Option<String>,
    pub headers: Option<serde_json::Value>,
    pub command: Option<String>,
    pub args: Option<serde_json::Value>,
    pub create_time: String,
    pub update_time: String,
}

// Web 存储(前端持久化 KV)
#[derive(Debug, Clone, FromRow, serde::Serialize)]
pub struct WebStorageEntity {
    pub key: String,
    pub value: String,
    pub create_time: String,
    pub update_time: String,
}

// 待写入的消息(add_conversation_messages 参数)
#[derive(Debug, Clone)]
pub struct NewMessageEntity {
    pub role: String,
    pub content: serde_json::Value,
    pub stop_reason: String,
    pub cache_read_input_tokens: i64,
    pub input_tokens: i64,
    pub output_tokens: i64,
    pub time: String,
}

// 对话查询结果(含消息列表)
#[derive(Debug, Clone, serde::Serialize)]
pub struct ConversationWithMessages {
    #[serde(flatten)]
    pub conversation: ConversationEntity,
    pub messages: Vec<MessageEntity>,
}

// 任务循环查询结果: 最新阶段对话状态
#[derive(Debug, Clone, FromRow)]
pub struct LatestConversationState {
    pub id: i64,
    pub agent_id: Option<i64>,
    pub has_messages: bool,
}

// 任务循环查询结果: 阶段对话历史摘要(执行 Agent 名称 + 最后一条消息内容)
#[derive(Debug, Clone, FromRow)]
pub struct ConversationHistorySummary {
    pub agent_name: Option<String>,
    pub last_content: Option<serde_json::Value>,
}
