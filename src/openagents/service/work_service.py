import asyncio
import json
from datetime import datetime

from anthropic import AsyncAnthropic, AsyncStream
from anthropic.types.raw_message_stream_event import RawMessageStreamEvent
from pydantic import BaseModel, ConfigDict

from openagents.repository import conversation_repository, model_provider_repository
from openagents.service import tool_service


# 每个 conversation 的流式数据状态，chunks 只追加，各请求通过自己的 index 游标读取
class WorkState(BaseModel):
    model_config = ConfigDict(arbitrary_types_allowed=True)

    chunks: list[dict]
    done: bool
    event: asyncio.Event
    task: asyncio.Task


_work_state_dict: dict[int, WorkState] = {}


def start_work(conversation_id: int, task_content: str | None = None) -> bool:
    # 同一 conversation 不允许同时运行多个 work 任务
    work_state = _work_state_dict.get(conversation_id)
    if work_state is not None and not work_state.done:
        return False
    _work_state_dict[conversation_id] = WorkState(chunks=[], done=False, event=asyncio.Event(), task=asyncio.create_task(work(conversation_id, task_content)))
    return True


def get_work_state(conversation_id: int) -> WorkState | None:
    return _work_state_dict.get(conversation_id)


def publish(conversation_id: int, _type: str, text: str, _id: str | None = None, name: str | None = None, is_error: bool | None = None) -> None:
    # 构造SSE数据
    chunk = {"type": _type, "text": text, "id": _id, "name": name, "is_error": is_error}
    chunk = {k: v for k, v in chunk.items() if v is not None}
    # 追加到列表
    work_state = _work_state_dict[conversation_id]
    work_state.chunks.append(chunk)
    # 唤醒当前所有等待者后更换新的 Event，后续等待者在新 Event 上等待
    work_state.event.set()
    work_state.event = asyncio.Event()


def finish(conversation_id: int) -> None:
    work_state = _work_state_dict[conversation_id]
    work_state.done = True
    work_state.event.set()


# 后台执行
async def work(conversation_id: int, task_content: str | None) -> None:
    # 查询历史消息
    conversation = await conversation_repository.get_conversation(conversation_id)
    if not conversation:
        # 没有对话 直接结束
        finish(conversation_id)
        return
    messages = [{"id": msg.id, "role": msg.role, "content": msg.content} for msg in conversation.messages]
    # 无任务内容时为查询模式，不追加用户消息
    if task_content:
        messages += [{"role": "user", "content": task_content}]
    for msg in messages:
        # 字符串消息整体作为用户消息发布，跳过块迭代
        if isinstance(msg["content"], str):
            publish(conversation_id, "user", msg["content"])
            continue
        for block in msg["content"]:
            if block["type"] == "thinking":
                publish(conversation_id, "thinking", block["thinking"])
            if block["type"] == "text":
                publish(conversation_id, "text", block["text"])
            if block["type"] == "tool_use":
                publish(conversation_id, "tool_use", json.dumps(block["input"], ensure_ascii=False), _id=block["id"], name=block["name"])
            if block["type"] == "tool_result":
                publish(conversation_id, "tool_result", block["tool_content"], _id=block["tool_use_id"], is_error=block["is_error"])

    # 没有任务 直接结束
    if not task_content:
        finish(conversation_id)
        return

    # 模型
    model_provider = await model_provider_repository.get_current_model_provider()
    base_url = model_provider.base_url if model_provider else ""
    api_key = model_provider.api_key if model_provider else ""
    model = await model_provider_repository.get_current_model() or ""
    work_dir = str(conversation.work_dir)
    system_prompt = str(conversation.system_prompt)
    tools = tool_service.list_tools()

    # 执行
    anthropic_client: AsyncAnthropic = AsyncAnthropic(base_url=base_url, api_key=api_key)
    while True:
        # 1. 发送 anthropic 请求
        response: AsyncStream[RawMessageStreamEvent] = await anthropic_client.messages.create(messages=messages, tools=tools, system=system_prompt, model=model, max_tokens=16000, stream=True)
        model_block_list = []
        async for event in response:
            if event.type == "content_block_start":
                if event.content_block.type == "thinking":
                    model_block_list += [{"type": "thinking", "thinking": "", "signature": ""}]
                    publish(conversation_id, "thinking", "")
                elif event.content_block.type == "text":
                    model_block_list += [{"type": "text", "text": ""}]
                    publish(conversation_id, "text", "")
                elif event.content_block.type == "tool_use":
                    model_block_list += [{"type": "tool_use", "id": event.content_block.id, "name": event.content_block.name, "input": ""}]
                    publish(conversation_id, "tool_use", "", _id=event.content_block.id, name=event.content_block.name)
            elif event.type == "content_block_delta":
                if event.delta.type == "thinking_delta":
                    model_block_list[-1]["thinking"] += event.delta.thinking
                    publish(conversation_id, "delta", event.delta.thinking)
                elif event.delta.type == "signature_delta":
                    model_block_list[-1]["signature"] += event.delta.signature
                elif event.delta.type == "text_delta":
                    model_block_list[-1]["text"] += event.delta.text
                    publish(conversation_id, "delta", event.delta.text)
                elif event.delta.type == "input_json_delta":
                    model_block_list[-1]["input"] += event.delta.partial_json
                    publish(conversation_id, "delta", event.delta.partial_json)
        for block in [block for block in model_block_list if block["type"] == "tool_use"]:
            block["input"] = json.loads(block["input"]) if block["input"] else {}
        messages += [{"role": "assistant", "content": model_block_list, "time": datetime.now()}]

        # 2. 判断结束
        if not [block for block in model_block_list if block["type"] == "tool_use"]:
            await conversation_repository.add_conversation_messages(conversation_id, [(msg["role"], msg["content"], msg["time"]) for msg in messages if not "id" in msg])
            finish(conversation_id)
            return

        # 3. 工具调用
        tool_result_list = []
        for tool_use in [block for block in model_block_list if block["type"] == "tool_use"]:
            tool_content, is_error = await tool_service.execute_tool(tool_use["name"], tool_use["input"], work_dir)
            tool_result_list += [{"type": "tool_result", "tool_use_id": tool_use["id"], "content": tool_content, "is_error": is_error}]
            publish(conversation_id, "tool_result", tool_content, _id=tool_use["id"], is_error=is_error)
        messages += [{"role": "user", "content": tool_result_list, "time": datetime.now()}]
