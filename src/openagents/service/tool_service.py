import shutil
import sys

from openagents.repository.database import ConversationEntity
from openagents.tool import file_tool, task_tool, mcp_tool, shell_tool, skill_tool

PWSH_DESCRIPTION = """
pwsh -Command <command>                                         # Execute a command in PowerShell 7 on Windows.
"""
POWERSHELL_DESCRIPTION = """
powershell -Command <command>                                   # Execute a command in Windows PowerShell 5.1 on Windows.
"""
BASH_DESCRIPTION = """
bash -c <command>                                               # Execute a command in Bash on Linux/macOS.
"""
DESCRIPTION = f"""
Execute a built-in command. Available commands:
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
{PWSH_DESCRIPTION if sys.platform.startswith("win") and shutil.which("pwsh") else POWERSHELL_DESCRIPTION if sys.platform.startswith("win") else BASH_DESCRIPTION}
"""
COMMAND_TOOL = {
    "name": "command",
    "description": DESCRIPTION,
    "input_schema": {
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
    }
}


# 获取工具描述列表
def list_tools() -> list[dict]:
    return [COMMAND_TOOL]


# 执行选择的工具，下游工具所需参数直接从 conversation 取
async def execute_tool(name: str, tool_input: dict, conversation: ConversationEntity) -> tuple[str, bool]:
    try:
        if name != "command":
            return f"Unknown tool: {name}", True
        if not tool_input.get("cmd_and_args"):
            return "No cmd_and_args", True
        cmd_and_args: list[str] = tool_input["cmd_and_args"]
        if cmd_and_args[0] == "file":
            return await file_tool.execute(cmd_and_args, str(conversation.work_dir))
        if cmd_and_args[0] == "skill":
            return await skill_tool.execute(cmd_and_args)
        if cmd_and_args[0] == "mcp":
            return await mcp_tool.execute(cmd_and_args)
        if cmd_and_args[0] == "task":
            return await task_tool.execute(cmd_and_args, conversation.task_id)
        return await shell_tool.execute(cmd_and_args, str(conversation.work_dir))
    except Exception as e:
        return f"{e}", True
