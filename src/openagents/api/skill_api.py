from fastapi import APIRouter

from openagents.tool import skill_tool
from openagents.tool.skill_tool import SkillInfo

router = APIRouter()


# 查询全部已加载的 Skill
@router.get("/skill/list")
async def list_skills() -> list[SkillInfo]:
    return skill_tool.list_skills()
