// Agent 管理工具
use sqlx::SqlitePool;

use super::{parse_bool, parse_id, ToolResult};
use crate::repository::{agent_repository, DeleteResult};

// 执行 Agent 管理命令
pub async fn execute(cmd_and_args: &[String], db: &SqlitePool) -> ToolResult {
    match cmd_and_args.get(1).map(String::as_str) {
        // agent list
        Some("list") if cmd_and_args.len() == 2 => match agent_repository::list_agents(db).await {
            Ok(agents) => (serde_json::to_string(&agents).unwrap_or_default(), false),
            Err(e) => (format!("Database error: {}", e), true),
        },
        // agent get <agent_id>
        Some("get") if cmd_and_args.len() == 3 => {
            let agent_id = match parse_id(&cmd_and_args[2], "agent_id") {
                Ok(id) => id,
                Err(r) => return r,
            };
            match agent_repository::get_agent(db, agent_id).await {
                // 工具输出只序列化 Agent 本体, 避免关联的模型提供商 api_key 进入模型上下文
                Ok(Some(agent)) => (
                    serde_json::to_string(&agent.agent).unwrap_or_default(),
                    false,
                ),
                Ok(None) => (format!("Agent not found: {}", agent_id), true),
                Err(e) => (format!("Database error: {}", e), true),
            }
        }
        // agent add <name> <description> <prompt> <model_provider_id> <model> <thinking>
        Some("add") if cmd_and_args.len() == 8 => {
            let model_provider_id = match parse_id(&cmd_and_args[5], "model_provider_id") {
                Ok(id) => id,
                Err(r) => return r,
            };
            let thinking = match parse_bool(&cmd_and_args[7], "thinking") {
                Ok(b) => b,
                Err(r) => return r,
            };
            match agent_repository::add_agent(
                db,
                &cmd_and_args[2],
                &cmd_and_args[3],
                &cmd_and_args[4],
                model_provider_id,
                &cmd_and_args[6],
                thinking,
            )
            .await
            {
                Ok(id) => (format!("Agent added with id {}", id), false),
                Err(e) => (format!("Failed to add agent: {}", e), true),
            }
        }
        // agent update <agent_id> <name> <description> <prompt> <model_provider_id> <model> <thinking>
        Some("update") if cmd_and_args.len() == 9 => {
            let agent_id = match parse_id(&cmd_and_args[2], "agent_id") {
                Ok(id) => id,
                Err(r) => return r,
            };
            let model_provider_id = match parse_id(&cmd_and_args[6], "model_provider_id") {
                Ok(id) => id,
                Err(r) => return r,
            };
            let thinking = match parse_bool(&cmd_and_args[8], "thinking") {
                Ok(b) => b,
                Err(r) => return r,
            };
            match agent_repository::update_agent(
                db,
                agent_id,
                &cmd_and_args[3],
                &cmd_and_args[4],
                &cmd_and_args[5],
                model_provider_id,
                &cmd_and_args[7],
                thinking,
            )
            .await
            {
                Ok(true) => (format!("Agent {} updated", agent_id), false),
                Ok(false) => (format!("Agent not found: {}", agent_id), true),
                Err(e) => (format!("Failed to update agent: {}", e), true),
            }
        }
        // agent delete <agent_id>
        Some("delete") if cmd_and_args.len() == 3 => {
            let agent_id = match parse_id(&cmd_and_args[2], "agent_id") {
                Ok(id) => id,
                Err(r) => return r,
            };
            match agent_repository::delete_agent(db, agent_id).await {
                Ok(DeleteResult::Deleted) => (format!("Agent {} deleted", agent_id), false),
                Ok(DeleteResult::NotFound) => (format!("Agent not found: {}", agent_id), true),
                Ok(DeleteResult::Referenced) => (
                    format!(
                        "Agent {} is referenced by conversations/schedules",
                        agent_id
                    ),
                    true,
                ),
                Err(e) => (format!("Failed to delete agent: {}", e), true),
            }
        }
        _ => (
            format!("Unknown agent command: {}", cmd_and_args.join(" ")),
            true,
        ),
    }
}
