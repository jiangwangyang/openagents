// 多 Agent 执行循环
use dashmap::DashMap;
use serde_json::{json, Value};
use sqlx::SqlitePool;
use std::sync::Arc;
use tokio::sync::RwLock;
use tokio::task::JoinHandle;

use crate::repository::{agent_repository, conversation_repository, model_provider_repository, task_repository};
use crate::service::conversation_service;
use crate::state::ConversationState;

// 每个 task 的执行循环后台任务句柄
static TASK_LOOPS: std::sync::LazyLock<DashMap<i64, JoinHandle<()>>> =
    std::sync::LazyLock::new(DashMap::new);

// 启动任务执行循环,同一 task 同时只允许一个循环运行
pub fn start_task(
    task_id: i64,
    agent_id: i64,
    db: &SqlitePool,
    conversations: &Arc<DashMap<i64, Arc<RwLock<ConversationState>>>>,
) -> bool {
    // 防重入: entry 原子检查并插入,持锁期间不 await
    match TASK_LOOPS.entry(task_id) {
        dashmap::mapref::entry::Entry::Occupied(mut entry) => {
            if !entry.get().is_finished() {
                return false;
            }
            let handle = tokio::spawn(run_task(task_id, agent_id, db.clone(), conversations.clone()));
            entry.insert(handle);
        }
        dashmap::mapref::entry::Entry::Vacant(entry) => {
            let handle = tokio::spawn(run_task(task_id, agent_id, db.clone(), conversations.clone()));
            entry.insert(handle);
        }
    }
    true
}

// 查询任务执行循环是否正在运行
pub fn is_task_running(task_id: i64) -> bool {
    match TASK_LOOPS.get(&task_id) {
        Some(handle) => !handle.is_finished(),
        None => false,
    }
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
        // 每轮重新查询任务基本字段与最新阶段对话状态
        let task = task_repository::get_task(db, task_id).await?;
        let task = match task {
            Some(t) => t,
            None => return Ok(()),
        };
        let latest = conversation_repository::get_latest_task_conversation_state(db, task_id).await?;
        let latest = match latest {
            Some(l) => l,
            None => return Ok(()),
        };

        // 最新的 agent 对话有消息(说明对话没有交接，则默认交接给用户),即创建一个无 agent 的用户对话并结束循环
        if latest.has_messages && latest.agent_id.is_some() {
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
        let latest_agent_id = match latest.agent_id {
            Some(id) => id,
            None => return Ok(()),
        };

        // 模型配置从当前对话的 Agent 读取
        let agent = agent_repository::get_agent(db, latest_agent_id)
            .await?
            .ok_or_else(|| anyhow::anyhow!("agent not found"))?;
        // 模型提供方由上层按需查询
        let provider = model_provider_repository::get_model_provider(db, agent.model_provider_id)
            .await?
            .ok_or_else(|| anyhow::anyhow!("model provider not configured"))?;
        if agent.model.is_empty() {
            anyhow::bail!("model not configured");
        }

        // 拼接各阶段对话的最后一条消息
        let mut task_content_list = Vec::new();
        task_content_list.push(format!(
            "# Task\n{}",
            serde_json::to_string(&json!({"title": task.title, "content": task.content}))?
        ));

        // 团队成员为 agent_ids 候选池对应的 Agent
        let all_agents = agent_repository::list_agents(db).await?;
        let team_agents: Vec<_> = all_agents.iter().filter(|a| task.agent_ids.0.contains(&a.id)).collect();
        let team_json: Vec<Value> = team_agents.iter().map(|a| json!({"id": a.id, "name": a.name, "description": a.description})).collect();
        task_content_list.push(format!("# Team\n{}", serde_json::to_string(&team_json)?));

        task_content_list.push("# History".to_string());
        let history = conversation_repository::list_task_conversation_history(db, task_id).await?;
        for item in &history {
            // 无消息的阶段对话不参与历史
            let last_content = match &item.last_content {
                Some(c) => c,
                None => continue,
            };
            let name = item.agent_name.as_deref().unwrap_or("User");
            let text = match last_content {
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

        // 触发对话执行并等待完成
        if !conversation_service::start_conversation(latest.id, task_content, provider.id, agent.model.clone(), agent.thinking, conversations, db).await {
            return Ok(());
        }
        // 等待对话完成: 订阅通知并等待,订阅后先复查 done 避免错过完成信号
        if let Some(state) = conversation_service::get_conversation_state(latest.id, conversations) {
            let mut receiver = {
                let s = state.read().await;
                s.notify.subscribe()
            };
            loop {
                {
                    let s = state.read().await;
                    if s.done {
                        break;
                    }
                }
                // 发送端关闭(状态被移除)时退出等待
                if receiver.changed().await.is_err() {
                    break;
                }
            }
        }
    }
}
