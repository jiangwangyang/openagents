use dashmap::DashMap;
use serde_json::json;
use sqlx::SqlitePool;
// 多 Agent 执行循环
use std::sync::Arc;
use tokio::sync::RwLock;
use tokio::task::JoinHandle;

use crate::repository::{agent_repository, conversation_repository, task_repository};
use crate::service::conversation_service;
use crate::state::ConversationState;

// 每个 task 的执行循环后台任务句柄
static TASK_LOOPS: std::sync::LazyLock<Arc<DashMap<i64, JoinHandle<()>>>> =
    std::sync::LazyLock::new(|| Arc::new(DashMap::new()));

// 启动任务执行循环,同一 task 同时只允许一个循环运行
pub fn start_task(
    task_id: i64,
    agent_id: i64,
    db: &SqlitePool,
    conversations: &Arc<DashMap<i64, Arc<RwLock<ConversationState>>>>,
) -> bool {
    // 防重入
    if let Some(handle) = TASK_LOOPS.get(&task_id) {
        if !handle.is_finished() {
            return false;
        }
    }
    let db = db.clone();
    let conversations = conversations.clone();
    let handle = tokio::spawn(run_task(task_id, agent_id, db, conversations));
    TASK_LOOPS.insert(task_id, handle);
    true
}

// 后台执行循环
async fn run_task(
    task_id: i64,
    agent_id: i64,
    db: SqlitePool,
    conversations: Arc<DashMap<i64, Arc<RwLock<ConversationState>>>>,
) {
    let result = do_run_task(task_id, agent_id, &db, &conversations).await;
    if let Err(e) = result {
        tracing::error!("Task execution failed: {}", e);
    }
    TASK_LOOPS.remove(&task_id);
}

// 实际任务循环逻辑
async fn do_run_task(
    task_id: i64,
    agent_id: i64,
    db: &SqlitePool,
    conversations: &Arc<DashMap<i64, Arc<RwLock<ConversationState>>>>,
) -> anyhow::Result<()> {
    // 为第一个执行的 agent 创建阶段对话
    let task = task_repository::get_task(db, task_id).await?;
    let agent = agent_repository::get_agent(db, agent_id).await?;
    let (task, agent) = match (task, agent) {
        (Some(t), Some(a)) => (t, a),
        _ => return Ok(()),
    };
    conversation_repository::add_conversation(
        db,
        &format!("{}-{}", task.title, agent.name),
        &task.work_dir,
        &agent.prompt,
        Some(task_id),
        Some(agent_id),
    )
        .await?;

    loop {
        // 每轮重新查询任务,取最新一条对话
        let task = task_repository::get_task(db, task_id).await?;
        let task = match task {
            Some(t) if !t.conversations.is_empty() => t,
            _ => return Ok(()),
        };
        let conversation = task.conversations.last().unwrap();

        // 最新的 agent 对话有消息(说明对话没有交接，则默认交接给用户),即创建一个无 agent 的用户对话并结束循环
        if !conversation.messages.is_empty() && conversation.agent_id.is_some() {
            conversation_repository::add_conversation(
                db,
                &format!("{}-User", task.title),
                &task.work_dir,
                "",
                Some(task_id),
                None,
            )
                .await?;
            return Ok(());
        }

        // 最新对话无 agent(用户审核阶段),结束循环
        if conversation.agent_id.is_none() {
            return Ok(());
        }

        // 拼接各阶段对话的最后一条消息
        let mut task_content_list = Vec::new();
        task_content_list.push(format!(
            "# Task\n{}",
            serde_json::to_string(&json!({"title": task.title, "content": task.content}))?
        ));

        // 团队成员为 agent_ids 候选池对应的 Agent
        let all_agents = agent_repository::list_agents(db).await?;
        let task_agent_ids: Vec<i64> = task.agent_ids.as_array()
            .map(|arr| arr.iter().filter_map(|v| v.as_i64()).collect())
            .unwrap_or_default();
        let team_agents: Vec<_> = all_agents.iter().filter(|a| task_agent_ids.contains(&a.id)).collect();
        let team_json: Vec<Value> = team_agents.iter().map(|a| json!({"id": a.id, "name": a.name, "description": a.description})).collect();
        task_content_list.push(format!("# Team\n{}", serde_json::to_string(&team_json)?));

        task_content_list.push("# History".to_string());
        for history_conversation in &task.conversations {
            if history_conversation.messages.is_empty() {
                continue;
            }
            let name = history_conversation.agent.as_ref().map(|a| a.name.as_str()).unwrap_or("User");
            let last_msg = history_conversation.messages.last().unwrap();
            let text = match &last_msg.content {
                Value::String(s) => s.clone(),
                Value::Array(arr) => {
                    arr.last()
                        .and_then(|b| b["text"].as_str())
                        .unwrap_or("")
                        .to_string()
                }
                _ => String::new(),
            };
            task_content_list.push(format!("## {}\n{}", name, serde_json::to_string(&text)?));
        }
        let task_content = task_content_list.join("\n\n");

        // 模型配置从当前对话的 Agent 读取
        let agent = conversation.agent.as_ref()
            .ok_or_else(|| anyhow::anyhow!("agent not found"))?;
        let provider = agent.model_provider.as_ref()
            .ok_or_else(|| anyhow::anyhow!("model provider not configured"))?;
        if agent.model.is_empty() {
            anyhow::bail!("model not configured");
        }

        // 触发对话执行并等待完成
        if !conversation_service::start_conversation(conversation.id, task_content, provider.id, agent.model.clone(), agent.thinking, conversations, db).await {
            return Ok(());
        }
        // 等待对话完成
        if let Some(state) = conversation_service::get_conversation_state(conversation.id, conversations) {
            loop {
                {
                    let s = state.read().await;
                    if s.done {
                        break;
                    }
                }
                tokio::time::sleep(std::time::Duration::from_millis(100)).await;
            }
        }
    }
}

use serde_json::Value;
