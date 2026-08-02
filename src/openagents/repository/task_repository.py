from datetime import datetime

from sqlalchemy import select, delete
from sqlalchemy.orm import selectinload

from openagents.repository.database import ConversationEntity, TaskEntity, async_session


# 查询全部任务，按 id 升序
async def list_tasks() -> list[TaskEntity]:
    async with async_session() as session:
        result = await session.execute(select(TaskEntity).order_by(TaskEntity.id))
        return list(result.scalars().all())


# 按 id 查询任务，预加载阶段对话（含每条对话的消息与执行 Agent），不存在返回 None
async def get_task(task_id: int) -> TaskEntity | None:
    async with async_session() as session:
        result = await session.execute(
            select(TaskEntity)
            .where(TaskEntity.id == task_id)
            .options(
                selectinload(TaskEntity.conversations).selectinload(ConversationEntity.messages),
                selectinload(TaskEntity.conversations).selectinload(ConversationEntity.agent),
            )
        )
        task = result.scalar_one_or_none()
        if task is None:
            return None
        task.conversations.sort(key=lambda x: x.id)
        for conversation in task.conversations:
            conversation.messages.sort(key=lambda message: message.id)
        return task


# 新增任务，时间字段统一赋当前时间，返回自增 id
async def add_task(title: str, content: str, agent_ids: list[int], work_dir: str) -> int:
    async with async_session() as session:
        now = datetime.now()
        task = TaskEntity(title=title, content=content, agent_ids=agent_ids, work_dir=work_dir, create_time=now, update_time=now)
        session.add(task)
        await session.commit()
        await session.refresh(task)
        return task.id


# 按 id 删除任务，不存在返回 False，关联对话由数据库外键 ON DELETE CASCADE 级联删除
async def delete_task(task_id: int) -> bool:
    async with async_session() as session:
        result = await session.execute(delete(TaskEntity).where(TaskEntity.id == task_id))
        await session.commit()
        return result.rowcount > 0
