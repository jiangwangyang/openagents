from fastapi import APIRouter, HTTPException, Body

from openagents.repository import agent_repository

router = APIRouter()


# 查询全部 Agent，按 id 升序
@router.get("/agent/list")
async def list_agents() -> list[dict]:
    agents = await agent_repository.list_agents()
    return [
        {
            "id": agent.id,
            "name": agent.name,
            "description": agent.description,
            "prompt": agent.prompt,
            "create_time": agent.create_time,
            "update_time": agent.update_time,
        }
        for agent in agents
    ]


# 按 id 查询 Agent，不存在返回 404
@router.get("/agent/{agent_id}")
async def get_agent(agent_id: int) -> dict:
    agent = await agent_repository.get_agent(agent_id)
    if agent is None:
        raise HTTPException(status_code=404, detail="Agent not found")
    return {
        "id": agent.id,
        "name": agent.name,
        "description": agent.description,
        "prompt": agent.prompt,
        "create_time": agent.create_time,
        "update_time": agent.update_time,
    }


# 新增 Agent，返回自增 id
@router.post("/agent")
async def add_agent(name: str = Body(..., embed=True), description: str = Body(..., embed=True), prompt: str = Body(..., embed=True)) -> int:
    return await agent_repository.add_agent(name, description, prompt)


# 按 id 更新 Agent，不存在返回 404
@router.put("/agent/{agent_id}")
async def update_agent(agent_id: int, name: str = Body(..., embed=True), description: str = Body(..., embed=True), prompt: str = Body(..., embed=True)) -> None:
    if not await agent_repository.update_agent(agent_id, name, description, prompt):
        raise HTTPException(status_code=404, detail="Agent not found")


# 按 id 删除 Agent，关联对话的 agent_id 由数据库外键 ON DELETE SET NULL 自动置空，不存在返回 404
@router.delete("/agent/{agent_id}")
async def delete_agent(agent_id: int) -> None:
    if not await agent_repository.delete_agent(agent_id):
        raise HTTPException(status_code=404, detail="Agent not found")
