// 内置工具实现
pub mod file_tool;
pub mod mcp_tool;
pub mod shell_tool;
pub mod skill_tool;
pub mod task_tool;

use serde_json::Value;
use sqlx::SqlitePool;

use crate::state::SkillInfo;
#[cfg(windows)]
use std::os::windows::process::CommandExt;

// 工具执行上下文
#[derive(Clone)]
pub struct ToolContext {
    pub db: SqlitePool,
    pub work_dir: String,
    pub task_id: Option<i64>,
    pub skills: std::sync::Arc<std::sync::RwLock<Vec<SkillInfo>>>,
}

// 工具执行结果: (内容, 是否错误)
pub type ToolResult = (String, bool);

// 工具描述
#[derive(Debug, Clone, serde::Serialize)]
pub struct ToolDefinition {
    pub name: String,
    pub description: String,
    pub input_schema: Value,
}

// 获取工具描述列表
pub fn list_tools() -> Vec<ToolDefinition> {
    vec![command_tool_definition()]
}

// command 工具定义
fn command_tool_definition() -> ToolDefinition {
    let shell_desc = if cfg!(windows) {
        if which_pwsh() {
            "pwsh -Command <command>                                         # Execute a command in PowerShell 7 on Windows."
        } else {
            "powershell -Command <command>                                   # Execute a command in Windows PowerShell 5.1 on Windows."
        }
    } else {
        "bash -c <command>                                                   # Execute a command in Bash on Linux/macOS."
    };

    let description = format!(
        "Execute a built-in command. Available commands:
file read <path>                                                # Read file content from <path>.
file write <path> <content>                                     # Write <content> to <path>. Creates the file if it doesn't exist, overwrites if it does.
file edit <path> <old_str> <new_str>                            # Replace all exact matches of <old_str> with <new_str> in <path>.
skill list                                                      # List all available skills.
mcp server list                                                 # List all MCP servers.
mcp server <server_name> tool list                              # List all tools of a specific MCP server.
mcp server <server_name> tool <tool_name> info                  # Show parameter format of a specific tool.
mcp server <server_name> tool <tool_name> call <tool_json_args> # Call a specific tool with JSON arguments.
task handover <agent_id>                                        # Hand over the task to the specified agent.
task handover user                                              # Hand over the task to the user.
{}",
        shell_desc
    );

    ToolDefinition {
        name: "command".to_string(),
        description,
        input_schema: serde_json::json!({
            "type": "object",
            "properties": {
                "cmd_and_args": {
                    "type": "array",
                    "description": "Command and its arguments as an array. The first element is the command name, followed by the corresponding arguments. Example: [\"file\", \"read\", \"/path/to/file.txt\"]",
                    "items": {
                        "type": "string"
                    }
                }
            },
            "required": ["cmd_and_args"]
        }),
    }
}

// 检查 pwsh 是否可用
fn which_pwsh() -> bool {
    let mut cmd = std::process::Command::new("pwsh");
    cmd.arg("--version")
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null());
    // Windows 下隐藏子进程控制台窗口,避免桌面模式调用外部模型时弹出黑框
    #[cfg(windows)]
    {
        cmd.creation_flags(0x08000000);
    }
    cmd.status()
        .map(|s| s.success())
        .unwrap_or(false)
}

// 执行选择的工具
pub async fn execute_tool(name: &str, tool_input: &Value, ctx: &ToolContext) -> ToolResult {
    if name != "command" {
        return (format!("Unknown tool: {}", name), true);
    }

    let cmd_and_args: Vec<String> = match tool_input.get("cmd_and_args") {
        Some(Value::Array(arr)) => arr
            .iter()
            .filter_map(|v| v.as_str().map(|s| s.to_string()))
            .collect(),
        _ => return ("No cmd_and_args".to_string(), true),
    };

    if cmd_and_args.is_empty() {
        return ("No cmd_and_args".to_string(), true);
    }

    match cmd_and_args[0].as_str() {
        "file" => file_tool::execute(&cmd_and_args, &ctx.work_dir).await,
        "skill" => skill_tool::execute(&cmd_and_args, &ctx.skills),
        "mcp" => mcp_tool::execute(&cmd_and_args, &ctx.db).await,
        "task" => task_tool::execute(&cmd_and_args, ctx.task_id, &ctx.db).await,
        _ => shell_tool::execute(&cmd_and_args, &ctx.work_dir).await,
    }
}
