from fastapi import APIRouter, HTTPException, Body

from openagents.repository import mcp_server_repository

router = APIRouter()


# 查询全部 MCP 服务，按名称升序
@router.get("/mcp-server/list")
async def list_mcp_servers() -> list[dict]:
    servers = await mcp_server_repository.list_mcp_servers()
    return [s.model_dump(mode="json", exclude_none=True) for s in servers]


# 按名称查询 MCP 服务，不存在返回 404
@router.get("/mcp-server/{name}")
async def get_mcp_server(name: str) -> dict:
    server = await mcp_server_repository.get_mcp_server(name)
    if server is None:
        raise HTTPException(status_code=404, detail="MCP server not found")
    return server.model_dump(mode="json", exclude_none=True)


# 新增 streamable_http 类型的 MCP 服务，名称已存在返回 409
@router.post("/mcp-server/streamable-http")
async def add_mcp_streamable_http_server(name: str = Body(..., embed=True), description: str = Body(..., embed=True), url: str = Body(..., embed=True), headers: dict[str, str] | None = Body(None, embed=True)) -> None:
    if not await mcp_server_repository.add_mcp_streamable_http_server(name, description, url, headers):
        raise HTTPException(status_code=409, detail="MCP server already exists")


# 新增 sse 类型的 MCP 服务，名称已存在返回 409
@router.post("/mcp-server/sse")
async def add_mcp_sse_server(name: str = Body(..., embed=True), description: str = Body(..., embed=True), url: str = Body(..., embed=True), headers: dict[str, str] | None = Body(None, embed=True)) -> None:
    if not await mcp_server_repository.add_mcp_sse_server(name, description, url, headers):
        raise HTTPException(status_code=409, detail="MCP server already exists")


# 新增 stdio 类型的 MCP 服务，名称已存在返回 409
@router.post("/mcp-server/stdio")
async def add_mcp_stdio_server(name: str = Body(..., embed=True), description: str = Body(..., embed=True), command: str = Body(..., embed=True), args: list[str] | None = Body(None, embed=True)) -> None:
    if not await mcp_server_repository.add_mcp_stdio_server(name, description, command, args):
        raise HTTPException(status_code=409, detail="MCP server already exists")


# 按名称更新 streamable_http 类型的 MCP 服务，不存在返回 404
@router.put("/mcp-server/{name}/streamable-http")
async def update_mcp_streamable_http_server(name: str, description: str = Body(..., embed=True), url: str = Body(..., embed=True), headers: dict[str, str] | None = Body(None, embed=True)) -> None:
    if not await mcp_server_repository.update_mcp_streamable_http_server(name, description, url, headers):
        raise HTTPException(status_code=404, detail="MCP server not found")


# 按名称更新 sse 类型的 MCP 服务，不存在返回 404
@router.put("/mcp-server/{name}/sse")
async def update_mcp_sse_server(name: str, description: str = Body(..., embed=True), url: str = Body(..., embed=True), headers: dict[str, str] | None = Body(None, embed=True)) -> None:
    if not await mcp_server_repository.update_mcp_sse_server(name, description, url, headers):
        raise HTTPException(status_code=404, detail="MCP server not found")


# 按名称更新 stdio 类型的 MCP 服务，不存在返回 404
@router.put("/mcp-server/{name}/stdio")
async def update_mcp_stdio_server(name: str, description: str = Body(..., embed=True), command: str = Body(..., embed=True), args: list[str] | None = Body(None, embed=True)) -> None:
    if not await mcp_server_repository.update_mcp_stdio_server(name, description, command, args):
        raise HTTPException(status_code=404, detail="MCP server not found")


# 按名称删除 MCP 服务，不存在返回 404
@router.delete("/mcp-server/{name}")
async def delete_mcp_server(name: str) -> None:
    if not await mcp_server_repository.delete_mcp_server(name):
        raise HTTPException(status_code=404, detail="MCP server not found")
