from fastapi import APIRouter, HTTPException, Body

from openagents.repository import agent_repository, task_repository
from openagents.repository.database import TaskEntity
from openagents.service import task_service

router = APIRouter()


# 任务列表接口，按 id 升序返回全部任务（仅基本字段，不含阶段对话）
@router.get("/task/list")
async def list_tasks() -> list[TaskEntity]:
    return await task_repository.list_tasks()


# 任务详情接口，包含各阶段对话（对话按 id 升序，每条对话含全部按 id 升序的消息），任务不存在返回 404
@router.get("/task/{task_id}")
async def get_task(task_id: int) -> dict:
    task = await task_repository.get_task(task_id)
    if task is None:
        raise HTTPException(status_code=404, detail="Task not found")
    return {
        "id": task.id,
        "title": task.title,
        "content": task.content,
        "agent_ids": task.agent_ids,
        "work_dir": task.work_dir,
        "create_time": task.create_time,
        "update_time": task.update_time,
        "conversations": [
            {
                "id": conversation.id,
                "task_id": conversation.task_id,
                "agent_id": conversation.agent_id,
                "title": conversation.title,
                "work_dir": conversation.work_dir,
                "create_time": conversation.create_time,
                "update_time": conversation.update_time,
                "messages": [{
                    "id": message.id,
                    "role": message.role,
                    "content": message.content,
                    "time": message.time
                } for message in conversation.messages],
            }
            for conversation in task.conversations
        ],
    }


# 新增任务接口，agent_ids 为可供 Agent 选择下一个执行者的候选池，work_dir 为任务阶段对话的工作目录，返回自增 id
@router.post("/task")
async def add_task(title: str = Body(..., embed=True), content: str = Body(..., embed=True), agent_ids: list[int] = Body(..., embed=True), work_dir: str = Body(..., embed=True)) -> int:
    return await task_repository.add_task(title, content, agent_ids, work_dir)


# 删除任务接口，关联对话由数据库外键 ON DELETE CASCADE 级联删除，任务不存在返回 404
@router.delete("/task/{task_id}")
async def delete_task(task_id: int) -> None:
    if not await task_repository.delete_task(task_id):
        raise HTTPException(status_code=404, detail="Task not found")


# 启动任务执行循环接口，agent_id 为首个执行的 Agent，阶段对话的工作目录取任务的 work_dir
# 任务/agent 不存在返回 404，执行循环已在运行返回 409
@router.post("/task/{task_id}/start")
async def start_task(task_id: int, agent_id: int = Body(..., embed=True)) -> None:
    if await task_repository.get_task(task_id) is None:
        raise HTTPException(status_code=404, detail="Task not found")
    if await agent_repository.get_agent(agent_id) is None:
        raise HTTPException(status_code=404, detail="Agent not found")
    if not task_service.start_task(task_id, agent_id):
        raise HTTPException(status_code=409, detail="Task already running")
