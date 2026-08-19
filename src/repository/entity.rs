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

// Agent 详情查询结果(含外键关联的模型提供商)
#[derive(Debug, Clone, serde::Serialize)]
pub struct AgentWithProvider {
    #[serde(flatten)]
    pub agent: AgentEntity,
    pub model_provider: Option<ModelProviderEntity>,
}

// JOIN 查询中 provider_ 前缀的模型提供商列(LEFT JOIN 未命中时全为 NULL)
#[derive(Debug, Clone, FromRow)]
pub struct ProviderJoinRow {
    pub provider_id: Option<i64>,
    pub provider_name: Option<String>,
    pub provider_protocol_type: Option<String>,
    pub provider_base_url: Option<String>,
    pub provider_api_key: Option<String>,
    pub provider_create_time: Option<String>,
    pub provider_update_time: Option<String>,
}

// ProviderJoinRow -> 模型提供商: provider_id 为 None 说明 LEFT JOIN 未命中
impl From<ProviderJoinRow> for Option<ModelProviderEntity> {
    fn from(r: ProviderJoinRow) -> Self {
        r.provider_id.map(|id| ModelProviderEntity {
            id,
            name: r.provider_name.unwrap_or_default(),
            protocol_type: r.provider_protocol_type.unwrap_or_default(),
            base_url: r.provider_base_url.unwrap_or_default(),
            api_key: r.provider_api_key.unwrap_or_default(),
            create_time: r.provider_create_time.unwrap_or_default(),
            update_time: r.provider_update_time.unwrap_or_default(),
        })
    }
}

// get_agent JOIN 查询中间行(Agent 列 + 关联的模型提供商列)
#[derive(Debug, Clone, FromRow)]
pub struct AgentProviderRow {
    pub id: i64,
    pub name: String,
    pub description: String,
    pub prompt: String,
    pub model_provider_id: i64,
    pub model: String,
    pub thinking: bool,
    pub create_time: String,
    pub update_time: String,
    #[sqlx(flatten)]
    pub provider: ProviderJoinRow,
}

impl From<AgentProviderRow> for AgentWithProvider {
    fn from(r: AgentProviderRow) -> Self {
        AgentWithProvider {
            agent: AgentEntity {
                id: r.id,
                name: r.name,
                description: r.description,
                prompt: r.prompt,
                model_provider_id: r.model_provider_id,
                model: r.model,
                thinking: r.thinking,
                create_time: r.create_time,
                update_time: r.update_time,
            },
            model_provider: r.provider.into(),
        }
    }
}

// 任务状态: 待启动/运行中/待审核/已完成/运行失败, 与前端 TASK_STATUS 常量对齐
pub const TASK_STATUS_IDLE: &str = "idle";
pub const TASK_STATUS_RUNNING: &str = "running";
pub const TASK_STATUS_REVIEW: &str = "review";
pub const TASK_STATUS_DONE: &str = "done";
pub const TASK_STATUS_FAILED: &str = "failed";

// 任务
#[derive(Debug, Clone, FromRow, serde::Serialize)]
pub struct TaskEntity {
    pub id: i64,
    pub title: String,
    pub content: String,
    pub agent_ids: sqlx::types::Json<Vec<i64>>,
    pub work_dir: String,
    pub status: String,
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

// 定时任务详情查询结果(含外键关联的执行 Agent)
#[derive(Debug, Clone, serde::Serialize)]
pub struct ScheduleWithAgent {
    #[serde(flatten)]
    pub schedule: ScheduleEntity,
    pub agent: Option<AgentEntity>,
}

// JOIN 查询中 agent_ 前缀的 Agent 列(LEFT JOIN 未命中时全为 NULL)
#[derive(Debug, Clone, FromRow)]
pub struct AgentJoinRow {
    pub agent_ref_id: Option<i64>,
    pub agent_name: Option<String>,
    pub agent_description: Option<String>,
    pub agent_prompt: Option<String>,
    pub agent_model_provider_id: Option<i64>,
    pub agent_model: Option<String>,
    pub agent_thinking: Option<bool>,
    pub agent_create_time: Option<String>,
    pub agent_update_time: Option<String>,
}

// AgentJoinRow -> Agent: agent_ref_id 为 None 说明 LEFT JOIN 未命中
impl From<AgentJoinRow> for Option<AgentEntity> {
    fn from(r: AgentJoinRow) -> Self {
        r.agent_ref_id.map(|id| AgentEntity {
            id,
            name: r.agent_name.unwrap_or_default(),
            description: r.agent_description.unwrap_or_default(),
            prompt: r.agent_prompt.unwrap_or_default(),
            model_provider_id: r.agent_model_provider_id.unwrap_or_default(),
            model: r.agent_model.unwrap_or_default(),
            thinking: r.agent_thinking.unwrap_or_default(),
            create_time: r.agent_create_time.unwrap_or_default(),
            update_time: r.agent_update_time.unwrap_or_default(),
        })
    }
}

// get_schedule JOIN 查询中间行(定时任务列 + 关联的 Agent 列)
#[derive(Debug, Clone, FromRow)]
pub struct ScheduleAgentRow {
    pub id: i64,
    pub name: String,
    pub content: String,
    pub work_dir: String,
    pub cron_expr: String,
    pub agent_id: i64,
    pub enabled: bool,
    pub create_time: String,
    pub update_time: String,
    #[sqlx(flatten)]
    pub agent: AgentJoinRow,
}

impl From<ScheduleAgentRow> for ScheduleWithAgent {
    fn from(r: ScheduleAgentRow) -> Self {
        ScheduleWithAgent {
            schedule: ScheduleEntity {
                id: r.id,
                name: r.name,
                content: r.content,
                work_dir: r.work_dir,
                cron_expr: r.cron_expr,
                agent_id: r.agent_id,
                enabled: r.enabled,
                create_time: r.create_time,
                update_time: r.update_time,
            },
            agent: r.agent.into(),
        }
    }
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

// 消息(content 列存整条 pi 消息 JSON, 含 role/usage/stopReason/timestamp)
#[derive(Debug, Clone, FromRow, serde::Serialize)]
pub struct MessageEntity {
    pub id: i64,
    pub conversation_id: i64,
    pub content: serde_json::Value,
}

// 对话详情查询结果(含外键关联的执行 Agent 与消息列表)
#[derive(Debug, Clone, serde::Serialize)]
pub struct ConversationWithMessages {
    #[serde(flatten)]
    pub conversation: ConversationEntity,
    pub agent: Option<AgentEntity>,
    pub messages: Vec<MessageEntity>,
}

// get_conversation JOIN 查询中间行(对话列 + 关联的 Agent 列), messages 由调用方另行查询填充
#[derive(Debug, Clone, FromRow)]
pub struct ConversationAgentRow {
    pub id: i64,
    pub task_id: Option<i64>,
    pub schedule_id: Option<i64>,
    pub agent_id: Option<i64>,
    pub title: String,
    pub work_dir: String,
    pub system_prompt: String,
    pub create_time: String,
    pub update_time: String,
    #[sqlx(flatten)]
    pub agent: AgentJoinRow,
}

impl From<ConversationAgentRow> for ConversationWithMessages {
    fn from(r: ConversationAgentRow) -> Self {
        ConversationWithMessages {
            conversation: ConversationEntity {
                id: r.id,
                task_id: r.task_id,
                schedule_id: r.schedule_id,
                agent_id: r.agent_id,
                title: r.title,
                work_dir: r.work_dir,
                system_prompt: r.system_prompt,
                create_time: r.create_time,
                update_time: r.update_time,
            },
            agent: r.agent.into(),
            messages: Vec::new(),
        }
    }
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
