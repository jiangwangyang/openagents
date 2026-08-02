from openagents.repository import task_repository, conversation_repository, agent_repository
from openagents.service import task_service


async def execute(cmd_and_args: list[str], work_dir: str) -> tuple[str, bool]:
    # 任务移交：从当前 async 上下文读取 task_id，为该任务创建新对话，run_task 下一轮循环自动接续
    # 延迟导入避免循环依赖（task_service → work_service → tool_service）
    task_id = task_service.get_task_id()
    if task_id is None:
        return "当前不在任务上下文中，无法执行移交", True
    if len(cmd_and_args) < 3 or cmd_and_args[1] != "handover":
        return f"Unknown task command: {" ".join(cmd_and_args)}", True
    task = await task_repository.get_task(task_id)
    if task is None or not task.conversations:
        return f"Task not found: {task_id}", True
    # 新对话的 work_dir 取任务的工作目录
    work_dir = str(task.work_dir)
    # 移交给用户：创建 agent_id 为 None 的用户审核对话
    if cmd_and_args[2] == "user":
        await conversation_repository.add_conversation(f"{task.title}-用户", work_dir, "", task_id, None)
        return "已将任务移交给用户，请对当前进展进行总结", False
    # 移交给智能体：校验 agent 存在且属于该任务团队
    try:
        agent_id = int(cmd_and_args[2])
    except ValueError:
        return f"Invalid agent_id: {cmd_and_args[2]}", True
    agent = await agent_repository.get_agent(agent_id)
    if agent is None or agent.id not in [team_agent.id for team_agent in task.agents]:
        return f"Agent not found in task team: {agent_id}", True
    await conversation_repository.add_conversation(f"{task.title}-{agent.name}", work_dir, str(agent.prompt), task_id, agent_id)
    return f"已将任务移交给智能体 {agent.name}，请对当前进展进行总结", False
