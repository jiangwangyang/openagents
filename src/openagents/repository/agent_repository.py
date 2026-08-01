from datetime import datetime

from sqlalchemy import select, delete, update

from openagents.repository.database import AgentEntity, async_session


# 查询全部 Agent，按 id 升序
async def list_agents() -> list[AgentEntity]:
    async with async_session() as session:
        result = await session.execute(select(AgentEntity).order_by(AgentEntity.id))
        return list(result.scalars().all())


# 按 id 查询 Agent，不存在返回 None
async def get_agent(agent_id: int) -> AgentEntity | None:
    async with async_session() as session:
        result = await session.execute(select(AgentEntity).where(AgentEntity.id == agent_id))
        return result.scalar_one_or_none()


# 新增 Agent，返回自增 id
async def add_agent(agent: AgentEntity) -> int:
    async with async_session() as session:
        session.add(agent)
        await session.commit()
        await session.refresh(agent)
        return agent.id


# 按 agent.id 更新 Agent 的 name/description/prompt，update_time 刷新为当前时间，id 不存在返回 False
async def update_agent(agent: AgentEntity) -> bool:
    async with async_session() as session:
        result = await session.execute(
            update(AgentEntity)
            .where(AgentEntity.id == agent.id)
            .values(name=agent.name, description=agent.description, prompt=agent.prompt, update_time=datetime.now())
        )
        await session.commit()
        return result.rowcount > 0


# 按 id 删除 Agent，不存在返回 False，关联对话的 agent_id 由数据库外键 ON DELETE SET NULL 自动置空
async def delete_agent(agent_id: int) -> bool:
    async with async_session() as session:
        result = await session.execute(delete(AgentEntity).where(AgentEntity.id == agent_id))
        await session.commit()
        return result.rowcount > 0
