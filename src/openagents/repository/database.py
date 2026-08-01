import pathlib
from contextlib import asynccontextmanager
from datetime import datetime
from typing import List

from sqlalchemy import Index, ForeignKey, JSON, event
from sqlalchemy.engine import Engine
from sqlalchemy.ext.asyncio import create_async_engine, async_sessionmaker
from sqlalchemy.orm import Mapped, mapped_column, relationship
from sqlalchemy.orm import declarative_base

DATABASE_FILE = str(pathlib.Path.home() / ".openagents" / "database.db")
DATABASE_URL = f"sqlite+aiosqlite:///{DATABASE_FILE}"
async_engine = create_async_engine(DATABASE_URL)
async_session = async_sessionmaker(async_engine, expire_on_commit=False)
Base = declarative_base()


@asynccontextmanager
async def lifespan():
    async with async_engine.begin() as conn:
        await conn.run_sync(Base.metadata.create_all, checkfirst=True)
    yield
    await async_engine.dispose()


# 开启 SQLite 的外键约束支持
@event.listens_for(Engine, "connect")
def set_sqlite_pragma(dbapi_connection, connection_record):
    # 确保数据库层面的 ON DELETE CASCADE 能够正常工作
    cursor = dbapi_connection.cursor()
    cursor.execute("PRAGMA foreign_keys=ON")
    cursor.close()


# Agent 定义
class AgentEntity(Base):
    __tablename__ = "t_agent"

    id: Mapped[int] = mapped_column(primary_key=True, autoincrement=True)
    name: Mapped[str] = mapped_column(nullable=False)
    description: Mapped[str] = mapped_column(nullable=False)
    prompt: Mapped[str] = mapped_column(nullable=False)
    create_time: Mapped[datetime] = mapped_column(nullable=False)
    update_time: Mapped[datetime] = mapped_column(nullable=False)


# 任务
# agent_ids 为可供 Agent 选择下一个执行者的候选池（JSON 数组，如 [1,2,3]）
# 任务状态不存字段，由最后一条 conversation 推导：
# 1. agent_id 有值 -> 执行中
# 2. agent_id 为空（用户对话）且无 message -> 审核中
# 3. agent_id 为空（用户对话）且有 message -> 已完成
class TaskEntity(Base):
    __tablename__ = "t_task"

    id: Mapped[int] = mapped_column(primary_key=True, autoincrement=True)
    title: Mapped[str] = mapped_column(nullable=False)
    content: Mapped[str] = mapped_column(nullable=False)
    agent_ids: Mapped[list[int]] = mapped_column(JSON, nullable=False)
    create_time: Mapped[datetime] = mapped_column(nullable=False)
    update_time: Mapped[datetime] = mapped_column(nullable=False)

    conversations: Mapped[List["ConversationEntity"]] = relationship("ConversationEntity", back_populates="task", cascade="all, delete-orphan")


# 对话
# task_id 为空为独立对话
# task_id 有值为任务中的一次阶段流程，agent_id 有值为 Agent 执行阶段，为空为用户审核阶段
class ConversationEntity(Base):
    __tablename__ = "t_conversation"

    id: Mapped[int] = mapped_column(primary_key=True, autoincrement=True)
    task_id: Mapped[int | None] = mapped_column(ForeignKey("t_task.id", ondelete="CASCADE"), nullable=True)
    agent_id: Mapped[int | None] = mapped_column(ForeignKey("t_agent.id", ondelete="SET NULL"), nullable=True)
    title: Mapped[str] = mapped_column(nullable=False)
    work_dir: Mapped[str] = mapped_column(nullable=False)
    system_prompt: Mapped[str] = mapped_column(nullable=False)
    create_time: Mapped[datetime] = mapped_column(nullable=False)
    update_time: Mapped[datetime] = mapped_column(nullable=False)

    task: Mapped["TaskEntity"] = relationship("TaskEntity", back_populates="conversations")
    agent: Mapped["AgentEntity"] = relationship("AgentEntity")
    messages: Mapped[List["MessageEntity"]] = relationship("MessageEntity", back_populates="conversation", cascade="all, delete-orphan")


# 对话中 每一次消息
class MessageEntity(Base):
    __tablename__ = "t_message"

    id: Mapped[int] = mapped_column(primary_key=True, autoincrement=True)
    conversation_id: Mapped[int] = mapped_column(ForeignKey("t_conversation.id", ondelete="CASCADE"), nullable=False)
    role: Mapped[str] = mapped_column(nullable=False)
    content: Mapped[str | list | dict] = mapped_column(JSON, nullable=False)
    time: Mapped[datetime] = mapped_column(nullable=False)

    conversation: Mapped["ConversationEntity"] = relationship("ConversationEntity", back_populates="messages")


# 定义索引
Index("idx_message_conversation", MessageEntity.conversation_id)
Index("idx_conversation_task", ConversationEntity.task_id)
