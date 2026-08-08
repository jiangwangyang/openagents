// 任务交接工具
use sqlx::SqlitePool;

use super::ToolResult;
use crate::repository::{agent_repository, conversation_repository, task_repository};

// 执行
pub async fn execute(cmd_and_args: &[String], task_id: Option<i64>, db: &SqlitePool) -> ToolResult {
    // 任务移交: task_id 为当前对话所属任务(独立对话为空)，为该任务创建新对话
    let task_id = match task_id {
        Some(id) => id,
        None => return ("Not in a task context, cannot hand over".to_string(), true),
    };

    if cmd_and_args.len() < 3 || cmd_and_args[1] != "handover" {
        return (format!("Unknown task command: {}", cmd_and_args.join(" ")), true);
    }

    // 查询任务
    let task = match task_repository::get_task(db, task_id).await {
        Ok(Some(t)) => t,
        Ok(None) => return (format!("Task not found: {}", task_id), true),
        Err(e) => return (format!("Database error: {}", e), true),
    };

    if task.conversations.is_empty() {
        return (format!("Task not found: {}", task_id), true);
    }

    // 新对话的 work_dir 取任务的工作目录
    let work_dir = &task.work_dir;

    // 移交给用户: 创建 agent_id 为 None 的用户审核对话
    if cmd_and_args[2] == "user" {
        let title = format!("{}-User", task.title);
        match conversation_repository::add_conversation(db, &title, work_dir, "", Some(task_id), None).await {
            Ok(_) => ("Task handed over to the user, please summarize the current progress".to_string(), false),
            Err(e) => (format!("Failed to create conversation: {}", e), true),
        }
    } else {
        // 移交给智能体: 校验 agent 存在且属于该任务团队
        let agent_id: i64 = match cmd_and_args[2].parse() {
            Ok(id) => id,
            Err(_) => return (format!("Invalid agent_id: {}", cmd_and_args[2]), true),
        };

        let agent = match agent_repository::get_agent(db, agent_id).await {
            Ok(Some(a)) => a,
            Ok(None) => return (format!("Agent not found in task team: {}", agent_id), true),
            Err(e) => return (format!("Database error: {}", e), true),
        };

        // 检查 agent 是否在任务团队中
        let agent_ids: Vec<i64> = match task.agent_ids.as_array() {
            Some(arr) => arr.iter().filter_map(|v| v.as_i64()).collect(),
            None => Vec::new(),
        };

        if !agent_ids.contains(&agent_id) {
            return (format!("Agent not found in task team: {}", agent_id), true);
        }

        let title = format!("{}-{}", task.title, agent.name);
        match conversation_repository::add_conversation(db, &title, work_dir, &agent.prompt, Some(task_id), Some(agent_id)).await {
            Ok(_) => (format!("Task handed over to agent {}, please summarize the current progress", agent.name), false),
            Err(e) => (format!("Failed to create conversation: {}", e), true),
        }
    }
}