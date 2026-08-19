// 多 Agent 执行循环
use serde_json::{json, Value};

use crate::ai::pi::types::Message;
use crate::error::AppError;
use crate::repository::entity::{TaskEntity, TASK_STATUS_FAILED, TASK_STATUS_REVIEW};
use crate::repository::{
    agent_repository, conversation_repository, model_provider_repository, task_repository,
};
use crate::service::conversation_service;
use crate::state::AppState;

// 同一对话连续交接提醒的最大次数, 超过后结束任务循环, 防止 agent 始终不交接导致死循环
const MAX_HANDOVER_REMINDERS: u32 = 3;

// 启动任务执行循环, 同一 task 同时只允许一个循环运行
pub fn start_task(state: &AppState, task_id: i64, agent_id: i64) -> bool {
    // 防重入: entry 原子检查并插入, 持锁期间不 await
    match state.task_loops.entry(task_id) {
        dashmap::mapref::entry::Entry::Occupied(mut entry) => {
            if !entry.get().0.is_finished() {
                return false;
            }
            let (stop_tx, stop_rx) = tokio::sync::watch::channel(false);
            let handle = tokio::spawn(run_task(state.clone(), task_id, agent_id, stop_rx));
            entry.insert((handle, stop_tx));
        }
        dashmap::mapref::entry::Entry::Vacant(entry) => {
            let (stop_tx, stop_rx) = tokio::sync::watch::channel(false);
            let handle = tokio::spawn(run_task(state.clone(), task_id, agent_id, stop_rx));
            entry.insert((handle, stop_tx));
        }
    }
    true
}

// 查询任务执行循环是否正在运行
pub fn is_task_running(state: &AppState, task_id: i64) -> bool {
    match state.task_loops.get(&task_id) {
        Some(entry) => !entry.0.is_finished(),
        None => false,
    }
}

// 停止任务执行循环: 发送停止信号, 任务未在运行返回 false, 幂等无副作用
pub fn stop_task(state: &AppState, task_id: i64) -> bool {
    if let Some(entry) = state.task_loops.get(&task_id) {
        if entry.0.is_finished() {
            return false;
        }
        let _ = entry.1.send(true);
        tracing::info!("Task stop requested: task_id={}", task_id);
        return true;
    }
    false
}

// 向任务落一条用户消息: 最新阶段对话为用户审核对话(agent_id 为空)时直接追加,
// 否则 allow_create 为 true 时新建用户审核对话承载, 为 false 时返回 Ok(None) 由调用方转 409
pub async fn append_task_user_message(
    state: &AppState,
    task: &TaskEntity,
    message: &str,
    allow_create: bool,
) -> Result<Option<i64>, AppError> {
    let latest =
        conversation_repository::get_latest_task_conversation_state(&state.db, task.id).await?;
    let conversation_id = match latest {
        Some(l) if l.agent_id.is_none() => l.id,
        _ if allow_create => {
            conversation_repository::add_conversation(
                &state.db,
                &format!("{}-User", task.title),
                &task.work_dir,
                "",
                Some(task.id),
                None,
                None,
            )
            .await?
        }
        _ => return Ok(None),
    };
    // content 列存整条 pi 消息 JSON
    let user_message = conversation_service::user_text_message(message);
    conversation_repository::add_conversation_messages(
        &state.db,
        conversation_id,
        &[serde_json::to_value(&user_message)?],
    )
    .await?;
    Ok(Some(conversation_id))
}

// 后台执行循环
async fn run_task(
    state: AppState,
    task_id: i64,
    agent_id: i64,
    stop_rx: tokio::sync::watch::Receiver<bool>,
) {
    let result = do_run_task(&state, task_id, agent_id, stop_rx).await;
    if let Err(e) = result {
        tracing::error!("Task execution failed: task_id={} error={}", task_id, e);
        // 循环异常退出: 置为运行失败(任务已删除时更新 0 行, 无副作用)
        if let Err(err) =
            task_repository::update_task_status(&state.db, task_id, TASK_STATUS_FAILED).await
        {
            tracing::error!(
                "Task status update failed: task_id={} error={}",
                task_id,
                err
            );
        }
    }
    state.task_loops.remove(&task_id);
}

// 实际任务循环逻辑
async fn do_run_task(
    state: &AppState,
    task_id: i64,
    agent_id: i64,
    mut stop_rx: tokio::sync::watch::Receiver<bool>,
) -> anyhow::Result<()> {
    tracing::info!(
        "Task loop started: task_id={} agent_id={}",
        task_id,
        agent_id
    );
    // 为第一个执行的 agent 创建阶段对话
    let task = task_repository::get_task(&state.db, task_id).await?;
    let agent = agent_repository::get_agent(&state.db, agent_id)
        .await?
        .map(|d| d.agent);
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

    // 上一轮对话 id 与连续提醒计数, 对话切换(发生交接)时清零
    let mut prev_conversation_id: Option<i64> = None;
    let mut consecutive_reminders = 0u32;

    loop {
        // 每轮开头检测任务停止信号, 覆盖交接间隙触发停止的场景
        if *stop_rx.borrow() {
            tracing::info!("Task loop stopped: task_id={} reason=user_stop", task_id);
            // 手动停止: 循环停在 Agent 对话, 与前端推导一致置为运行失败
            task_repository::update_task_status(&state.db, task_id, TASK_STATUS_FAILED).await?;
            return Ok(());
        }

        // 每轮重新查询任务基本字段与最新阶段对话状态
        let task = task_repository::get_task(&state.db, task_id).await?;
        let task = match task {
            Some(t) => t,
            None => return Ok(()),
        };
        let latest =
            conversation_repository::get_latest_task_conversation_state(&state.db, task_id).await?;
        let latest = match latest {
            Some(l) => l,
            None => return Ok(()),
        };

        // 对话切换(发生交接)时清零连续提醒计数
        if prev_conversation_id != Some(latest.id) {
            prev_conversation_id = Some(latest.id);
            consecutive_reminders = 0;
        }

        // 最新的 agent 对话有消息(说明上一轮没有交接), 不自动交接给用户, 本轮改为向当前对话追加提醒 user 消息
        let need_handover_reminder = latest.has_messages && latest.agent_id.is_some();
        if need_handover_reminder {
            // 同一对话连续提醒达到上限仍未交接, 结束循环防止死循环
            consecutive_reminders += 1;
            if consecutive_reminders > MAX_HANDOVER_REMINDERS {
                tracing::warn!(
                    "Task loop ended: task_id={} conversation_id={} reason=handover_reminder_limit",
                    task_id,
                    latest.id
                );
                // 连续不交接超限: 置为运行失败
                task_repository::update_task_status(&state.db, task_id, TASK_STATUS_FAILED).await?;
                return Ok(());
            }
            tracing::info!(
                "Task round ended without handover, reminding agent: task_id={} conversation_id={} reminder={}/{}",
                task_id,
                latest.id,
                consecutive_reminders,
                MAX_HANDOVER_REMINDERS
            );
        }

        // 最新对话无 agent(用户审核阶段), 结束循环
        let latest_agent_id = match latest.agent_id {
            Some(id) => id,
            None => {
                tracing::info!("Task loop ended: task_id={} reason=user_review", task_id);
                // 最新对话无 agent: 交接给用户, 置为待审核
                task_repository::update_task_status(&state.db, task_id, TASK_STATUS_REVIEW).await?;
                return Ok(());
            }
        };

        // 模型配置从当前对话的 Agent 读取
        let agent = agent_repository::get_agent(&state.db, latest_agent_id)
            .await?
            .ok_or_else(|| anyhow::anyhow!("agent not found"))?
            .agent;
        let provider =
            model_provider_repository::get_model_provider(&state.db, agent.model_provider_id)
                .await?
                .ok_or_else(|| anyhow::anyhow!("model provider not configured"))?;
        if agent.model.is_empty() {
            anyhow::bail!("model not configured");
        }

        // 提醒轮仅发送交接提醒消息, 正常轮拼接任务与各阶段对话的最后一条消息
        let task_content = if need_handover_reminder {
            "[System] Your previous turn ended without handing over the task. You must call the handover tool now: use `task handover <agent_id>` to hand over to a teammate, or `task handover user` to hand over to the user for review.".to_string()
        } else {
            let mut task_content_list = Vec::new();
            task_content_list.push(format!(
                "# Task\n{}",
                serde_json::to_string(&json!({"title": task.title, "content": task.content}))?
            ));

            // 团队成员为 agent_ids 候选池对应的 Agent
            let all_agents = agent_repository::list_agents(&state.db).await?;
            let team_agents: Vec<_> = all_agents
                .iter()
                .filter(|a| task.agent_ids.0.contains(&a.id))
                .collect();
            let team_json: Vec<Value> = team_agents
                .iter()
                .map(|a| json!({"id": a.id, "name": a.name, "description": a.description}))
                .collect();
            task_content_list.push(format!("# Team\n{}", serde_json::to_string(&team_json)?));

            task_content_list.push("# History".to_string());
            let history =
                conversation_repository::list_task_conversation_history(&state.db, task_id).await?;
            for item in &history {
                // 无消息的阶段对话不参与历史
                let last_content = match &item.last_content {
                    Some(c) => c,
                    None => continue,
                };
                let name = item.agent_name.as_deref().unwrap_or("User");
                // content 列为 pi 消息协议 JSON: 取用户文本或 assistant 最后一个文本块
                let text = match serde_json::from_value::<Message>(last_content.clone()) {
                    Ok(Message::User(u)) => conversation_service::user_message_text(&u.content),
                    Ok(Message::Assistant(a)) => conversation_service::assistant_message_text(&a),
                    _ => String::new(),
                };
                task_content_list.push(format!("## {}\n{}", name, serde_json::to_string(&text)?));
            }
            // 交接提示: 要求 agent 完成工作后必须调用交接工具, 避免未交接直接结束
            task_content_list.push("# Handover\nWhen your work is done, you must call the handover tool: use `task handover <agent_id>` to hand over to a teammate, or `task handover user` to hand over to the user for review. Do not end your turn without handing over.".to_string());
            task_content_list.join("\n\n")
        };

        // 触发对话执行并等待完成
        tracing::info!(
            "Task triggering conversation: task_id={} conversation_id={} agent={} model={}",
            task_id,
            latest.id,
            agent.name,
            agent.model
        );
        if !conversation_service::start_conversation(
            state,
            latest.id,
            task_content,
            provider.id,
            agent.model.clone(),
            agent.thinking,
            false,
        )
        .await
        {
            tracing::warn!(
                "Task loop ended: task_id={} conversation_id={} already running",
                task_id,
                latest.id
            );
            return Ok(());
        }
        // 等待对话完成: 订阅通知并等待, 订阅后先复查 done 避免错过完成信号; 同时监听任务停止信号
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
                tokio::select! {
                    // 发送端关闭(状态被移除)时退出等待
                    changed = receiver.changed() => {
                        if changed.is_err() {
                            break;
                        }
                    }
                    // 收到任务停止信号: 优雅停止当前对话(已执行内容入库)后结束任务循环
                    _ = stop_rx.changed() => {
                        conversation_service::stop_conversation(state, latest.id).await;
                        tracing::info!("Task loop stopped: task_id={} reason=user_stop", task_id);
                        // 手动停止: 置为运行失败
                        task_repository::update_task_status(&state.db, task_id, TASK_STATUS_FAILED)
                            .await?;
                        return Ok(());
                    }
                }
            }
        }
    }
}
