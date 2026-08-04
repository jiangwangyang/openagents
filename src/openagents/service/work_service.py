import asyncio
import json
import logging
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
    try:
        # 查询历史消息
        conversation = await conversation_repository.get_conversation(conversation_id)
        if not conversation:
            # 没有对话 直接结束
            return
        messages = [{"id": msg.id, "role": msg.role, "content": msg.content, "time": msg.time} for msg in conversation.messages]
        # 无任务内容时为查询模式，不追加用户消息
        if task_content:
            messages += [{"role": "user", "content": task_content, "time": datetime.now()}]
        for msg in messages:
            # 字符串消息整体作为用户消息发布，跳过块迭代
            if isinstance(msg["content"], str):
                publish(conversation_id, "user", msg["content"])
            elif isinstance(msg["content"], list):
                for block in msg["content"]:
                    if block["type"] == "thinking":
                        publish(conversation_id, "thinking", block["thinking"])
                    elif block["type"] == "text":
                        publish(conversation_id, "text", block["text"])
                    elif block["type"] == "tool_use":
                        publish(conversation_id, "tool_use", json.dumps(block["input"], ensure_ascii=False), _id=block["id"], name=block["name"])
                    elif block["type"] == "tool_result":
                        publish(conversation_id, "tool_result", block["content"], _id=block["tool_use_id"], is_error=block["is_error"])

        # 没有任务 直接结束
        if not task_content:
            return

        # 模型
        model_provider = await model_provider_repository.get_current_model_provider()
        base_url = model_provider.base_url if model_provider else ""
        api_key = model_provider.api_key if model_provider else ""
        model = await model_provider_repository.get_current_model() or ""
        thinking = {"type": "enabled", "display": "summarized"} if await model_provider_repository.get_thinking() else {"type": "disabled"}
        work_dir = str(conversation.work_dir)
        system_prompt = str(conversation.system_prompt)
        tools = tool_service.list_tools()

        # 执行
        anthropic_client: AsyncAnthropic = AsyncAnthropic(base_url=base_url, api_key=api_key)
        while True:
            # 1. 发送 anthropic 请求
            response: AsyncStream[RawMessageStreamEvent] = await anthropic_client.messages.create(messages=messages, tools=tools, system=system_prompt, model=model, thinking=thinking, max_tokens=16000, stream=True)
            input_json = ""
            async for event in response:
                if event.type == "message_start":
                    msg = event.message
                elif event.type == "content_block_start":
                    msg.content += [event.content_block]
                    if event.content_block.type == "thinking":
                        publish(conversation_id, "thinking", "")
                    elif event.content_block.type == "text":
                        publish(conversation_id, "text", "")
                    elif event.content_block.type == "tool_use":
                        publish(conversation_id, "tool_use", "", _id=event.content_block.id, name=event.content_block.name)
                elif event.type == "content_block_delta":
                    if event.delta.type == "thinking_delta":
                        msg.content[-1].thinking += event.delta.thinking
                        publish(conversation_id, "delta", event.delta.thinking)
                    elif event.delta.type == "signature_delta":
                        msg.content[-1].signature += event.delta.signature
                    elif event.delta.type == "text_delta":
                        msg.content[-1].text += event.delta.text
                        publish(conversation_id, "delta", event.delta.text)
                    elif event.delta.type == "input_json_delta":
                        input_json += event.delta.partial_json
                        publish(conversation_id, "delta", event.delta.partial_json)
                elif event.type == "content_block_stop":
                    if msg.content[-1].type == "tool_use":
                        try:
                            msg.content[-1].input = json.loads(input_json)
                        except Exception as e:
                            msg.content[-1].input = {"error": str(e)}
                        input_json = ""
                elif event.type == "message_delta":
                    msg.container = event.delta.container
                    msg.stop_details = event.delta.stop_details
                    msg.stop_reason = event.delta.stop_reason
                    msg.stop_sequence = event.delta.stop_sequence
                    msg.usage.output_tokens = event.usage.output_tokens
                elif event.type == "message_stop":
                    pass
            msg_dict = msg.model_dump()
            msg_dict["time"] = datetime.now()
            messages += [msg_dict]
            logging.info(f"Response: {json.dumps(msg_dict, ensure_ascii=False, default=str)}")

            # 2. 判断结束
            tool_use_list = [block for block in msg.content if block.type == "tool_use"]
            if not tool_use_list:
                await conversation_repository.add_conversation_messages(conversation_id, [(msg_dict["role"], msg_dict["content"], msg_dict["time"]) for msg_dict in messages if "time" in msg_dict])
                return

            # 3. 工具调用
            msg_dict = {"role": "user", "content": []}
            for tool_use in tool_use_list:
                tool_content, is_error = await tool_service.execute_tool(tool_use.name, tool_use.input, work_dir)
                msg_dict["content"] += [{"type": "tool_result", "tool_use_id": tool_use.id, "content": tool_content, "is_error": is_error}]
                publish(conversation_id, "tool_result", tool_content, _id=tool_use.id, is_error=is_error)
            msg_dict["time"] = datetime.now()
            messages += [msg_dict]
            logging.info(f"Tool: {json.dumps(msg_dict, ensure_ascii=False, default=str)}")
    except Exception as e:
        publish(conversation_id, "error", f"Work execution failed: {e}")
        logging.error(f"Work execution failed: {e}", exc_info=True)
    finally:
        finish(conversation_id)
        # 查询任务状态可以保留，执行任务存储的chunk太碎需要清理
        if task_content:
            _work_state_dict.pop(conversation_id)
