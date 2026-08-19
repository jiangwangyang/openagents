// 定时任务管理工具
use std::str::FromStr;

use super::{parse_bool, parse_id, ToolResult};
use crate::repository::schedule_repository;
use crate::service::schedule_service;
use crate::state::AppState;

// 执行定时任务管理命令
pub async fn execute(cmd_and_args: &[String], state: &AppState) -> ToolResult {
    match cmd_and_args.get(1).map(String::as_str) {
        // schedule list
        Some("list") if cmd_and_args.len() == 2 => {
            match schedule_repository::list_schedules(&state.db).await {
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
            }
        }
        // schedule get <schedule_id>
        Some("get") if cmd_and_args.len() == 3 => {
            let schedule_id = match parse_id(&cmd_and_args[2], "schedule_id") {
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
        Some("add") if cmd_and_args.len() == 7 => {
            // 校验 cron 表达式合法性(与调度器使用同一 cron 解析器)
            if cron::Schedule::from_str(&cmd_and_args[5]).is_err() {
                return (format!("Invalid cron_expr: {}", cmd_and_args[5]), true);
            }
            let agent_id = match parse_id(&cmd_and_args[6], "agent_id") {
                Ok(id) => id,
                Err(r) => return r,
            };
            match schedule_service::add_schedule(
                state,
                &cmd_and_args[2],
                &cmd_and_args[3],
                &cmd_and_args[4],
                &cmd_and_args[5],
                agent_id,
            )
            .await
            {
                Ok(id) => (format!("Schedule added with id {}", id), false),
                Err(e) => (format!("Failed to add schedule: {}", e), true),
            }
        }
        // schedule update <schedule_id> <name> <content> <work_dir> <cron_expr> <agent_id> <enabled>
        Some("update") if cmd_and_args.len() == 9 => {
            let schedule_id = match parse_id(&cmd_and_args[2], "schedule_id") {
                Ok(id) => id,
                Err(r) => return r,
            };
            // 校验 cron 表达式合法性(与调度器使用同一 cron 解析器)
            if cron::Schedule::from_str(&cmd_and_args[6]).is_err() {
                return (format!("Invalid cron_expr: {}", cmd_and_args[6]), true);
            }
            let agent_id = match parse_id(&cmd_and_args[7], "agent_id") {
                Ok(id) => id,
                Err(r) => return r,
            };
            let enabled = match parse_bool(&cmd_and_args[8], "enabled") {
                Ok(b) => b,
                Err(r) => return r,
            };
            match schedule_service::update_schedule(
                state,
                schedule_id,
                &cmd_and_args[3],
                &cmd_and_args[4],
                &cmd_and_args[5],
                &cmd_and_args[6],
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
        Some("delete") if cmd_and_args.len() == 3 => {
            let schedule_id = match parse_id(&cmd_and_args[2], "schedule_id") {
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
            format!("Unknown schedule command: {}", cmd_and_args.join(" ")),
            true,
        ),
    }
}
