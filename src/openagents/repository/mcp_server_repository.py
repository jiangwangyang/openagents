from datetime import datetime

from sqlalchemy import select, delete, update

from openagents.repository.database import McpServerEntity, async_session


# 查询全部 MCP 服务，按 id 升序
async def list_mcp_servers() -> list[McpServerEntity]:
    async with async_session() as session:
        result = await session.execute(select(McpServerEntity).order_by(McpServerEntity.id))
        return list(result.scalars().all())


# 按 id 查询 MCP 服务，不存在返回 None
async def get_mcp_server(server_id: int) -> McpServerEntity | None:
    async with async_session() as session:
        result = await session.execute(select(McpServerEntity).where(McpServerEntity.id == server_id))
        return result.scalar_one_or_none()


# 新增 MCP 服务，按类型使用 url+headers 或 command+args，名称已存在返回 None，成功返回自增 id
async def add_mcp_server(name: str, description: str, type: str, url: str | None = None, headers: dict[str, str] | None = None, command: str | None = None, args: list[str] | None = None) -> int | None:
    async with async_session() as session:
        result = await session.execute(select(McpServerEntity).where(McpServerEntity.name == name))
        if result.scalar_one_or_none() is not None:
            return None
        now = datetime.now()
        server = McpServerEntity(name=name, description=description, type=type, url=url, headers=headers, command=command, args=args, create_time=now, update_time=now)
        session.add(server)
        await session.commit()
        await session.refresh(server)
        return server.id


# 按 id 更新 MCP 服务，名称被其它记录占用或 id 不存在返回 False
async def update_mcp_server(server_id: int, name: str, description: str, type: str, url: str | None = None, headers: dict[str, str] | None = None, command: str | None = None, args: list[str] | None = None) -> bool:
    async with async_session() as session:
        result = await session.execute(select(McpServerEntity).where(McpServerEntity.name == name, McpServerEntity.id != server_id))
        if result.scalar_one_or_none() is not None:
            return False
        result = await session.execute(
            update(McpServerEntity)
            .where(McpServerEntity.id == server_id)
            .values(name=name, description=description, type=type, url=url, headers=headers, command=command, args=args, update_time=datetime.now())
        )
        await session.commit()
        return result.rowcount > 0


# 按 id 删除 MCP 服务，不存在返回 False
async def delete_mcp_server(server_id: int) -> bool:
    async with async_session() as session:
        result = await session.execute(delete(McpServerEntity).where(McpServerEntity.id == server_id))
        await session.commit()
        return result.rowcount > 0
