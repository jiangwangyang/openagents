from datetime import datetime

from fastapi import APIRouter, HTTPException, Body

from openagents.repository import conversation_repository
from openagents.repository.database import ConversationEntity

router = APIRouter()


# 对话列表接口，按更新时间倒序返回独立对话（不含任务中的阶段对话）
@router.get("/conversation/list")
async def get_conversations() -> list[ConversationEntity]:
    conversations = await conversation_repository.get_conversations()
    return [conversation for conversation in conversations if conversation.task_id is None]


# 删除对话接口，消息由数据库外键 ON DELETE CASCADE 级联删除，对话不存在时返回 404
@router.delete("/conversation/{conversation_id}")
async def delete_conversation(conversation_id: int) -> None:
    if not await conversation_repository.delete_conversation(conversation_id):
        raise HTTPException(status_code=404, detail="Conversation not found")


# 追加用户消息接口，向指定对话追加一条 role 为 user 的消息并刷新对话更新时间，对话不存在返回 404
@router.post("/conversation/{conversation_id}/message")
async def add_conversation_message(conversation_id: int, content: str = Body(..., embed=True)) -> None:
    if await conversation_repository.get_conversation(conversation_id) is None:
        raise HTTPException(status_code=404, detail="Conversation not found")
    await conversation_repository.add_conversation_messages(conversation_id, [("user", content, "", 0, 0, 0, datetime.now())])
