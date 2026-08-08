import asyncio
import json
import logging

from openagents.repository import agent_repository, conversation_repository, task_repository
from openagents.service import work_service

# 每个 task 的执行循环后台任务，同一 task 同时只允许一个循环运行
_task_loop_dict: dict[int, asyncio.Task] = {}


# 启动任务执行循环，agent_id 为首个执行的 Agent，循环已在运行返回 False
def start_task(task_id: int, agent_id: int) -> bool:
    # 同一 task 不允许同时运行多个执行循环
    loop_task = _task_loop_dict.get(task_id)
    if loop_task is not None and not loop_task.done():
        return False
    _task_loop_dict[task_id] = asyncio.create_task(run_task(task_id, agent_id))
    return True


# 后台执行循环：先为首个 agent 创建阶段对话，之后每轮取任务最新对话，有 agent 则交给其执行，无 agent（用户审核对话）或对话已执行过则结束
# agent 通过 task handover 工具指派下一个执行者，创建一条 agent_id 为新 agent 的对话，下一轮循环自动接续
async def run_task(task_id: int, agent_id: int) -> None:
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
            task_content_list += [f"# Task\n{json.dumps({'title': task.title, 'content': task.content}, ensure_ascii=False)}"]
            # 团队成员为 agent_ids 候选池对应的 Agent
            team_agents = [agent for agent in await agent_repository.list_agents() if agent.id in task.agent_ids]
            task_content_list += [f"# Team\n{json.dumps([{'id': team_agent.id, 'name': team_agent.name, 'description': team_agent.description} for team_agent in team_agents], ensure_ascii=False)}"]
            task_content_list += ["# History"]
            for history_conversation in task.conversations:
                if not history_conversation.messages:
                    continue
                name = history_conversation.agent.name if history_conversation.agent else "User"
                content = history_conversation.messages[-1].content
                text = content if isinstance(content, str) else content[-1].get("text", "")
                task_content_list += [f"## {name}\n{json.dumps(text, ensure_ascii=False)}"]
            task_content = "\n\n".join(task_content_list)
            # 模型配置从当前对话的 Agent 读取，缺失时记录日志并结束循环
            agent = conversation.agent
            if agent is None or agent.model_provider is None or not agent.model:
                logging.error("Task execution failed: agent model provider or model not configured")
                return
            # 触发 work 执行并等待完成
            if not work_service.start_work(conversation.id, task_content, agent.model_provider.id, str(agent.model), bool(agent.thinking)):
                return
            await work_service.get_work_state(conversation.id).task
    except Exception as e:
        logging.error(f"Task execution failed: {e}", exc_info=True)
    finally:
        _task_loop_dict.pop(task_id, None)
