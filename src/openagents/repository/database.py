import os
import pathlib
from collections.abc import AsyncIterator
from contextlib import asynccontextmanager
from datetime import datetime

from sqlalchemy import JSON, Column, Index, event, select
from sqlalchemy.engine import Engine
from sqlalchemy.ext.asyncio import create_async_engine, async_sessionmaker
from sqlmodel import Field, Relationship, SQLModel

DATABASE_FILE = str(pathlib.Path.home() / ".openagents" / "database.db")
DATABASE_URL = f"sqlite+aiosqlite:///{DATABASE_FILE}"
async_engine = create_async_engine(DATABASE_URL)
async_session = async_sessionmaker(async_engine, expire_on_commit=False)

# 内置模型提供商定义：(名称, base_url, API Key 环境变量)，type 固定为 anthropic
PROVIDER_DEFS = [
    ("deepseek", "anthropic", "https://api.deepseek.com/anthropic", "DEEPSEEK_API_KEY"),
    ("bigmodel", "anthropic", "https://open.bigmodel.cn/api/anthropic", "BIGMODEL_API_KEY"),
    ("moonshot", "anthropic", "https://api.moonshot.cn/anthropic", "MOONSHOT_API_KEY"),
    ("minimaxi", "anthropic", "https://api.minimaxi.com/anthropic", "MINIMAXI_API_KEY"),
]


@asynccontextmanager
async def lifespan() -> AsyncIterator[None]:
    # 创建数据库表
    async with async_engine.begin() as conn:
        await conn.run_sync(SQLModel.metadata.create_all, checkfirst=True)
    # 根据环境变量自动补充缺失的内置模型提供商，type 固定为 anthropic
    async with async_session() as session:
        for name, _type, base_url, env_key in PROVIDER_DEFS:
            api_key = os.getenv(env_key, "")
            if not api_key:
                continue
            result = await session.execute(select(ModelProviderEntity).where(ModelProviderEntity.name == name))
            if result.scalar_one_or_none() is None:
                now = datetime.now()
                session.add(ModelProviderEntity(name=name, type=_type, base_url=base_url, api_key=api_key, create_time=now, update_time=now))
        await session.commit()
    yield
    await async_engine.dispose()


# 开启 SQLite 的外键约束支持
@event.listens_for(Engine, "connect")
def set_sqlite_pragma(dbapi_connection: object, connection_record: object) -> None:
    # 确保数据库层面的 ON DELETE CASCADE 能够正常工作
    cursor = dbapi_connection.cursor()
    cursor.execute("PRAGMA foreign_keys=ON")
    cursor.close()


# 模型提供商，type 为协议类型（如 anthropic）
class ModelProviderEntity(SQLModel, table=True):
    __tablename__ = "t_model_provider"

    id: int | None = Field(default=None, primary_key=True)
    name: str = Field(unique=True)
    type: str
    base_url: str
    api_key: str
    create_time: datetime
    update_time: datetime


# Agent 定义，model_provider/model/thinking 为该 Agent 执行时使用的模型配置，model_provider_id 必填且 provider 被引用时不允许删除
class AgentEntity(SQLModel, table=True):
    __tablename__ = "t_agent"

    id: int | None = Field(default=None, primary_key=True)
    name: str
    description: str
    prompt: str
    model_provider_id: int = Field(foreign_key="t_model_provider.id", ondelete="RESTRICT")
    model: str
    thinking: bool
    create_time: datetime
    update_time: datetime

    model_provider: ModelProviderEntity = Relationship()


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

    task: TaskEntity | None = Relationship(back_populates="conversations")
    agent: AgentEntity | None = Relationship()
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


# MCP 服务，type 为协议类型（streamable_http / sse / stdio），按类型使用 url+headers 或 command+args
class McpServerEntity(SQLModel, table=True):
    __tablename__ = "t_mcp_server"

    id: int | None = Field(default=None, primary_key=True)
    name: str = Field(unique=True)
    description: str
    type: str
    url: str | None = None
    headers: dict[str, str] | None = Field(default=None, sa_column=Column(JSON, nullable=True))
    command: str | None = None
    args: list[str] | None = Field(default=None, sa_column=Column(JSON, nullable=True))
    create_time: datetime
    update_time: datetime


# 定义索引
Index("idx_message_conversation", MessageEntity.__table__.c.conversation_id)
Index("idx_conversation_task", ConversationEntity.__table__.c.task_id)
