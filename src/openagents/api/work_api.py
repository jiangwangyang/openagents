import json
from collections.abc import AsyncGenerator

import anyio
from fastapi import APIRouter, HTTPException, Body
from fastapi.responses import StreamingResponse

from openagents.repository import agent_repository, conversation_repository
from openagents.service import work_service

router = APIRouter()


@router.post("/work/start")
async def create_work(task_content: str = Body(..., embed=True), work_dir: str = Body(..., embed=True), model_provider_id: int | None = Body(None, embed=True), model: str | None = Body(None, embed=True), thinking: bool | None = Body(None, embed=True), agent_id: int | None = Body(None, embed=True)) -> int:
    # 指定 agent 时使用其 prompt 作为 system_prompt（与任务流水线的 Agent 阶段对话一致），否则读取 AGENTS.md，按优先级取第一个存在的文件
    system_prompt = ""
    if agent_id is not None:
        agent = await agent_repository.get_agent(agent_id)
        if agent is None:
            raise HTTPException(status_code=404, detail="Agent not found")
        system_prompt = str(agent.prompt)
        # 模型配置必须全部为 None 或全部与 agent 配置一致，否则拒绝请求
        user_configs = [model_provider_id, model, thinking]
        agent_configs = [agent.model_provider_id, agent.model, agent.thinking]
        if any(config is not None for config in user_configs) and user_configs != agent_configs:
            raise HTTPException(status_code=400, detail="Model config must be all None or all consistent with agent config")
        # 使用 agent 的模型配置，忽略用户传入的参数
        model_provider_id, model, thinking = agent.model_provider_id, agent.model, agent.thinking
    else:
        # 未指定 agent 时模型配置必填
        if model_provider_id is None or model is None or thinking is None:
            raise HTTPException(status_code=400, detail="Model config is required when agent_id is not provided")
        for agents_file in [anyio.Path(work_dir) / "AGENTS.md", await anyio.Path.home() / ".openagents" / "AGENTS.md", await anyio.Path.home() / ".agents" / "AGENTS.md"]:
            if await agents_file.exists() and await agents_file.is_file():
                system_prompt = await agents_file.read_text(encoding="utf-8")
                break
    # 先创建对话，再根据对话ID开始任务
    conversation_id = await conversation_repository.add_conversation(task_content, work_dir, system_prompt, agent_id=agent_id)
    if not work_service.start_work(conversation_id, task_content, model_provider_id, model, thinking):
        # 启动失败时清理刚创建的对话，避免产生孤儿数据
        await conversation_repository.delete_conversation(conversation_id)
        raise HTTPException(status_code=409, detail="Work already running")
    return conversation_id


@router.post("/work/{conversation_id}/start")
async def start_work(conversation_id: int, task_content: str = Body(..., embed=True), model_provider_id: int = Body(..., embed=True), model: str = Body(..., embed=True), thinking: bool = Body(..., embed=True)) -> None:
    if not work_service.start_work(conversation_id, task_content, model_provider_id, model, thinking):
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
        work_service.start_query(conversation_id)
    work_state = work_service.get_work_state(conversation_id)

    # 返回流式数据
    async def _generate() -> AsyncGenerator[str]:
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
