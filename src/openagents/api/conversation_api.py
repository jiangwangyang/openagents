from datetime import datetime

from fastapi import APIRouter, HTTPException, Body

from openagents.repository import conversation_repository

router = APIRouter()


# 对话列表接口，按更新时间倒序返回全部对话（仅基本字段，不含消息内容与 system_prompt）
@router.get("/conversation/list")
async def get_conversations() -> list[dict]:
    conversations = await conversation_repository.get_conversations()
    return [
        {
            "id": conversation.id,
            "task_id": conversation.task_id,
            "agent_id": conversation.agent_id,
            "title": conversation.title,
            "work_dir": conversation.work_dir,
            "create_time": conversation.create_time,
            "update_time": conversation.update_time,
        }
        for conversation in conversations if not conversation.task_id
    ]


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
    await conversation_repository.add_conversation_messages(conversation_id, [("user", content, datetime.now())])
