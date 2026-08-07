import httpx
from fastapi import APIRouter, HTTPException, Body
from mcp import ClientSession, StdioServerParameters
from mcp.client.sse import sse_client
from mcp.client.stdio import stdio_client
from mcp.client.streamable_http import streamable_http_client

from openagents.repository import mcp_server_repository
from openagents.repository.database import McpServerEntity

router = APIRouter()


# 查询全部 MCP 服务，按 id 升序
@router.get("/mcp-server/list", response_model_exclude_none=True)
async def list_mcp_servers() -> list[McpServerEntity]:
    return await mcp_server_repository.list_mcp_servers()


# 按 id 查询 MCP 服务，不存在返回 404
@router.get("/mcp-server/{server_id}", response_model_exclude_none=True)
async def get_mcp_server(server_id: int) -> McpServerEntity:
    server = await mcp_server_repository.get_mcp_server(server_id)
    if server is None:
        raise HTTPException(status_code=404, detail="MCP server not found")
    return server


# 新增 streamable_http 类型的 MCP 服务，名称已存在返回 409
@router.post("/mcp-server/streamable-http")
async def add_mcp_streamable_http_server(name: str = Body(..., embed=True), description: str = Body(..., embed=True), url: str = Body(..., embed=True), headers: dict[str, str] | None = Body(None, embed=True)) -> None:
    if await mcp_server_repository.add_mcp_server(name, description, "streamable_http", url=url, headers=headers) is None:
        raise HTTPException(status_code=409, detail="MCP server already exists")


# 新增 sse 类型的 MCP 服务，名称已存在返回 409
@router.post("/mcp-server/sse")
async def add_mcp_sse_server(name: str = Body(..., embed=True), description: str = Body(..., embed=True), url: str = Body(..., embed=True), headers: dict[str, str] | None = Body(None, embed=True)) -> None:
    if await mcp_server_repository.add_mcp_server(name, description, "sse", url=url, headers=headers) is None:
        raise HTTPException(status_code=409, detail="MCP server already exists")


# 新增 stdio 类型的 MCP 服务，名称已存在返回 409
@router.post("/mcp-server/stdio")
async def add_mcp_stdio_server(name: str = Body(..., embed=True), description: str = Body(..., embed=True), command: str = Body(..., embed=True), args: list[str] | None = Body(None, embed=True)) -> None:
    if await mcp_server_repository.add_mcp_server(name, description, "stdio", command=command, args=args) is None:
        raise HTTPException(status_code=409, detail="MCP server already exists")


# 按 id 更新 streamable_http 类型的 MCP 服务，不存在或名称冲突返回 404
@router.put("/mcp-server/{server_id}/streamable-http")
async def update_mcp_streamable_http_server(server_id: int, name: str = Body(..., embed=True), description: str = Body(..., embed=True), url: str = Body(..., embed=True), headers: dict[str, str] | None = Body(None, embed=True)) -> None:
    if not await mcp_server_repository.update_mcp_server(server_id, name, description, "streamable_http", url=url, headers=headers):
        raise HTTPException(status_code=404, detail="MCP server not found")


# 按 id 更新 sse 类型的 MCP 服务，不存在或名称冲突返回 404
@router.put("/mcp-server/{server_id}/sse")
async def update_mcp_sse_server(server_id: int, name: str = Body(..., embed=True), description: str = Body(..., embed=True), url: str = Body(..., embed=True), headers: dict[str, str] | None = Body(None, embed=True)) -> None:
    if not await mcp_server_repository.update_mcp_server(server_id, name, description, "sse", url=url, headers=headers):
        raise HTTPException(status_code=404, detail="MCP server not found")


# 按 id 更新 stdio 类型的 MCP 服务，不存在或名称冲突返回 404
@router.put("/mcp-server/{server_id}/stdio")
async def update_mcp_stdio_server(server_id: int, name: str = Body(..., embed=True), description: str = Body(..., embed=True), command: str = Body(..., embed=True), args: list[str] | None = Body(None, embed=True)) -> None:
    if not await mcp_server_repository.update_mcp_server(server_id, name, description, "stdio", command=command, args=args):
        raise HTTPException(status_code=404, detail="MCP server not found")


# 按 id 删除 MCP 服务，不存在返回 404
@router.delete("/mcp-server/{server_id}")
async def delete_mcp_server(server_id: int) -> None:
    if not await mcp_server_repository.delete_mcp_server(server_id):
        raise HTTPException(status_code=404, detail="MCP server not found")


# 测试指定类型的 MCP 服务连接，创建会话获取工具列表返回，参数缺失返回 400，连接失败返回 502
@router.post("/mcp-server/{_type}/test")
async def test_mcp_server(_type: str, url: str | None = Body(None, embed=True), headers: dict[str, str] | None = Body(None, embed=True), command: str | None = Body(None, embed=True), args: list[str] | None = Body(None, embed=True)) -> list[dict]:
    # 按协议类型创建客户端
    http_client: httpx.AsyncClient | None = None
    if _type == "streamable_http":
        if not url:
            raise HTTPException(status_code=400, detail="url is required")
        http_client = httpx.AsyncClient(headers=headers, timeout=httpx.Timeout(30.0, read=300.0))
        client = streamable_http_client(url, http_client=http_client)
    elif _type == "sse":
        if not url:
            raise HTTPException(status_code=400, detail="url is required")
        client = sse_client(url, headers)
    elif _type == "stdio":
        if not command:
            raise HTTPException(status_code=400, detail="command is required")
        client = stdio_client(StdioServerParameters(command=command, args=args or []))
    else:
        raise HTTPException(status_code=400, detail="Unknown MCP server type")
    # 建立连接并初始化会话，获取工具列表后关闭连接，无论成功与否均释放 http_client
    try:
        async with client as streams:
            read, write = streams[:2]
            async with ClientSession(read, write) as session:
                await session.initialize()
                list_tools_result = await session.list_tools()
                return [{"name": tool.name, "description": tool.description} for tool in list_tools_result.tools]
    except Exception as e:
        raise HTTPException(status_code=502, detail=f"MCP connection failed: {e}")
    finally:
        if http_client is not None:
            await http_client.aclose()
