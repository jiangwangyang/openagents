// MCP 服务 CRUD API
use axum::extract::{Path, State};
use axum::Json;
use serde::Deserialize;

use crate::error::AppError;
use crate::repository::entity::McpServerEntity;
use crate::repository::mcp_server_repository;
use crate::state::AppState;
use crate::tool::mcp_tool;

// 查询全部 MCP 服务，按 id 升序
pub async fn list_mcp_servers(State(state): State<AppState>) -> Result<Json<Vec<McpServerEntity>>, AppError> {
    let servers = mcp_server_repository::list_mcp_servers(&state.db).await?;
    Ok(Json(servers))
}

// 按 id 查询 MCP 服务，不存在返回 404
pub async fn get_mcp_server(State(state): State<AppState>, Path(server_id): Path<i64>) -> Result<Json<McpServerEntity>, AppError> {
    let server = mcp_server_repository::get_mcp_server(&state.db, server_id).await?;
    match server {
        Some(s) => Ok(Json(s)),
        None => Err(AppError::NotFound("MCP server not found".to_string())),
    }
}

// streamable_http 类型 MCP 服务新增/更新请求体
#[derive(Debug, Deserialize)]
pub struct McpStreamableHttpRequest {
    pub name: String,
    pub description: String,
    pub url: String,
    pub headers: Option<serde_json::Value>,
}

// 新增 streamable_http 类型的 MCP 服务，名称已存在返回 409
pub async fn add_mcp_streamable_http_server(State(state): State<AppState>, Json(req): Json<McpStreamableHttpRequest>) -> Result<(), AppError> {
    let result = mcp_server_repository::add_mcp_server(&state.db, &req.name, &req.description, "streamable_http", Some(&req.url), req.headers.as_ref(), None, None).await?;
    if result.is_none() {
        return Err(AppError::Conflict("MCP server already exists".to_string()));
    }
    Ok(())
}

// stdio 类型 MCP 服务新增/更新请求体
#[derive(Debug, Deserialize)]
pub struct McpStdioRequest {
    pub name: String,
    pub description: String,
    pub command: String,
    pub args: Option<serde_json::Value>,
}

// 新增 stdio 类型的 MCP 服务，名称已存在返回 409
pub async fn add_mcp_stdio_server(State(state): State<AppState>, Json(req): Json<McpStdioRequest>) -> Result<(), AppError> {
    let result = mcp_server_repository::add_mcp_server(&state.db, &req.name, &req.description, "stdio", None, None, Some(&req.command), req.args.as_ref()).await?;
    if result.is_none() {
        return Err(AppError::Conflict("MCP server already exists".to_string()));
    }
    Ok(())
}

// 按 id 更新 streamable_http 类型的 MCP 服务，不存在或名称冲突返回 404
pub async fn update_mcp_streamable_http_server(State(state): State<AppState>, Path(server_id): Path<i64>, Json(req): Json<McpStreamableHttpRequest>) -> Result<(), AppError> {
    let updated = mcp_server_repository::update_mcp_server(&state.db, server_id, &req.name, &req.description, "streamable_http", Some(&req.url), req.headers.as_ref(), None, None).await?;
    if !updated {
        return Err(AppError::NotFound("MCP server not found".to_string()));
    }
    Ok(())
}

// 按 id 更新 stdio 类型的 MCP 服务，不存在或名称冲突返回 404
pub async fn update_mcp_stdio_server(State(state): State<AppState>, Path(server_id): Path<i64>, Json(req): Json<McpStdioRequest>) -> Result<(), AppError> {
    let updated = mcp_server_repository::update_mcp_server(&state.db, server_id, &req.name, &req.description, "stdio", None, None, Some(&req.command), req.args.as_ref()).await?;
    if !updated {
        return Err(AppError::NotFound("MCP server not found".to_string()));
    }
    Ok(())
}

// 按 id 删除 MCP 服务，不存在返回 404
pub async fn delete_mcp_server(State(state): State<AppState>, Path(server_id): Path<i64>) -> Result<(), AppError> {
    let deleted = mcp_server_repository::delete_mcp_server(&state.db, server_id).await?;
    if !deleted {
        return Err(AppError::NotFound("MCP server not found".to_string()));
    }
    Ok(())
}

// 按 id 连接数据库中的 MCP 服务，创建会话获取工具列表返回，不存在返回 404，连接失败返回 500
pub async fn list_mcp_server_tools(State(state): State<AppState>, Path(server_id): Path<i64>) -> Result<Json<Vec<serde_json::Value>>, AppError> {
    // 先校验服务存在，不存在返回 404(连接内部的查询不再区分该场景)
    let server = mcp_server_repository::get_mcp_server(&state.db, server_id).await?;
    if server.is_none() {
        return Err(AppError::NotFound("MCP server not found".to_string()));
    }
    let service = mcp_tool::connect_mcp_server(&state.db, server_id)
        .await
        .map_err(|e| AppError::Internal(anyhow::anyhow!("MCP connect failed: {}", e)))?;
    // 获取工具列表
    let tools = match service.peer().list_all_tools().await {
        Ok(t) => t,
        Err(e) => {
            let _ = service.cancel().await;
            return Err(AppError::Internal(anyhow::anyhow!("MCP list tools failed: {}", e)));
        }
    };
    let result: Vec<serde_json::Value> = tools
        .iter()
        .map(|tool| {
            serde_json::json!({
                "name": tool.name,
                "description": tool.description,
            })
        })
        .collect();
    // 获取完毕，关闭连接
    let _ = service.cancel().await;
    Ok(Json(result))
}
