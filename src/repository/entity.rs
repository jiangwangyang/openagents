// 表结构对应的 struct(FromRow)
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

// Agent 查询结果(含关联的 ModelProvider)
#[derive(Debug, Clone, serde::Serialize)]
pub struct AgentWithProvider {
    pub id: i64,
    pub name: String,
    pub description: String,
    pub prompt: String,
    pub model_provider_id: i64,
    pub model: String,
    pub thinking: bool,
    pub create_time: String,
    pub update_time: String,
    pub model_provider: Option<ModelProviderEntity>,
}

// 任务
#[derive(Debug, Clone, FromRow, serde::Serialize)]
pub struct TaskEntity {
    pub id: i64,
    pub title: String,
    pub content: String,
    pub agent_ids: serde_json::Value,
    pub work_dir: String,
    pub create_time: String,
    pub update_time: String,
}

// 对话
#[derive(Debug, Clone, FromRow, serde::Serialize)]
pub struct ConversationEntity {
    pub id: i64,
    pub task_id: Option<i64>,
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

// 对话查询结果(含消息列表)
#[derive(Debug, Clone, serde::Serialize)]
pub struct ConversationWithMessages {
    pub id: i64,
    pub task_id: Option<i64>,
    pub agent_id: Option<i64>,
    pub title: String,
    pub work_dir: String,
    pub system_prompt: String,
    pub create_time: String,
    pub update_time: String,
    pub messages: Vec<MessageEntity>,
}

// 任务查询结果(含阶段对话)
#[derive(Debug, Clone, serde::Serialize)]
pub struct TaskWithConversations {
    pub id: i64,
    pub title: String,
    pub content: String,
    pub agent_ids: serde_json::Value,
    pub work_dir: String,
    pub create_time: String,
    pub update_time: String,
    pub conversations: Vec<ConversationWithMessagesAndAgent>,
}

// 阶段对话(含消息与执行 Agent)
#[derive(Debug, Clone, serde::Serialize)]
pub struct ConversationWithMessagesAndAgent {
    pub id: i64,
    pub task_id: Option<i64>,
    pub agent_id: Option<i64>,
    pub title: String,
    pub work_dir: String,
    pub system_prompt: String,
    pub create_time: String,
    pub update_time: String,
    pub messages: Vec<MessageEntity>,
    pub agent: Option<AgentWithProvider>,
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
