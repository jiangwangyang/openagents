from datetime import datetime

from sqlalchemy import select, delete, update
from sqlalchemy.orm import selectinload

from openagents.repository.database import ConversationEntity, MessageEntity, async_session


# 查询全部对话（包含独立对话与任务中的阶段对话），按更新时间倒序
async def get_conversations() -> list[ConversationEntity]:
    async with async_session() as session:
        result = await session.execute(select(ConversationEntity).order_by(ConversationEntity.update_time.desc()))
        return list(result.scalars().all())


# 按 id 查询对话，预加载消息并
async def get_conversation(conversation_id: int) -> ConversationEntity | None:
    async with async_session() as session:
        result = await session.execute(
            select(ConversationEntity)
            .where(ConversationEntity.id == conversation_id)
            .options(selectinload(ConversationEntity.messages))
        )
        conversation = result.scalar_one_or_none()
        if conversation is None:
            return None
        conversation.messages.sort(key=lambda message: message.id)
        return conversation


# 新建对话，时间字段未设置时统一赋当前时间，task_id 为空为独立对话，agent_id 有值为 Agent 执行阶段、为空为用户审核阶段
async def add_conversation(conversation: ConversationEntity) -> ConversationEntity:
    async with async_session() as session:
        session.add(conversation)
        await session.commit()
        await session.refresh(conversation)
        return conversation


# 批量追加对话消息，校验消息归属的对话，time 未设置时统一赋当前时间，并刷新对话的更新时间
async def add_conversation_messages(conversation_id: int, messages: list[MessageEntity]) -> None:
    async with async_session() as session:
        now = datetime.now()
        for message in messages:
            if message.conversation_id != conversation_id:
                raise ValueError(f"消息 conversation_id {message.conversation_id} 与目标对话 {conversation_id} 不一致")
        session.add_all(messages)
        # 原子更新对话的更新时间，避免先查后改的并发问题
        await session.execute(update(ConversationEntity).where(ConversationEntity.id == conversation_id).values(update_time=now))
        await session.commit()


# 删除对话，消息由数据库外键 ON DELETE CASCADE 级联删除
async def delete_conversation(conversation_id: int) -> bool:
    async with async_session() as session:
        result = await session.execute(delete(ConversationEntity).where(ConversationEntity.id == conversation_id))
        await session.commit()
        return result.rowcount > 0
