// MCP 客户端管理工具
use std::collections::HashMap;

use rmcp::model::{CallToolRequestParams, ClientInfo};
use rmcp::service::{RoleClient, RunningService};
use rmcp::transport::{ConfigureCommandExt, StreamableHttpClientTransport, TokioChildProcess};
use rmcp::ServiceExt;
use serde_json::Value;
use sqlx::SqlitePool;

use super::ToolResult;
use crate::repository::mcp_server_repository;

// 连接 MCP 服务: 按 id 从数据库读取配置即时创建客户端, 获取工具与调用工具由调用方执行, 使用完毕后由调用方 cancel
pub async fn connect_mcp_server(
    db: &SqlitePool,
    server_id: i64,
) -> Result<RunningService<RoleClient, ClientInfo>, String> {
    let server = mcp_server_repository::get_mcp_server(db, server_id)
        .await
        .map_err(|e| format!("failed to get MCP server: {}", e))?
        .ok_or_else(|| "MCP server not found".to_string())?;

    // 按协议类型创建 transport
    let service = match server.protocol_type.as_str() {
        "stdio" => {
            let command = server
                .command
                .as_deref()
                .ok_or_else(|| "missing command".to_string())?;
            let args: Vec<String> = match &server.args {
                Some(Value::Array(arr)) => arr
                    .iter()
                    .filter_map(|v| v.as_str().map(|s| s.to_string()))
                    .collect(),
                _ => Vec::new(),
            };
            let mut cmd = tokio::process::Command::new(command);
            for arg in &args {
                cmd.arg(arg);
            }
            // Windows 下隐藏子进程控制台窗口, 避免桌面模式调用外部模型时弹出黑框
            #[cfg(windows)]
            {
                cmd.creation_flags(0x08000000);
            }
            let transport = TokioChildProcess::new(cmd.configure(|_c| {}))
                .map_err(|e| format!("failed to create stdio transport: {}", e))?;
            ClientInfo::default()
                .serve(transport)
                .await
                .map_err(|e| format!("failed to connect: {}", e))?
        }
        "streamable_http" => {
            let url = server
                .url
                .as_deref()
                .ok_or_else(|| "missing url".to_string())?;
            let mut config = rmcp::transport::streamable_http_client::StreamableHttpClientTransportConfig::with_uri(url);
            if let Some(Value::Object(headers_map)) = &server.headers {
                let mut custom_headers = HashMap::new();
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
            let transport = StreamableHttpClientTransport::from_config(config);
            ClientInfo::default()
                .serve(transport)
                .await
                .map_err(|e| format!("failed to connect: {}", e))?
        }
        _ => {
            return Err(format!(
                "unsupported protocol type: {}",
                server.protocol_type
            ))
        }
    };

    Ok(service)
}

// 解析 server_id 并连接 MCP 服务, 失败转为 ToolResult
async fn connect_by_id(
    db: &SqlitePool,
    server_id: &str,
) -> Result<RunningService<RoleClient, ClientInfo>, ToolResult> {
    let server_id = server_id
        .parse::<i64>()
        .map_err(|_| (format!("Invalid server id {}", server_id), true))?;
    connect_mcp_server(db, server_id).await.map_err(|e| {
        (
            format!("Failed to connect MCP server {}: {}", server_id, e),
            true,
        )
    })
}

// 执行 MCP 命令, 用到某个客户端时按 id 从数据库读取配置即时创建, 使用完后直接 cancel
pub async fn execute(cmd_and_args: &[String], db: &SqlitePool) -> ToolResult {
    let args: Vec<&str> = cmd_and_args.iter().map(String::as_str).collect();
    match args.as_slice() {
        // mcp server list
        ["mcp", "server", "list"] => {
            let servers = match mcp_server_repository::list_mcp_servers(db).await {
                Ok(s) => s,
                Err(e) => return (format!("Failed to list MCP servers: {}", e), true),
            };
            let result: Vec<serde_json::Value> = servers
                .iter()
                .map(|server| {
                    serde_json::json!({
                        "id": server.id,
                        "name": server.name,
                        "description": server.description,
                    })
                })
                .collect();
            (serde_json::to_string(&result).unwrap_or_default(), false)
        }
        // mcp server <server_id> tool list
        ["mcp", "server", server_id, "tool", "list"] => {
            let service = match connect_by_id(db, server_id).await {
                Ok(s) => s,
                Err(r) => return r,
            };
            // 先算出结果再统一关闭连接, 避免各提前返回分支重复 cancel
            let result = async {
                match service.peer().list_all_tools().await {
                    Ok(tools) => {
                        let tools: Vec<serde_json::Value> = tools
                            .iter()
                            .map(|tool| {
                                serde_json::json!({
                                    "name": tool.name,
                                    "description": tool.description,
                                })
                            })
                            .collect();
                        (serde_json::to_string(&tools).unwrap_or_default(), false)
                    }
                    Err(e) => (
                        format!("Failed to list tools of MCP server {}: {}", server_id, e),
                        true,
                    ),
                }
            }
            .await;
            // 使用完毕, 关闭连接
            let _ = service.cancel().await;
            result
        }
        // mcp server <server_id> tool <tool_name> info
        ["mcp", "server", server_id, "tool", tool_name, "info"] => {
            let service = match connect_by_id(db, server_id).await {
                Ok(s) => s,
                Err(r) => return r,
            };
            // 先算出结果再统一关闭连接, 避免各提前返回分支重复 cancel
            let result = async {
                match service.peer().list_all_tools().await {
                    Err(e) => (
                        format!("Failed to list tools of MCP server {}: {}", server_id, e),
                        true,
                    ),
                    Ok(tools) => match tools.iter().find(|t| t.name.as_ref() == *tool_name) {
                        Some(tool) => {
                            let result = serde_json::json!({
                                "name": tool.name,
                                "description": tool.description,
                                "input_schema": tool.input_schema,
                            });
                            (serde_json::to_string(&result).unwrap_or_default(), false)
                        }
                        None => (format!("Unknown tool {}", tool_name), true),
                    },
                }
            }
            .await;
            // 使用完毕, 关闭连接
            let _ = service.cancel().await;
            result
        }
        // mcp server <server_id> tool <tool_name> call <tool_json_args>
        ["mcp", "server", server_id, "tool", tool_name, "call", json_string] => {
            let service = match connect_by_id(db, server_id).await {
                Ok(s) => s,
                Err(r) => return r,
            };
            // 先算出结果再统一关闭连接, 避免各提前返回分支重复 cancel
            let result = async {
                // 空参数视为无参数, 否则解析 JSON 对象
                let arguments = if json_string.is_empty() {
                    Ok(None)
                } else {
                    serde_json::from_str::<serde_json::Map<String, Value>>(json_string)
                        .map(Some)
                        .map_err(|e| (format!("Invalid JSON arguments: {}", e), true))
                };
                match arguments {
                    Err(r) => r,
                    Ok(arguments) => {
                        let mut params = CallToolRequestParams::new((*tool_name).to_string());
                        if let Some(args) = arguments {
                            params = params.with_arguments(args);
                        }
                        match service.peer().call_tool(params).await {
                            Ok(tool_result) => {
                                let tool_content_list: Vec<String> = tool_result
                                    .content
                                    .iter()
                                    .map(|content| match content {
                                        rmcp::model::ContentBlock::Text(text) => text.text.clone(),
                                        other => format!("{:?}", other),
                                    })
                                    .collect();
                                let tool_content = if tool_content_list.len() == 1 {
                                    tool_content_list.into_iter().next().unwrap_or_default()
                                } else {
                                    serde_json::to_string(&tool_content_list).unwrap_or_default()
                                };
                                (tool_content, tool_result.is_error.unwrap_or(false))
                            }
                            Err(e) => (format!("Tool call failed: {}", e), true),
                        }
                    }
                }
            }
            .await;
            // 使用完毕, 关闭连接
            let _ = service.cancel().await;
            result
        }
        _ => ("Unknown command".to_string(), true),
    }
}
