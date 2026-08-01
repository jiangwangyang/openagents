from openagents.repository.setting import McpServer, load_setting, save_setting


# 查询全部 MCP 服务，按名称升序
async def list_mcp_servers() -> list[McpServer]:
    setting = await load_setting()
    return list(setting.mcp_servers.values())


# 按名称查询 MCP 服务，不存在返回 None
async def get_mcp_server(name: str) -> McpServer | None:
    setting = await load_setting()
    return setting.mcp_servers.get(name)


# 新增 MCP 服务，名称已存在返回 False
async def add_mcp_server(name: str, type: str | None = None, url: str | None = None, headers: dict[str, str] | None = None, command: str | None = None, args: list[str] | None = None) -> bool:
    setting = await load_setting()
    if name in setting.mcp_servers:
        return False
    setting.mcp_servers[name] = McpServer(name=name, type=type, url=url, headers=headers, command=command, args=args)
    await save_setting(setting)
    return True


# 按名称更新 MCP 服务，不存在返回 False
async def update_mcp_server(name: str, type: str | None = None, url: str | None = None, headers: dict[str, str] | None = None, command: str | None = None, args: list[str] | None = None) -> bool:
    setting = await load_setting()
    if name not in setting.mcp_servers:
        return False
    setting.mcp_servers[name] = McpServer(name=name, type=type, url=url, headers=headers, command=command, args=args)
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
