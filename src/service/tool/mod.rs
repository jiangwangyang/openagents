// 内置工具: 定义、注册与分发
pub mod agent_tool;
pub mod file_tool;
pub mod mcp_tool;
pub mod model_provider_tool;
pub mod schedule_tool;
pub mod shell_tool;
pub mod skill_tool;
pub mod task_tool;

#[cfg(windows)]
use std::os::windows::process::CommandExt;

use serde_json::Value;

use crate::ai::truncate_str;
use crate::state::AppState;

// 工具执行上下文
#[derive(Clone)]
pub struct ToolContext {
    pub state: AppState,
    pub work_dir: String,
    pub task_id: Option<i64>,
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

// 内置命令帮助(所有对话可用): (命令用法, 说明)
const COMMAND_HELP: &[(&str, &str)] = &[
    ("file read <path>", "Read file content from <path>."),
    (
        "file write <path> <content>",
        "Write <content> to <path>. Creates the file if it doesn't exist, overwrites if it does.",
    ),
    (
        "file edit <path> <old_str> <new_str>",
        "Replace all exact matches of <old_str> with <new_str> in <path>.",
    ),
    ("skill list", "List all available skills."),
    ("mcp server list", "List all MCP servers."),
    (
        "mcp server <server_id> tool list",
        "List all tools of a specific MCP server.",
    ),
    (
        "mcp server <server_id> tool <tool_name> info",
        "Show parameter format of a specific tool.",
    ),
    (
        "mcp server <server_id> tool <tool_name> call <tool_json_args>",
        "Call a specific tool with JSON arguments.",
    ),
];

// 任务交接命令帮助(仅任务阶段对话可用)
const HANDOVER_COMMAND_HELP: &[(&str, &str)] = &[
    (
        "task handover <agent_id>",
        "Hand over the task to the specified agent.",
    ),
    ("task handover user", "Hand over the task to the user."),
];

// 管理命令帮助(仅独立对话可用, 任务/定时执行对话不可用)
const MANAGE_COMMAND_HELP: &[(&str, &str)] = &[
    ("agent list", "List all agents."),
    ("agent get <agent_id>", "Show details of a specific agent."),
    (
        "agent add <name> <description> <prompt> <model_provider_id> <model> <thinking>",
        "Add a new agent. <thinking> is true or false.",
    ),
    (
        "agent update <agent_id> <name> <description> <prompt> <model_provider_id> <model> <thinking>",
        "Update an existing agent. <thinking> is true or false.",
    ),
    (
        "agent delete <agent_id>",
        "Delete an agent. Fails if the agent is referenced by conversations or schedules.",
    ),
    ("task list", "List all tasks."),
    ("task get <task_id>", "Show details of a specific task."),
    (
        "task add <title> <content> <agent_ids> <work_dir>",
        "Add a new task. <agent_ids> is a JSON array of agent ids, e.g. [1,2].",
    ),
    (
        "task update <task_id> <title> <content> <agent_ids> <work_dir>",
        "Update an existing task. <agent_ids> is a JSON array of agent ids, e.g. [1,2].",
    ),
    (
        "task delete <task_id>",
        "Delete a task. Fails if the task is running.",
    ),
    ("schedule list", "List all schedules."),
    (
        "schedule get <schedule_id>",
        "Show details of a specific schedule.",
    ),
    (
        "schedule add <name> <content> <work_dir> <cron_expr> <agent_id>",
        "Add a new schedule. <cron_expr> has 6 fields: second minute hour day month day_of_week, e.g. \"0 0 9 * * *\".",
    ),
    (
        "schedule update <schedule_id> <name> <content> <work_dir> <cron_expr> <agent_id> <enabled>",
        "Update an existing schedule. <enabled> is true or false.",
    ),
    ("schedule delete <schedule_id>", "Delete a schedule."),
    (
        "model_provider list",
        "List all model providers (id, name, protocol_type, base_url) without api_key.",
    ),
];

// 工具描述列表缓存, 按对话上下文(独立/任务/定时)各生成一次
static TOOL_DEFINITIONS_STANDALONE: std::sync::LazyLock<Vec<ToolDefinition>> =
    std::sync::LazyLock::new(|| build_tool_definitions(false, false));
static TOOL_DEFINITIONS_TASK: std::sync::LazyLock<Vec<ToolDefinition>> =
    std::sync::LazyLock::new(|| build_tool_definitions(true, false));
static TOOL_DEFINITIONS_SCHEDULE: std::sync::LazyLock<Vec<ToolDefinition>> =
    std::sync::LazyLock::new(|| build_tool_definitions(false, true));

// 获取工具描述列表, has_task 为 true 时包含任务交接命令, has_task 与 has_schedule 均为 false 时包含管理命令
pub fn list_tools(has_task: bool, has_schedule: bool) -> &'static [ToolDefinition] {
    if has_task {
        &TOOL_DEFINITIONS_TASK
    } else if has_schedule {
        &TOOL_DEFINITIONS_SCHEDULE
    } else {
        &TOOL_DEFINITIONS_STANDALONE
    }
}

// 构建工具描述列表
fn build_tool_definitions(has_task: bool, has_schedule: bool) -> Vec<ToolDefinition> {
    let (shell_cmd, shell_desc): (&str, &str) = if cfg!(windows) {
        if which_pwsh() {
            (
                "pwsh -Command <command>",
                "Execute a command in PowerShell 7 on Windows.",
            )
        } else {
            (
                "powershell -Command <command>",
                "Execute a command in Windows PowerShell 5.1 on Windows.",
            )
        }
    } else {
        (
            "bash -c <command>",
            "Execute a command in Bash on Linux/macOS.",
        )
    };

    let mut description = String::from("Execute a built-in command. Available commands:\n");
    for &(cmd, desc) in COMMAND_HELP {
        description.push_str(&format!("{cmd} # {desc}\n"));
    }
    // 任务阶段对话追加任务交接命令
    if has_task {
        for &(cmd, desc) in HANDOVER_COMMAND_HELP {
            description.push_str(&format!("{cmd} # {desc}\n"));
        }
    }
    // 独立对话追加管理命令
    if !has_task && !has_schedule {
        for &(cmd, desc) in MANAGE_COMMAND_HELP {
            description.push_str(&format!("{cmd} # {desc}\n"));
        }
    }
    description.push_str(&format!("{shell_cmd} # {shell_desc}\n"));

    vec![ToolDefinition {
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
    }]
}

// 检查 pwsh 是否可用
fn which_pwsh() -> bool {
    let mut cmd = std::process::Command::new("pwsh");
    cmd.arg("--version")
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null());
    // Windows 下隐藏子进程控制台窗口, 避免桌面模式调用外部模型时弹出黑框
    #[cfg(windows)]
    {
        cmd.creation_flags(0x08000000);
    }
    cmd.status().map(|s| s.success()).unwrap_or(false)
}

// 执行选择的工具
pub async fn execute_tool(name: &str, tool_input: &Value, ctx: &ToolContext) -> ToolResult {
    let cmd_and_args: Vec<String> = tool_input
        .get("cmd_and_args")
        .and_then(Value::as_array)
        .map(|arr| {
            arr.iter()
                .filter_map(|v| v.as_str().map(|s| s.to_string()))
                .collect()
        })
        .unwrap_or_default();

    // 调用摘要: 工具名 + 完整命令, 截断至 200 字符
    let summary = truncate_str(&format!("{} {}", name, cmd_and_args.join(" ")), 200);
    tracing::info!("Tool call: cmd={}", summary);

    let result: ToolResult = if name != "command" {
        (format!("Unknown tool: {}", name), true)
    } else if cmd_and_args.is_empty() {
        ("No cmd_and_args".to_string(), true)
    } else {
        match cmd_and_args[0].as_str() {
            "file" => file_tool::execute(&cmd_and_args, &ctx.work_dir).await,
            "skill" => skill_tool::execute(&cmd_and_args, &ctx.state.skills),
            "mcp" => mcp_tool::execute(&cmd_and_args, &ctx.state.db).await,
            "task" => task_tool::execute(&cmd_and_args, ctx).await,
            "schedule" => schedule_tool::execute(&cmd_and_args, &ctx.state).await,
            "agent" => agent_tool::execute(&cmd_and_args, &ctx.state.db).await,
            "model_provider" => model_provider_tool::execute(&cmd_and_args, &ctx.state.db).await,
            _ => shell_tool::execute(&cmd_and_args, &ctx.work_dir).await,
        }
    };

    if result.1 {
        tracing::warn!(
            "Tool call failed: cmd={} error={}",
            summary,
            truncate_str(&result.0, 500)
        );
    }
    result
}
