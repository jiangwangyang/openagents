// MCP 服务 CRUD
use sqlx::SqlitePool;

use super::entity::McpServerEntity;

// 查询全部 MCP 服务，按 id 升序
pub async fn list_mcp_servers(pool: &SqlitePool) -> Result<Vec<McpServerEntity>, sqlx::Error> {
    sqlx::query_as::<_, McpServerEntity>(
        "SELECT id, name, description, protocol_type, url, headers, command, args, create_time, update_time FROM t_mcp_server ORDER BY id",
    )
    .fetch_all(pool)
    .await
}

// 按 id 查询 MCP 服务
pub async fn get_mcp_server(pool: &SqlitePool, server_id: i64) -> Result<Option<McpServerEntity>, sqlx::Error> {
    sqlx::query_as::<_, McpServerEntity>(
        "SELECT id, name, description, protocol_type, url, headers, command, args, create_time, update_time FROM t_mcp_server WHERE id = ?",
    )
    .bind(server_id)
    .fetch_optional(pool)
    .await
}

// 按名称查询 MCP 服务
pub async fn get_mcp_server_by_name(pool: &SqlitePool, name: &str) -> Result<Option<McpServerEntity>, sqlx::Error> {
    sqlx::query_as::<_, McpServerEntity>(
        "SELECT id, name, description, protocol_type, url, headers, command, args, create_time, update_time FROM t_mcp_server WHERE name = ?",
    )
    .bind(name)
    .fetch_optional(pool)
    .await
}

// 新增 MCP 服务，名称已存在返回 None，成功返回自增 id
pub async fn add_mcp_server(pool: &SqlitePool, name: &str, description: &str, protocol_type: &str, url: Option<&str>, headers: Option<&serde_json::Value>, command: Option<&str>, args: Option<&serde_json::Value>) -> Result<Option<i64>, sqlx::Error> {
    let exists: Option<(i64,)> = sqlx::query_as("SELECT id FROM t_mcp_server WHERE name = ?")
        .bind(name)
        .fetch_optional(pool)
        .await?;
    if exists.is_some() {
        return Ok(None);
    }
    let now = chrono::Local::now().to_rfc3339();
    let result = sqlx::query(
        "INSERT INTO t_mcp_server (name, description, protocol_type, url, headers, command, args, create_time, update_time) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)",
    )
    .bind(name)
    .bind(description)
    .bind(protocol_type)
    .bind(url)
    .bind(headers)
    .bind(command)
    .bind(args)
    .bind(&now)
    .bind(&now)
    .execute(pool)
    .await?;
    Ok(Some(result.last_insert_rowid()))
}

// 按 id 更新 MCP 服务，名称被其它记录占用或 id 不存在返回 false
pub async fn update_mcp_server(pool: &SqlitePool, server_id: i64, name: &str, description: &str, protocol_type: &str, url: Option<&str>, headers: Option<&serde_json::Value>, command: Option<&str>, args: Option<&serde_json::Value>) -> Result<bool, sqlx::Error> {
    let conflict: Option<(i64,)> = sqlx::query_as("SELECT id FROM t_mcp_server WHERE name = ? AND id != ?")
        .bind(name)
        .bind(server_id)
        .fetch_optional(pool)
        .await?;
    if conflict.is_some() {
        return Ok(false);
    }
    let now = chrono::Local::now().to_rfc3339();
    let result = sqlx::query(
        "UPDATE t_mcp_server SET name = ?, description = ?, protocol_type = ?, url = ?, headers = ?, command = ?, args = ?, update_time = ? WHERE id = ?",
    )
    .bind(name)
    .bind(description)
    .bind(protocol_type)
    .bind(url)
    .bind(headers)
    .bind(command)
    .bind(args)
    .bind(&now)
    .bind(server_id)
    .execute(pool)
    .await?;
    Ok(result.rows_affected() > 0)
}

// 按 id 删除 MCP 服务，不存在返回 false
pub async fn delete_mcp_server(pool: &SqlitePool, server_id: i64) -> Result<bool, sqlx::Error> {
    let result = sqlx::query("DELETE FROM t_mcp_server WHERE id = ?")
        .bind(server_id)
        .execute(pool)
        .await?;
    Ok(result.rows_affected() > 0)
}
