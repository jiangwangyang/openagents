// 工具分发(委托给 tool/mod.rs)
use serde_json::Value;

use crate::tool::{self, ToolContext, ToolDefinition, ToolResult};

// 获取工具描述列表
pub fn list_tools() -> Vec<ToolDefinition> {
    tool::list_tools()
}

// 执行选择的工具
pub async fn execute_tool(name: &str, tool_input: &Value, ctx: &ToolContext) -> ToolResult {
    tool::execute_tool(name, tool_input, ctx).await
}
