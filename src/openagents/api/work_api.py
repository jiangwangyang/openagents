import json

from fastapi import APIRouter, HTTPException, Body
from fastapi.responses import StreamingResponse

from openagents.repository import conversation_repository
from openagents.service import work_service

router = APIRouter()


@router.post("/work/start")
async def start_work(task_content: str = Body(..., embed=True), work_dir: str = Body(..., embed=True)) -> int:
    conversation_id = await conversation_repository.add_conversation(task_content[:30], work_dir)
    if not work_service.start_work(conversation_id, task_content):
        raise HTTPException(status_code=409, detail="Work already running")
    return conversation_id


@router.post("/work/{conversation_id}/start")
async def start_work(conversation_id: int, task_content: str = Body(..., embed=True)) -> None:
    if not work_service.start_work(conversation_id, task_content):
        raise HTTPException(status_code=409, detail="Work already running")


@router.get("/work/{conversation_id}/stream")
async def stream_work(conversation_id: int) -> StreamingResponse:
    conversation = await conversation_repository.get_conversation(conversation_id)
    if conversation is None:
        raise HTTPException(status_code=404, detail="Work not found")

    # 查询 work_state
    work_state = work_service.get_work_state(conversation_id)
    if work_state is None:
        # 如果没有 work 启动一个查询 work 不执行业务
        work_service.start_work(conversation_id)
    work_state = work_service.get_work_state(conversation_id)

    # 返回流式数据
    async def _generate():
        # 每个请求持有独立的 index 游标，先回放历史数据，再实时跟随新数据
        index = 0
        while True:
            while index < len(work_state.chunks):
                chunk = work_state.chunks[index]
                yield f"data: {json.dumps(chunk, ensure_ascii=False)}\n\n"
                index += 1
            if work_state.done:
                break
            # 读取 len 与 await 之间没有异步切换点，不存在丢信号的竞态
            await work_state.event.wait()

    return StreamingResponse(_generate(), media_type="text/event-stream")
