// 定时任务管理工具
use std::str::FromStr;

use super::{parse_bool, parse_id, ToolResult};
use crate::repository::schedule_repository;
use crate::service::schedule_service;
use crate::state::AppState;

// 执行定时任务管理命令
pub async fn execute(cmd_and_args: &[String], state: &AppState) -> ToolResult {
    let args: Vec<&str> = cmd_and_args.iter().map(String::as_str).collect();
    match args.as_slice() {
        // schedule list
        ["schedule", "list"] => match schedule_repository::list_schedules(&state.db).await {
            Ok(schedules) => {
                let result: Vec<serde_json::Value> = schedules
                    .iter()
                    .map(|s| {
                        serde_json::json!({
                            "id": s.id,
                            "name": s.name,
                            "content": s.content,
                            "work_dir": s.work_dir,
                            "cron_expr": s.cron_expr,
                            "agent_id": s.agent_id,
                            "enabled": s.enabled,
                            "next_fire_time": schedule_service::next_fire_time(&s.cron_expr),
                            "create_time": s.create_time,
                            "update_time": s.update_time,
                        })
                    })
                    .collect();
                (serde_json::to_string(&result).unwrap_or_default(), false)
            }
            Err(e) => (format!("Database error: {}", e), true),
        },
        // schedule get <schedule_id>
        ["schedule", "get", schedule_id] => {
            let schedule_id = match parse_id(schedule_id, "schedule_id") {
                Ok(id) => id,
                Err(r) => return r,
            };
            match schedule_repository::get_schedule(&state.db, schedule_id).await {
                Ok(Some(s)) => {
                    // 工具输出保持定时任务基本字段, 不含关联的 Agent 实体
                    let s = s.schedule;
                    let result = serde_json::json!({
                        "id": s.id,
                        "name": s.name,
                        "content": s.content,
                        "work_dir": s.work_dir,
                        "cron_expr": s.cron_expr,
                        "agent_id": s.agent_id,
                        "enabled": s.enabled,
                        "next_fire_time": schedule_service::next_fire_time(&s.cron_expr),
                        "create_time": s.create_time,
                        "update_time": s.update_time,
                    });
                    (serde_json::to_string(&result).unwrap_or_default(), false)
                }
                Ok(None) => (format!("Schedule not found: {}", schedule_id), true),
                Err(e) => (format!("Database error: {}", e), true),
            }
        }
        // schedule add <name> <content> <work_dir> <cron_expr> <agent_id>
        ["schedule", "add", name, content, work_dir, cron_expr, agent_id] => {
            // 校验 cron 表达式合法性(与调度器使用同一 cron 解析器)
            if cron::Schedule::from_str(cron_expr).is_err() {
                return (format!("Invalid cron_expr: {}", cron_expr), true);
            }
            let agent_id = match parse_id(agent_id, "agent_id") {
                Ok(id) => id,
                Err(r) => return r,
            };
            match schedule_service::add_schedule(
                state, name, content, work_dir, cron_expr, agent_id,
            )
            .await
            {
                Ok(id) => (format!("Schedule added with id {}", id), false),
                Err(e) => (format!("Failed to add schedule: {}", e), true),
            }
        }
        // schedule update <schedule_id> <name> <content> <work_dir> <cron_expr> <agent_id> <enabled>
        ["schedule", "update", schedule_id, name, content, work_dir, cron_expr, agent_id, enabled] =>
        {
            let schedule_id = match parse_id(schedule_id, "schedule_id") {
                Ok(id) => id,
                Err(r) => return r,
            };
            // 校验 cron 表达式合法性(与调度器使用同一 cron 解析器)
            if cron::Schedule::from_str(cron_expr).is_err() {
                return (format!("Invalid cron_expr: {}", cron_expr), true);
            }
            let agent_id = match parse_id(agent_id, "agent_id") {
                Ok(id) => id,
                Err(r) => return r,
            };
            let enabled = match parse_bool(enabled, "enabled") {
                Ok(b) => b,
                Err(r) => return r,
            };
            match schedule_service::update_schedule(
                state,
                schedule_id,
                name,
                content,
                work_dir,
                cron_expr,
                agent_id,
                enabled,
            )
            .await
            {
                Ok(true) => (format!("Schedule {} updated", schedule_id), false),
                Ok(false) => (format!("Schedule not found: {}", schedule_id), true),
                Err(e) => (format!("Failed to update schedule: {}", e), true),
            }
        }
        // schedule delete <schedule_id>
        ["schedule", "delete", schedule_id] => {
            let schedule_id = match parse_id(schedule_id, "schedule_id") {
                Ok(id) => id,
                Err(r) => return r,
            };
            match schedule_service::delete_schedule(state, schedule_id).await {
                Ok(true) => (format!("Schedule {} deleted", schedule_id), false),
                Ok(false) => (format!("Schedule not found: {}", schedule_id), true),
                Err(e) => (format!("Failed to delete schedule: {}", e), true),
            }
        }
        _ => (
            format!("Unknown schedule command: {}", args.join(" ")),
            true,
        ),
    }
}
