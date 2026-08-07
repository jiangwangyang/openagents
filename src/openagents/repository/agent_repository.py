from datetime import datetime

from sqlalchemy import select, delete, update
from sqlalchemy.orm import selectinload

from openagents.repository.database import AgentEntity, async_session


# 查询全部 Agent，按 id 升序，预加载关联的模型提供商
async def list_agents() -> list[AgentEntity]:
    async with async_session() as session:
        result = await session.execute(select(AgentEntity).options(selectinload(AgentEntity.model_provider)).order_by(AgentEntity.id))
        return list(result.scalars().all())


# 按 id 查询 Agent，不存在返回 None，预加载关联的模型提供商
async def get_agent(agent_id: int) -> AgentEntity | None:
    async with async_session() as session:
        result = await session.execute(select(AgentEntity).options(selectinload(AgentEntity.model_provider)).where(AgentEntity.id == agent_id))
        return result.scalar_one_or_none()


# 新增 Agent，时间字段统一赋当前时间，返回自增 id
async def add_agent(name: str, description: str, prompt: str, model_provider_id: int, model: str, thinking: bool) -> int:
    async with async_session() as session:
        now = datetime.now()
        agent = AgentEntity(name=name, description=description, prompt=prompt, model_provider_id=model_provider_id, model=model, thinking=thinking, create_time=now, update_time=now)
        session.add(agent)
        await session.commit()
        await session.refresh(agent)
        return agent.id


# 按 id 更新 Agent，update_time 刷新为当前时间，id 不存在返回 False
async def update_agent(agent_id: int, name: str, description: str, prompt: str, model_provider_id: int, model: str, thinking: bool) -> bool:
    async with async_session() as session:
        result = await session.execute(
            update(AgentEntity)
            .where(AgentEntity.id == agent_id)
            .values(name=name, description=description, prompt=prompt, model_provider_id=model_provider_id, model=model, thinking=thinking, update_time=datetime.now())
        )
        await session.commit()
        return result.rowcount > 0


# 按 id 删除 Agent，不存在返回 False，关联对话的 agent_id 由数据库外键 ON DELETE SET NULL 自动置空
async def delete_agent(agent_id: int) -> bool:
    async with async_session() as session:
        result = await session.execute(delete(AgentEntity).where(AgentEntity.id == agent_id))
        await session.commit()
        return result.rowcount > 0
