// MCP 客户端管理工具
use std::collections::HashMap;

use dashmap::DashMap;
use rmcp::model::{CallToolRequestParams, ClientCapabilities, ClientInfo, Implementation, Tool};
use rmcp::service::{Peer, RoleClient, RunningService};
use rmcp::transport::{ConfigureCommandExt, StreamableHttpClientTransport, TokioChildProcess};
use rmcp::ServiceExt;
use serde_json::Value;
use sqlx::SqlitePool;

use crate::repository::mcp_server_repository;

// MCP 服务信息
pub struct McpServerInfo {
    pub description: String,
    pub peer: Peer<RoleClient>,
    pub tool_dict: HashMap<String, Tool>,
}

// 存储所有的 MCP 信息
pub static MCP_DICT: std::sync::LazyLock<DashMap<String, McpServerInfo>> = std::sync::LazyLock::new(DashMap::new);

// 存储 RunningService 的句柄，用于保持连接
static MCP_HANDLES: std::sync::LazyLock<DashMap<String, tokio::task::JoinHandle<()>>> = std::sync::LazyLock::new(DashMap::new);

// 连接 MCP 服务并获取工具列表，返回 peer、工具表与保持连接的服务句柄
pub async fn connect_mcp_server(
    protocol_type: &str,
    url: Option<&str>,
    headers: Option<&Value>,
    command: Option<&str>,
    args: Option<&Value>,
) -> Result<(Peer<RoleClient>, HashMap<String, Tool>, RunningService<RoleClient, ClientInfo>), String> {
    // 创建客户端信息
    let client_info = ClientInfo::new(ClientCapabilities::default(), Implementation::new("openagents", env!("CARGO_PKG_VERSION")));

    // 按协议类型创建 transport
    let service = match protocol_type {
        "stdio" => {
            let command = command.ok_or_else(|| "missing command".to_string())?;
            let args: Vec<String> = match args {
                Some(Value::Array(arr)) => arr.iter().filter_map(|v| v.as_str().map(|s| s.to_string())).collect(),
                _ => Vec::new(),
            };
            let mut cmd = tokio::process::Command::new(command);
            for arg in &args {
                cmd.arg(arg);
            }
            // Windows 下隐藏子进程控制台窗口,避免桌面模式调用外部模型时弹出黑框
            #[cfg(windows)]
            {
                cmd.creation_flags(0x08000000);
            }
            let transport = TokioChildProcess::new(cmd.configure(|_c| {}))
                .map_err(|e| format!("failed to create stdio transport: {}", e))?;
            client_info
                .serve(transport)
                .await
                .map_err(|e| format!("failed to connect: {}", e))?
        }
        "streamable_http" => {
            let url = url.ok_or_else(|| "missing url".to_string())?;
            let mut config = rmcp::transport::streamable_http_client::StreamableHttpClientTransportConfig::with_uri(url);
            if let Some(Value::Object(headers_map)) = headers {
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
            client_info
                .serve(transport)
                .await
                .map_err(|e| format!("failed to connect: {}", e))?
        }
        _ => return Err(format!("unsupported protocol type: {}", protocol_type)),
    };

    // 获取工具列表
    let peer = service.peer().clone();
    let tools = peer
        .list_all_tools()
        .await
        .map_err(|e| format!("failed to list tools: {}", e))?;
    let tool_dict: HashMap<String, Tool> = tools.into_iter().map(|t| (t.name.to_string(), t)).collect();

    Ok((peer, tool_dict, service))
}

// 初始化所有 MCP 客户端
pub async fn init_mcp_clients(db: &SqlitePool) {
    let servers = match mcp_server_repository::list_mcp_servers(db).await {
        Ok(s) => s,
        Err(e) => {
            tracing::error!("Failed to list MCP servers: {}", e);
            return;
        }
    };

    for server in servers {
        let name = server.name.clone();
        let description = server.description.clone();

        match connect_mcp_server(
            &server.protocol_type,
            server.url.as_deref(),
            server.headers.as_ref(),
            server.command.as_deref(),
            server.args.as_ref(),
        )
        .await
        {
            Ok((peer, tool_dict, service)) => {
                let count = tool_dict.len();
                MCP_DICT.insert(name.clone(), McpServerInfo { description, peer, tool_dict });
                tracing::info!("MCP client {} started, having {} tools", name, count);
                // 保持 service 运行
                let handle = tokio::spawn(async move {
                    let _ = service.waiting().await;
                });
                MCP_HANDLES.insert(name, handle);
            }
            Err(e) => {
                tracing::error!("Failed to start MCP client {}: {}", name, e);
            }
        }
    }
}

// 执行 MCP 命令
pub async fn execute(cmd_and_args: &[String]) -> (String, bool) {
    // 1. mcp server list
    if cmd_and_args.len() == 3 && cmd_and_args[0] == "mcp" && cmd_and_args[1] == "server" && cmd_and_args[2] == "list" {
        let result: Vec<serde_json::Value> = MCP_DICT
            .iter()
            .map(|entry| {
                serde_json::json!({
                    "name": entry.key(),
                    "description": entry.value().description,
                })
            })
            .collect();
        return (serde_json::to_string(&result).unwrap_or_default(), false);
    }
    // 2. mcp server <server_name> tool list
    else if cmd_and_args.len() == 5 && cmd_and_args[0] == "mcp" && cmd_and_args[1] == "server" && cmd_and_args[3] == "tool" && cmd_and_args[4] == "list" {
        let server_name = &cmd_and_args[2];
        let entry = match MCP_DICT.get(server_name) {
            Some(e) => e,
            None => return (format!("Unknown server {}", server_name), true),
        };
        let result: Vec<serde_json::Value> = entry
            .tool_dict
            .values()
            .map(|tool| {
                serde_json::json!({
                    "name": tool.name,
                    "description": tool.description,
                })
            })
            .collect();
        return (serde_json::to_string(&result).unwrap_or_default(), false);
    }
    // 3. mcp server <server_name> tool <tool_name> info
    else if cmd_and_args.len() == 6 && cmd_and_args[0] == "mcp" && cmd_and_args[1] == "server" && cmd_and_args[3] == "tool" && cmd_and_args[5] == "info" {
        let server_name = &cmd_and_args[2];
        let tool_name = &cmd_and_args[4];
        let entry = match MCP_DICT.get(server_name) {
            Some(e) => e,
            None => return (format!("Unknown server {}", server_name), true),
        };
        let tool = match entry.tool_dict.get(tool_name) {
            Some(t) => t,
            None => return (format!("Unknown tool {}", tool_name), true),
        };
        let result = serde_json::json!({
            "name": tool.name,
            "description": tool.description,
            "input_schema": tool.input_schema,
        });
        return (serde_json::to_string(&result).unwrap_or_default(), false);
    }
    // 4. mcp server <server_name> tool <tool_name> call [tool_json_args]
    else if cmd_and_args.len() == 7 && cmd_and_args[0] == "mcp" && cmd_and_args[1] == "server" && cmd_and_args[3] == "tool" && cmd_and_args[5] == "call" {
        let server_name = &cmd_and_args[2];
        let tool_name = &cmd_and_args[4];
        let json_string = &cmd_and_args[6];
        let entry = match MCP_DICT.get(server_name) {
            Some(e) => e,
            None => return (format!("Unknown server {}", server_name), true),
        };
        if !entry.tool_dict.contains_key(tool_name) {
            return (format!("Unknown tool {}", tool_name), true);
        }
        let arguments: Option<rmcp::model::JsonObject> = if json_string.is_empty() {
            None
        } else {
            match serde_json::from_str::<serde_json::Map<String, Value>>(json_string) {
                Ok(map) => Some(map),
                Err(e) => return (format!("Invalid JSON arguments: {}", e), true),
            }
        };
        let mut params = CallToolRequestParams::new(tool_name.clone());
        if let Some(args) = arguments {
            params = params.with_arguments(args);
        }
        match entry.peer.call_tool(params).await {
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
                let is_error = tool_result.is_error.unwrap_or(false);
                return (tool_content, is_error);
            }
            Err(e) => {
                return (format!("Tool call failed: {}", e), true);
            }
        }
    }
    ("Unknown command".to_string(), true)
}
