from openagents.repository import task_repository, conversation_repository, agent_repository
from openagents.service import task_service


async def execute(cmd_and_args: list[str], work_dir: str) -> tuple[str, bool]:
    # 任务移交：从当前 async 上下文读取 task_id，为该任务创建新对话，run_task 下一轮循环自动接续
    # 顶层导入 task_service 不构成循环依赖：导入本模块时 task_service 已在 sys.modules 中，其函数仅在运行时被访问
    task_id = task_service.get_task_id()
    if task_id is None:
        return "Not in a task context, cannot hand over", True
    if len(cmd_and_args) < 3 or cmd_and_args[1] != "handover":
        return f"Unknown task command: {' '.join(cmd_and_args)}", True
    task = await task_repository.get_task(task_id)
    if task is None or not task.conversations:
        return f"Task not found: {task_id}", True
    # 新对话的 work_dir 取任务的工作目录
    work_dir = str(task.work_dir)
    # 移交给用户：创建 agent_id 为 None 的用户审核对话
    if cmd_and_args[2] == "user":
        await conversation_repository.add_conversation(f"{task.title}-用户", work_dir, "", task_id, None)
        return "Task handed over to the user, please summarize the current progress", False
    # 移交给智能体：校验 agent 存在且属于该任务团队
    try:
        agent_id = int(cmd_and_args[2])
    except ValueError:
        return f"Invalid agent_id: {cmd_and_args[2]}", True
    agent = await agent_repository.get_agent(agent_id)
    if agent is None or agent.id not in [team_agent.id for team_agent in task.agents]:
        return f"Agent not found in task team: {agent_id}", True
    await conversation_repository.add_conversation(f"{task.title}-{agent.name}", work_dir, str(agent.prompt), task_id, agent_id)
    return f"Task handed over to agent {agent.name}, please summarize the current progress", False
