import json
import logging
from contextlib import asynccontextmanager, AsyncExitStack

import httpx
from mcp import ClientSession, StdioServerParameters
from mcp.client.sse import sse_client
from mcp.client.stdio import stdio_client
from mcp.client.streamable_http import streamable_http_client
from mcp.types import TextContent, Tool, ListToolsResult
from pydantic import BaseModel, ConfigDict

from openagents.repository import mcp_server_repository


# MCP数据结构，session 与 tool_dict 为第三方任意类型
class McpServerInfo(BaseModel):
    model_config = ConfigDict(arbitrary_types_allowed=True)

    description: str
    session: ClientSession
    tool_dict: dict[str, Tool]


# 存储所有的MCP信息
MCP_DICT: dict[str, McpServerInfo] = {}


@asynccontextmanager
async def _register_mcp_client(name: str, description: str, proto_type: str, arg_dict: dict):
    # 创建客户端
    if proto_type == "streamable_http":
        http_client = httpx.AsyncClient(headers=arg_dict.get("headers"), timeout=httpx.Timeout(30.0, read=300.0))
        client = streamable_http_client(arg_dict["url"], http_client=http_client)
    elif proto_type == "sse":
        client = sse_client(arg_dict["url"], arg_dict.get("headers"))
    elif proto_type == "stdio":
        client = stdio_client(StdioServerParameters(command=arg_dict["command"], args=arg_dict["args"]))
    else:
        raise ValueError(f"Unknown proto type: {proto_type}")
    # 建立连接
    async with client as streams:
        read, write = streams[:2]
        async with ClientSession(read, write) as session:
            _ = await session.initialize()
            # 获取工具列表
            list_tools_result: ListToolsResult = await session.list_tools()
            MCP_DICT[name] = McpServerInfo(description=description, session=session, tool_dict={tool.name: tool for tool in list_tools_result.tools})
            logging.info(f"MCP client {name} started, having {len(list_tools_result.tools)} tools")
            # 等待
            yield
            # 结束
            MCP_DICT.pop(name)
            logging.info(f"MCP client {name} stopped")


@asynccontextmanager
async def lifespan():
    mcp_servers = await mcp_server_repository.list_mcp_servers()
    mcp_clients = [_register_mcp_client(server.name, server.description, server.type, server.model_dump(mode="json", exclude_none=True)) for server in mcp_servers]
    async with AsyncExitStack() as stack:
        for client in mcp_clients:
            try:
                await stack.enter_async_context(client)
            except Exception as e:
                if hasattr(e, "exceptions"):
                    logging.error(f"Error registering mcp client: {e.exceptions}")
                else:
                    logging.error(f"Error registering mcp client: {e}")
        yield


# 执行
async def execute(cmd_and_args: list[str], work_dir: str) -> tuple[str, bool]:
    # 1. mcp server list
    if len(cmd_and_args) == 3 and cmd_and_args[0] == "mcp" and cmd_and_args[1] == "server" and cmd_and_args[2] == "list":
        result = [{"name": name, "description": mcp_server_info.description} for name, mcp_server_info in MCP_DICT.items()]
        return json.dumps(result, ensure_ascii=False), False
    # 2. mcp server <server_name> tool list
    elif len(cmd_and_args) == 5 and cmd_and_args[0] == "mcp" and cmd_and_args[1] == "server" and cmd_and_args[3] == "tool" and cmd_and_args[4] == "list":
        server_name = cmd_and_args[2]
        if not server_name in MCP_DICT:
            return f"Unknown server {server_name}", True
        result = [{"name": tool.name, "description": tool.description} for tool in MCP_DICT[server_name].tool_dict.values()]
        return json.dumps(result, ensure_ascii=False), False
    # 3. mcp server <server_name> tool <tool_name> info
    elif len(cmd_and_args) == 6 and cmd_and_args[0] == "mcp" and cmd_and_args[1] == "server" and cmd_and_args[3] == "tool" and cmd_and_args[5] == "info":
        server_name, tool_name = cmd_and_args[2], cmd_and_args[4]
        if not server_name in MCP_DICT:
            return f"Unknown server {server_name}", True
        if not tool_name in MCP_DICT[server_name].tool_dict:
            return f"Unknown tool {tool_name}", True
        tool = MCP_DICT[server_name].tool_dict[tool_name]
        result = {"name": tool.name, "description": tool.description, "input_schema": tool.inputSchema}
        return json.dumps(result, ensure_ascii=False), False
    # 4. mcp server <server_name> tool <tool_name> call [tool_json_args]
    elif len(cmd_and_args) == 7 and cmd_and_args[0] == "mcp" and cmd_and_args[1] == "server" and cmd_and_args[3] == "tool" and cmd_and_args[5] == "call":
        server_name, tool_name, json_string = cmd_and_args[2], cmd_and_args[4], cmd_and_args[6]
        if not server_name in MCP_DICT:
            return f"Unknown server {server_name}", True
        if not tool_name in MCP_DICT[server_name].tool_dict:
            return f"Unknown tool {tool_name}", True
        session = MCP_DICT[server_name].session
        tool_result = await session.call_tool(tool_name, json.loads(json_string) if json_string else {})
        tool_content_list = [content.text if isinstance(content, TextContent) else content.type for content in tool_result.content]
        tool_content = tool_content_list[0] if len(tool_content_list) == 1 else json.dumps(tool_content_list, ensure_ascii=False)
        is_error = tool_result.isError
        return tool_content, is_error
    return "未知命令", True
