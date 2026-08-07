from datetime import datetime

from sqlalchemy import select, delete, update

from openagents.repository.database import AgentEntity, ModelProviderEntity, async_session


# 查询全部模型提供商，按 id 升序
async def list_model_providers() -> list[ModelProviderEntity]:
    async with async_session() as session:
        result = await session.execute(select(ModelProviderEntity).order_by(ModelProviderEntity.id))
        return list(result.scalars().all())


# 按 id 查询模型提供商，不存在返回 None
async def get_model_provider(provider_id: int) -> ModelProviderEntity | None:
    async with async_session() as session:
        result = await session.execute(select(ModelProviderEntity).where(ModelProviderEntity.id == provider_id))
        return result.scalar_one_or_none()


# 新增模型提供商，名称已存在返回 None，成功返回自增 id
async def add_model_provider(name: str, type: str, base_url: str, api_key: str) -> int | None:
    async with async_session() as session:
        result = await session.execute(select(ModelProviderEntity).where(ModelProviderEntity.name == name))
        if result.scalar_one_or_none() is not None:
            return None
        now = datetime.now()
        provider = ModelProviderEntity(name=name, type=type, base_url=base_url, api_key=api_key, create_time=now, update_time=now)
        session.add(provider)
        await session.commit()
        await session.refresh(provider)
        return provider.id


# 按 id 更新模型提供商，名称被其它记录占用或 id 不存在返回 False
async def update_model_provider(provider_id: int, name: str, type: str, base_url: str, api_key: str) -> bool:
    async with async_session() as session:
        result = await session.execute(select(ModelProviderEntity).where(ModelProviderEntity.name == name, ModelProviderEntity.id != provider_id))
        if result.scalar_one_or_none() is not None:
            return False
        result = await session.execute(
            update(ModelProviderEntity)
            .where(ModelProviderEntity.id == provider_id)
            .values(name=name, type=type, base_url=base_url, api_key=api_key, update_time=datetime.now())
        )
        await session.commit()
        return result.rowcount > 0


# 按 id 删除模型提供商，不存在或被 Agent 引用返回 False
async def delete_model_provider(provider_id: int) -> bool:
    async with async_session() as session:
        referenced = await session.execute(select(AgentEntity.id).where(AgentEntity.model_provider_id == provider_id).limit(1))
        if referenced.scalar_one_or_none() is not None:
            return False
        result = await session.execute(delete(ModelProviderEntity).where(ModelProviderEntity.id == provider_id))
        await session.commit()
        return result.rowcount > 0
