// Agent 管理工具
use sqlx::SqlitePool;

use super::{parse_bool, parse_id, ToolResult};
use crate::repository::{agent_repository, DeleteResult};

// 执行 Agent 管理命令
pub async fn execute(cmd_and_args: &[String], db: &SqlitePool) -> ToolResult {
    let args: Vec<&str> = cmd_and_args.iter().map(String::as_str).collect();
    match args.as_slice() {
        // agent list
        ["agent", "list"] => match agent_repository::list_agents(db).await {
            Ok(agents) => (serde_json::to_string(&agents).unwrap_or_default(), false),
            Err(e) => (format!("Database error: {}", e), true),
        },
        // agent get <agent_id>
        ["agent", "get", agent_id] => {
            let agent_id = match parse_id(agent_id, "agent_id") {
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
        ["agent", "add", name, description, prompt, model_provider_id, model, thinking] => {
            let model_provider_id = match parse_id(model_provider_id, "model_provider_id") {
                Ok(id) => id,
                Err(r) => return r,
            };
            let thinking = match parse_bool(thinking, "thinking") {
                Ok(b) => b,
                Err(r) => return r,
            };
            match agent_repository::add_agent(
                db,
                name,
                description,
                prompt,
                model_provider_id,
                model,
                thinking,
            )
            .await
            {
                Ok(id) => (format!("Agent added with id {}", id), false),
                Err(e) => (format!("Failed to add agent: {}", e), true),
            }
        }
        // agent update <agent_id> <name> <description> <prompt> <model_provider_id> <model> <thinking>
        ["agent", "update", agent_id, name, description, prompt, model_provider_id, model, thinking] =>
        {
            let agent_id = match parse_id(agent_id, "agent_id") {
                Ok(id) => id,
                Err(r) => return r,
            };
            let model_provider_id = match parse_id(model_provider_id, "model_provider_id") {
                Ok(id) => id,
                Err(r) => return r,
            };
            let thinking = match parse_bool(thinking, "thinking") {
                Ok(b) => b,
                Err(r) => return r,
            };
            match agent_repository::update_agent(
                db,
                agent_id,
                name,
                description,
                prompt,
                model_provider_id,
                model,
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
        ["agent", "delete", agent_id] => {
            let agent_id = match parse_id(agent_id, "agent_id") {
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
        _ => (format!("Unknown agent command: {}", args.join(" ")), true),
    }
}
