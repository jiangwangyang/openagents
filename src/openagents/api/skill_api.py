from fastapi import APIRouter

from openagents.tool import skill_tool

router = APIRouter()


# 查询全部已加载的 Skill
@router.get("/skill/list")
async def list_skills() -> list[dict]:
    return [{"name": skill["name"], "description": skill["description"], "path": skill["path"], "content": skill["content"]} for skill in skill_tool.SKILLS]
