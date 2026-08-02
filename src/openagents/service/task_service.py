import asyncio
import json
import logging
from contextvars import ContextVar

from openagents.repository import agent_repository, conversation_repository, task_repository
from openagents.service import work_service

# 每个 task 的执行循环后台任务，同一 task 同时只允许一个循环运行
_task_loop_dict: dict[int, asyncio.Task] = {}

# 当前 async 上下文中的 task_id，由任务执行循环注入，随 create_task 的上下文复制传播到工具执行等下游环节
_task_id_context: ContextVar[int | None] = ContextVar("task_id", default=None)


# 获取当前 async 上下文中的 task_id，非任务上下文返回 None
def get_task_id() -> int | None:
    return _task_id_context.get()


# 启动任务执行循环，agent_id 为首个执行的 Agent，循环已在运行返回 False
def start_task(task_id: int, agent_id: int) -> bool:
    # 同一 task 不允许同时运行多个执行循环
    loop_task = _task_loop_dict.get(task_id)
    if loop_task is not None and not loop_task.done():
        return False
    _task_loop_dict[task_id] = asyncio.create_task(run_task(task_id, agent_id))
    return True


# 后台执行循环：先为首个 agent 创建阶段对话，之后每轮取任务最新对话，有 agent 则交给其执行，无 agent（用户审核对话）或对话已执行过则结束
# agent 指派下一个 agent 即创建一条 agent_id 为新 agent 的对话（dispatch 工具后续实现），下一轮循环自动接续
async def run_task(task_id: int, agent_id: int) -> None:
    # 向当前 async 上下文注入 task_id，下游（work → 工具执行）通过 get_task_id 读取
    token = _task_id_context.set(task_id)
    try:
        # 进入循环前，为第一个执行的 agent 创建阶段对话（task_id 与 agent_id 均不为空）
        task = await task_repository.get_task(task_id)
        agent = await agent_repository.get_agent(agent_id)
        if task is None or agent is None:
            return
        await conversation_repository.add_conversation(f"{task.title}-{agent.name}", str(task.work_dir), str(agent.prompt), task_id, agent_id)
        while True:
            # 每轮重新查询任务，取最新一条对话
            task = await task_repository.get_task(task_id)
            if task is None or not task.conversations:
                return
            conversation = task.conversations[-1]
            # 最新对话无 agent（用户审核阶段），结束循环
            if conversation.agent_id is None:
                return
            # 拼接各阶段对话的最后一条消息：agent 对话以 agent 名为标题，用户对话以“用户”为标题
            task_content_list = []
            task_content_list += [f"# Task\n{json.dumps({"title": task.title, "content": task.content}, ensure_ascii=False)}"]
            task_content_list += [f"# Team\n{json.dumps([{"id": team_agent.id, "name": team_agent.name, "description": team_agent.description} for team_agent in task.agents], ensure_ascii=False)}"]
            task_content_list += ["# Tool\n当需要将当前任务的控制权移交予其他智能体或人类时，请先执行上述内置移交命令，随后采用结构化格式（包括：当前状态、已完成事项、阻塞问题及待办计划）对任务进展进行总结。"]
            task_content_list += ["# History"]
            for history_conversation in task.conversations:
                if not history_conversation.messages:
                    continue
                name = history_conversation.agent.name if history_conversation.agent else "用户"
                content = history_conversation.messages[-1].content if history_conversation.messages else ""
                text = content if isinstance(content, str) else content[-1].get("text", "")
                task_content_list += [f"## {name}\n{json.dumps(text, ensure_ascii=False)}"]
            task_content = "\n\n".join(task_content_list)
            # 触发 work 执行并等待完成
            if not work_service.start_work(conversation.id, task_content):
                return
            await work_service.get_work_state(conversation.id).task
    except Exception as e:
        logging.error(f"任务执行异常: {str(e)}", exc_info=True)
    finally:
        _task_id_context.reset(token)
        _task_loop_dict.pop(task_id, None)
