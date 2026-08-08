// MCP 服务 CRUD API
use axum::extract::{Path, State};
use axum::Json;
use rmcp::transport::ConfigureCommandExt;
use serde::Deserialize;

use crate::error::AppError;
use crate::repository::entity::McpServerEntity;
use crate::repository::mcp_server_repository;
use crate::state::AppState;

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

// 新增 streamable_http 类型 MCP 服务请求体
#[derive(Debug, Deserialize)]
pub struct AddMcpStreamableHttpRequest {
    pub name: String,
    pub description: String,
    pub url: String,
    pub headers: Option<serde_json::Value>,
}

// 新增 streamable_http 类型的 MCP 服务，名称已存在返回 409
pub async fn add_mcp_streamable_http_server(State(state): State<AppState>, Json(req): Json<AddMcpStreamableHttpRequest>) -> Result<(), AppError> {
    let result = mcp_server_repository::add_mcp_server(&state.db, &req.name, &req.description, "streamable_http", Some(&req.url), req.headers.as_ref(), None, None).await?;
    if result.is_none() {
        return Err(AppError::Conflict("MCP server already exists".to_string()));
    }
    Ok(())
}

// 新增 stdio 类型 MCP 服务请求体
#[derive(Debug, Deserialize)]
pub struct AddMcpStdioRequest {
    pub name: String,
    pub description: String,
    pub command: String,
    pub args: Option<serde_json::Value>,
}

// 新增 stdio 类型的 MCP 服务，名称已存在返回 409
pub async fn add_mcp_stdio_server(State(state): State<AppState>, Json(req): Json<AddMcpStdioRequest>) -> Result<(), AppError> {
    let result = mcp_server_repository::add_mcp_server(&state.db, &req.name, &req.description, "stdio", None, None, Some(&req.command), req.args.as_ref()).await?;
    if result.is_none() {
        return Err(AppError::Conflict("MCP server already exists".to_string()));
    }
    Ok(())
}

// 更新 streamable_http 类型 MCP 服务请求体
#[derive(Debug, Deserialize)]
pub struct UpdateMcpStreamableHttpRequest {
    pub name: String,
    pub description: String,
    pub url: String,
    pub headers: Option<serde_json::Value>,
}

// 按 id 更新 streamable_http 类型的 MCP 服务，不存在或名称冲突返回 404
pub async fn update_mcp_streamable_http_server(State(state): State<AppState>, Path(server_id): Path<i64>, Json(req): Json<UpdateMcpStreamableHttpRequest>) -> Result<(), AppError> {
    let updated = mcp_server_repository::update_mcp_server(&state.db, server_id, &req.name, &req.description, "streamable_http", Some(&req.url), req.headers.as_ref(), None, None).await?;
    if !updated {
        return Err(AppError::NotFound("MCP server not found".to_string()));
    }
    Ok(())
}

// 更新 stdio 类型 MCP 服务请求体
#[derive(Debug, Deserialize)]
pub struct UpdateMcpStdioRequest {
    pub name: String,
    pub description: String,
    pub command: String,
    pub args: Option<serde_json::Value>,
}

// 按 id 更新 stdio 类型的 MCP 服务，不存在或名称冲突返回 404
pub async fn update_mcp_stdio_server(State(state): State<AppState>, Path(server_id): Path<i64>, Json(req): Json<UpdateMcpStdioRequest>) -> Result<(), AppError> {
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

// MCP 连接测试请求体
#[derive(Debug, Deserialize)]
pub struct TestMcpServerRequest {
    pub url: Option<String>,
    pub headers: Option<serde_json::Value>,
    pub command: Option<String>,
    pub args: Option<serde_json::Value>,
}

// 测试指定类型的 MCP 服务连接，创建会话获取工具列表返回，参数缺失返回 400，连接失败返回 502
pub async fn test_mcp_server(Path(_type): Path<String>, Json(req): Json<TestMcpServerRequest>) -> Result<Json<Vec<serde_json::Value>>, AppError> {
    match _type.as_str() {
        "streamable_http" => {
            let url = req.url.as_deref().ok_or_else(|| AppError::BadRequest("url is required".to_string()))?;
            let mut config = rmcp::transport::streamable_http_client::StreamableHttpClientTransportConfig::with_uri(url);
            if let Some(serde_json::Value::Object(headers_map)) = &req.headers {
                let mut custom_headers = std::collections::HashMap::new();
                for (k, v) in headers_map {
                    if let Some(val_str) = v.as_str() {
                        if let (Ok(header_name), Ok(header_value)) = (
                            k.parse::<http::HeaderName>(),
                            val_str.parse::<http::HeaderValue>(),
                        ) {
                            custom_headers.insert(header_name, header_value);
                        }
                    }
                }
                config = config.custom_headers(custom_headers);
            }
            let transport = rmcp::transport::StreamableHttpClientTransport::from_config(config);
            let client_info = rmcp::model::ClientInfo::new(rmcp::model::ClientCapabilities::default(), rmcp::model::Implementation::new("openagents", env!("CARGO_PKG_VERSION")));
            let service = rmcp::ServiceExt::serve(client_info, transport).await.map_err(|e| AppError::Internal(anyhow::anyhow!("MCP connection failed: {}", e)))?;
            let tools = service.peer().list_all_tools().await.map_err(|e| AppError::Internal(anyhow::anyhow!("MCP list tools failed: {}", e)))?;
            let result: Vec<serde_json::Value> = tools
                .iter()
                .map(|tool| {
                    serde_json::json!({
                        "name": tool.name,
                        "description": tool.description,
                    })
                })
                .collect();
            Ok(Json(result))
        }
        "stdio" => {
            let command = req.command.as_deref().ok_or_else(|| AppError::BadRequest("command is required".to_string()))?;
            let args: Vec<String> = match &req.args {
                Some(serde_json::Value::Array(arr)) => arr.iter().filter_map(|v| v.as_str().map(|s| s.to_string())).collect(),
                _ => Vec::new(),
            };
            let mut cmd = tokio::process::Command::new(command);
            for arg in &args {
                cmd.arg(arg);
            }
            let transport = rmcp::transport::TokioChildProcess::new(cmd.configure(|_c| {})).map_err(|e| AppError::Internal(anyhow::anyhow!("MCP stdio transport failed: {}", e)))?;
            let client_info = rmcp::model::ClientInfo::new(rmcp::model::ClientCapabilities::default(), rmcp::model::Implementation::new("openagents", env!("CARGO_PKG_VERSION")));
            let service = rmcp::ServiceExt::serve(client_info, transport).await.map_err(|e| AppError::Internal(anyhow::anyhow!("MCP connection failed: {}", e)))?;
            let tools = service.peer().list_all_tools().await.map_err(|e| AppError::Internal(anyhow::anyhow!("MCP list tools failed: {}", e)))?;
            let result: Vec<serde_json::Value> = tools
                .iter()
                .map(|tool| {
                    serde_json::json!({
                        "name": tool.name,
                        "description": tool.description,
                    })
                })
                .collect();
            Ok(Json(result))
        }
        _ => Err(AppError::BadRequest("Unknown MCP server type".to_string())),
    }
}
