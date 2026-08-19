// 任务工具: 任务交接与任务管理
use super::{parse_id, ToolContext, ToolResult};
use crate::repository::{agent_repository, conversation_repository, task_repository};
use crate::service::task_service;

// 执行任务命令
pub async fn execute(cmd_and_args: &[String], ctx: &ToolContext) -> ToolResult {
    let args: Vec<&str> = cmd_and_args.iter().map(String::as_str).collect();
    match args.as_slice() {
        // task handover <agent_id> | task handover user
        ["task", "handover", target] => {
            // 任务移交: task_id 为当前对话所属任务(独立对话为空), 为该任务创建新对话
            let task_id = match ctx.task_id {
                Some(id) => id,
                None => return ("Not in a task context, cannot hand over".to_string(), true),
            };

            // 查询任务基本字段(交接只需标题/工作目录/团队, 无需全量加载对话与消息)
            let task = match task_repository::get_task(&ctx.state.db, task_id).await {
                Ok(Some(t)) => t,
                Ok(None) => return (format!("Task not found: {}", task_id), true),
                Err(e) => return (format!("Database error: {}", e), true),
            };

            // 新对话的 work_dir 取任务的工作目录
            let work_dir = &task.work_dir;

            // 移交给用户: 创建 agent_id 为 None 的用户审核对话
            if *target == "user" {
                let title = format!("{}-User", task.title);
                match conversation_repository::add_conversation(
                    &ctx.state.db,
                    &title,
                    work_dir,
                    "",
                    Some(task_id),
                    None,
                    None,
                )
                .await
                {
                    Ok(_) => (
                        "Task handed over to the user, please summarize the current progress"
                            .to_string(),
                        false,
                    ),
                    Err(e) => (format!("Failed to create conversation: {}", e), true),
                }
            } else {
                // 移交给智能体: 校验 agent 存在且属于该任务团队
                let agent_id = match parse_id(target, "agent_id") {
                    Ok(id) => id,
                    Err(r) => return r,
                };

                let agent = match agent_repository::get_agent(&ctx.state.db, agent_id).await {
                    Ok(Some(a)) => a.agent,
                    Ok(None) => {
                        return (format!("Agent not found in task team: {}", agent_id), true)
                    }
                    Err(e) => return (format!("Database error: {}", e), true),
                };

                if !task.agent_ids.0.contains(&agent_id) {
                    return (format!("Agent not found in task team: {}", agent_id), true);
                }

                let title = format!("{}-{}", task.title, agent.name);
                match conversation_repository::add_conversation(
                    &ctx.state.db,
                    &title,
                    work_dir,
                    &agent.prompt,
                    Some(task_id),
                    Some(agent_id),
                    None,
                )
                .await
                {
                    Ok(_) => (
                        format!(
                            "Task handed over to agent {}, please summarize the current progress",
                            agent.name
                        ),
                        false,
                    ),
                    Err(e) => (format!("Failed to create conversation: {}", e), true),
                }
            }
        }
        // task list
        ["task", "list"] => match task_repository::list_tasks(&ctx.state.db).await {
            Ok(tasks) => (serde_json::to_string(&tasks).unwrap_or_default(), false),
            Err(e) => (format!("Database error: {}", e), true),
        },
        // task get <task_id>
        ["task", "get", task_id] => {
            let task_id = match parse_id(task_id, "task_id") {
                Ok(id) => id,
                Err(r) => return r,
            };
            match task_repository::get_task(&ctx.state.db, task_id).await {
                Ok(Some(task)) => (serde_json::to_string(&task).unwrap_or_default(), false),
                Ok(None) => (format!("Task not found: {}", task_id), true),
                Err(e) => (format!("Database error: {}", e), true),
            }
        }
        // task add <title> <content> <agent_ids> <work_dir>
        ["task", "add", title, content, agent_ids, work_dir] => {
            let agent_ids: Vec<i64> = match serde_json::from_str(agent_ids) {
                Ok(ids) => ids,
                Err(_) => return (format!("Invalid agent_ids JSON array: {}", agent_ids), true),
            };
            match task_repository::add_task(&ctx.state.db, title, content, &agent_ids, work_dir)
                .await
            {
                Ok(id) => (format!("Task added with id {}", id), false),
                Err(e) => (format!("Failed to add task: {}", e), true),
            }
        }
        // task update <task_id> <title> <content> <agent_ids> <work_dir>
        ["task", "update", task_id, title, content, agent_ids, work_dir] => {
            let task_id = match parse_id(task_id, "task_id") {
                Ok(id) => id,
                Err(r) => return r,
            };
            let agent_ids: Vec<i64> = match serde_json::from_str(agent_ids) {
                Ok(ids) => ids,
                Err(_) => return (format!("Invalid agent_ids JSON array: {}", agent_ids), true),
            };
            match task_repository::update_task(
                &ctx.state.db,
                task_id,
                title,
                content,
                &agent_ids,
                work_dir,
            )
            .await
            {
                Ok(true) => (format!("Task {} updated", task_id), false),
                Ok(false) => (format!("Task not found: {}", task_id), true),
                Err(e) => (format!("Failed to update task: {}", e), true),
            }
        }
        // task delete <task_id>
        ["task", "delete", task_id] => {
            let task_id = match parse_id(task_id, "task_id") {
                Ok(id) => id,
                Err(r) => return r,
            };
            // 运行中的任务不允许删除, 避免级联删除阶段对话导致后台循环写消息失败
            if task_service::is_task_running(&ctx.state, task_id) {
                return (format!("Task {} is running", task_id), true);
            }
            match task_repository::delete_task(&ctx.state.db, task_id).await {
                Ok(true) => (format!("Task {} deleted", task_id), false),
                Ok(false) => (format!("Task not found: {}", task_id), true),
                Err(e) => (format!("Failed to delete task: {}", e), true),
            }
        }
        _ => (format!("Unknown task command: {}", args.join(" ")), true),
    }
}
