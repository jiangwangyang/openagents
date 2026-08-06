import pathlib
from collections.abc import AsyncIterator
from contextlib import asynccontextmanager
from datetime import datetime

from sqlalchemy import JSON, Column, Index, event
from sqlalchemy.engine import Engine
from sqlalchemy.ext.asyncio import create_async_engine, async_sessionmaker
from sqlmodel import Field, Relationship, SQLModel

DATABASE_FILE = str(pathlib.Path.home() / ".openagents" / "database.db")
DATABASE_URL = f"sqlite+aiosqlite:///{DATABASE_FILE}"
async_engine = create_async_engine(DATABASE_URL)
async_session = async_sessionmaker(async_engine, expire_on_commit=False)


@asynccontextmanager
async def lifespan() -> AsyncIterator[None]:
    async with async_engine.begin() as conn:
        await conn.run_sync(SQLModel.metadata.create_all, checkfirst=True)
    yield
    await async_engine.dispose()


# 开启 SQLite 的外键约束支持
@event.listens_for(Engine, "connect")
def set_sqlite_pragma(dbapi_connection: object, connection_record: object) -> None:
    # 确保数据库层面的 ON DELETE CASCADE 能够正常工作
    cursor = dbapi_connection.cursor()
    cursor.execute("PRAGMA foreign_keys=ON")
    cursor.close()


# Agent 定义
class AgentEntity(SQLModel, table=True):
    __tablename__ = "t_agent"

    id: int | None = Field(default=None, primary_key=True)
    name: str
    description: str
    prompt: str
    create_time: datetime
    update_time: datetime


# 任务
# agent_ids 为可供 Agent 选择下一个执行者的候选池（JSON 数组，如 [1,2,3]）
# 任务状态不存字段，由最后一条 conversation 推导：
# 1. agent_id 有值 -> 执行中
# 2. agent_id 为空（用户对话）且无 message -> 审核中
# 3. agent_id 为空（用户对话）且有 message -> 已完成
class TaskEntity(SQLModel, table=True):
    __tablename__ = "t_task"

    id: int | None = Field(default=None, primary_key=True)
    title: str
    content: str
    agent_ids: list[int] = Field(sa_column=Column(JSON, nullable=False))
    work_dir: str
    create_time: datetime
    update_time: datetime

    conversations: list["ConversationEntity"] = Relationship(back_populates="task", sa_relationship_kwargs={"cascade": "all, delete-orphan"})


# 对话
# task_id 为空为独立对话
# task_id 有值为任务中的一次阶段流程，agent_id 有值为 Agent 执行阶段，为空为用户审核阶段
class ConversationEntity(SQLModel, table=True):
    __tablename__ = "t_conversation"

    id: int | None = Field(default=None, primary_key=True)
    task_id: int | None = Field(default=None, foreign_key="t_task.id", ondelete="CASCADE")
    agent_id: int | None = Field(default=None, foreign_key="t_agent.id", ondelete="SET NULL")
    title: str
    work_dir: str
    system_prompt: str
    create_time: datetime
    update_time: datetime

    agent: AgentEntity | None = Relationship()
    task: TaskEntity | None = Relationship(back_populates="conversations")
    messages: list["MessageEntity"] = Relationship(back_populates="conversation", sa_relationship_kwargs={"cascade": "all, delete-orphan"})


# 对话中 每一次消息
class MessageEntity(SQLModel, table=True):
    __tablename__ = "t_message"

    id: int | None = Field(default=None, primary_key=True)
    conversation_id: int = Field(foreign_key="t_conversation.id", ondelete="CASCADE")
    role: str
    content: str | list | dict = Field(sa_column=Column(JSON, nullable=False))
    stop_reason: str
    cache_read_input_tokens: int
    input_tokens: int
    output_tokens: int
    time: datetime

    conversation: ConversationEntity = Relationship(back_populates="messages")


# 定义索引
Index("idx_message_conversation", MessageEntity.__table__.c.conversation_id)
Index("idx_conversation_task", ConversationEntity.__table__.c.task_id)
