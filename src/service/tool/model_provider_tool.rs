// 模型提供商查询工具
use sqlx::SqlitePool;

use super::ToolResult;
use crate::repository::model_provider_repository;

// 执行模型提供商命令
pub async fn execute(cmd_and_args: &[String], db: &SqlitePool) -> ToolResult {
    let args: Vec<&str> = cmd_and_args.iter().map(String::as_str).collect();
    match args.as_slice() {
        // model_provider list
        ["model_provider", "list"] => {
            match model_provider_repository::list_model_providers(db).await {
                Ok(providers) => {
                    // 不返回 api_key, 避免密钥进入对话上下文
                    let result: Vec<serde_json::Value> = providers.iter().map(|p| serde_json::json!({"id": p.id, "name": p.name, "protocol_type": p.protocol_type, "base_url": p.base_url, "create_time": p.create_time, "update_time": p.update_time})).collect();
                    (serde_json::to_string(&result).unwrap_or_default(), false)
                }
                Err(e) => (format!("Database error: {}", e), true),
            }
        }
        _ => (format!("Unknown model_provider command: {}", args.join(" ")), true),
    }
}
