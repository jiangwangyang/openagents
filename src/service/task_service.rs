// 多 Agent 执行循环
use serde_json::{json, Value};

use crate::repository::{agent_repository, conversation_repository, model_provider_repository, task_repository};
use crate::service::conversation_service;
use crate::state::AppState;

// 启动任务执行循环, 同一 task 同时只允许一个循环运行
pub fn start_task(state: &AppState, task_id: i64, agent_id: i64) -> bool {
    // 防重入: entry 原子检查并插入, 持锁期间不 await
    match state.task_loops.entry(task_id) {
        dashmap::mapref::entry::Entry::Occupied(mut entry) => {
            if !entry.get().is_finished() {
                return false;
            }
            let handle = tokio::spawn(run_task(state.clone(), task_id, agent_id));
            entry.insert(handle);
        }
        dashmap::mapref::entry::Entry::Vacant(entry) => {
            let handle = tokio::spawn(run_task(state.clone(), task_id, agent_id));
            entry.insert(handle);
        }
    }
    true
}

// 查询任务执行循环是否正在运行
pub fn is_task_running(state: &AppState, task_id: i64) -> bool {
    match state.task_loops.get(&task_id) {
        Some(handle) => !handle.is_finished(),
        None => false,
    }
}

// 后台执行循环
async fn run_task(state: AppState, task_id: i64, agent_id: i64) {
    let result = do_run_task(&state, task_id, agent_id).await;
    if let Err(e) = result {
        tracing::error!("Task execution failed: task_id={} error={}", task_id, e);
    }
    state.task_loops.remove(&task_id);
}

// 实际任务循环逻辑
async fn do_run_task(state: &AppState, task_id: i64, agent_id: i64) -> anyhow::Result<()> {
    tracing::info!("Task loop started: task_id={} agent_id={}", task_id, agent_id);
    // 为第一个执行的 agent 创建阶段对话
    let task = task_repository::get_task(&state.db, task_id).await?;
    let agent = agent_repository::get_agent(&state.db, agent_id).await?;
    let (task, agent) = match (task, agent) {
        (Some(t), Some(a)) => (t, a),
        _ => return Ok(()),
    };
    conversation_repository::add_conversation(
        &state.db,
        &format!("{}-{}", task.title, agent.name),
        &task.work_dir,
        &agent.prompt,
        Some(task_id),
        Some(agent_id),
        None,
    )
        .await?;

    loop {
        // 每轮重新查询任务基本字段与最新阶段对话状态
        let task = task_repository::get_task(&state.db, task_id).await?;
        let task = match task {
            Some(t) => t,
            None => return Ok(()),
        };
        let latest = conversation_repository::get_latest_task_conversation_state(&state.db, task_id).await?;
        let latest = match latest {
            Some(l) => l,
            None => return Ok(()),
        };

        // 最新的 agent 对话有消息(说明对话没有交接, 则默认交接给用户), 即创建一个无 agent 的用户对话并结束循环
        if latest.has_messages && latest.agent_id.is_some() {
            tracing::info!("Task handed over to user: task_id={}", task_id);
            conversation_repository::add_conversation(
                &state.db,
                &format!("{}-User", task.title),
                &task.work_dir,
                "",
                Some(task_id),
                None,
                None,
            )
                .await?;
            return Ok(());
        }

        // 最新对话无 agent(用户审核阶段), 结束循环
        let latest_agent_id = match latest.agent_id {
            Some(id) => id,
            None => {
                tracing::info!("Task loop ended: task_id={} reason=user_review", task_id);
                return Ok(());
            }
        };

        // 模型配置从当前对话的 Agent 读取
        let agent = agent_repository::get_agent(&state.db, latest_agent_id)
            .await?
            .ok_or_else(|| anyhow::anyhow!("agent not found"))?;
        // 模型提供方由上层按需查询
        let provider = model_provider_repository::get_model_provider(&state.db, agent.model_provider_id)
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
        let all_agents = agent_repository::list_agents(&state.db).await?;
        let team_agents: Vec<_> = all_agents.iter().filter(|a| task.agent_ids.0.contains(&a.id)).collect();
        let team_json: Vec<Value> = team_agents.iter().map(|a| json!({"id": a.id, "name": a.name, "description": a.description})).collect();
        task_content_list.push(format!("# Team\n{}", serde_json::to_string(&team_json)?));

        task_content_list.push("# History".to_string());
        let history = conversation_repository::list_task_conversation_history(&state.db, task_id).await?;
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
        tracing::info!("Task triggering conversation: task_id={} conversation_id={} agent={} model={}", task_id, latest.id, agent.name, agent.model);
        if !conversation_service::start_conversation(state, latest.id, task_content, provider.id, agent.model.clone(), agent.thinking).await {
            tracing::warn!("Task loop ended: task_id={} conversation_id={} already running", task_id, latest.id);
            return Ok(());
        }
        // 等待对话完成: 订阅通知并等待, 订阅后先复查 done 避免错过完成信号
        if let Some(conv_state) = conversation_service::get_conversation_state(state, latest.id) {
            let mut receiver = {
                let s = conv_state.read().await;
                s.notify.subscribe()
            };
            loop {
                {
                    let s = conv_state.read().await;
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
