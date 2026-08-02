from openagents.repository.setting import McpServer, load_setting, save_setting


# 查询全部 MCP 服务，按名称升序
async def list_mcp_servers() -> list[McpServer]:
    setting = await load_setting()
    return sorted(setting.mcp_servers.values(), key=lambda server: server.name)


# 按名称查询 MCP 服务，不存在返回 None
async def get_mcp_server(name: str) -> McpServer | None:
    setting = await load_setting()
    return setting.mcp_servers.get(name)


# 新增 streamable_http 类型的 MCP 服务，名称已存在返回 False
async def add_mcp_streamable_http_server(name: str, description: str, url: str, headers: dict[str, str] | None = None) -> bool:
    setting = await load_setting()
    if name in setting.mcp_servers:
        return False
    setting.mcp_servers[name] = McpServer(name=name, description=description, type="streamable_http", url=url, headers=headers)
    await save_setting(setting)
    return True


# 新增 sse 类型的 MCP 服务，名称已存在返回 False
async def add_mcp_sse_server(name: str, description: str, url: str, headers: dict[str, str] | None = None) -> bool:
    setting = await load_setting()
    if name in setting.mcp_servers:
        return False
    setting.mcp_servers[name] = McpServer(name=name, description=description, type="sse", url=url, headers=headers)
    await save_setting(setting)
    return True


# 新增 stdio 类型的 MCP 服务，名称已存在返回 False
async def add_mcp_stdio_server(name: str, description: str, command: str, args: list[str] | None = None) -> bool:
    setting = await load_setting()
    if name in setting.mcp_servers:
        return False
    setting.mcp_servers[name] = McpServer(name=name, description=description, type="stdio", command=command, args=args)
    await save_setting(setting)
    return True


# 按名称更新 streamable_http 类型的 MCP 服务，不存在返回 False
async def update_mcp_streamable_http_server(name: str, description: str, url: str, headers: dict[str, str] | None = None) -> bool:
    setting = await load_setting()
    if name not in setting.mcp_servers:
        return False
    setting.mcp_servers[name] = McpServer(name=name, description=description, type="streamable_http", url=url, headers=headers)
    await save_setting(setting)
    return True


# 按名称更新 sse 类型的 MCP 服务，不存在返回 False
async def update_mcp_sse_server(name: str, description: str, url: str, headers: dict[str, str] | None = None) -> bool:
    setting = await load_setting()
    if name not in setting.mcp_servers:
        return False
    setting.mcp_servers[name] = McpServer(name=name, description=description, type="sse", url=url, headers=headers)
    await save_setting(setting)
    return True


# 按名称更新 stdio 类型的 MCP 服务，不存在返回 False
async def update_mcp_stdio_server(name: str, description: str, command: str, args: list[str] | None = None) -> bool:
    setting = await load_setting()
    if name not in setting.mcp_servers:
        return False
    setting.mcp_servers[name] = McpServer(name=name, description=description, type="stdio", command=command, args=args)
    await save_setting(setting)
    return True


# 按名称删除 MCP 服务，不存在返回 False
async def delete_mcp_server(name: str) -> bool:
    setting = await load_setting()
    if name not in setting.mcp_servers:
        return False
    del setting.mcp_servers[name]
    await save_setting(setting)
    return True
