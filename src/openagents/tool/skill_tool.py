import json
import logging
import pathlib

import anyio
from pydantic import BaseModel


# Skill数据结构
class SkillInfo(BaseModel):
    name: str
    description: str
    path: str
    content: str


# Skill读取目录
SKILLS_DIR_LIST = [str(pathlib.Path.home() / ".openagents" / "skills"), str(pathlib.Path.home() / ".agents" / "skills")]
# 存储所有的skill信息
SKILLS: list[SkillInfo] = []


# 初始化所有skill信息
async def init_skills() -> None:
    loaded = set()
    # 遍历技能目录
    for skills_dir in map(anyio.Path, SKILLS_DIR_LIST):
        if not await skills_dir.exists():
            continue
        # 遍历技能
        async for skill_dir in skills_dir.iterdir():
            if skill_dir.name in loaded:
                continue
            skill_file = skill_dir / "SKILL.md"
            if not await skill_file.is_file():
                continue
            # 尝试读取 SKILL.md 提取 name 和 description
            text = await skill_file.read_text(encoding="utf-8")
            lines = [line.strip() for line in text.split("\n")]
            if lines[0] == "---" and "---" in lines[1:]:
                second_index = lines.index("---", 1)
            else:
                continue
            name, description = "", ""
            for line in lines[1:second_index]:
                if line.startswith("name:"):
                    name = line[5:].strip()
                elif line.startswith("description:"):
                    description = line[12:].strip()
            if name == skill_dir.name:
                content = text.split("---\n", 2)[2].strip()
                SKILLS.append(SkillInfo(name=name, description=description, path=str(await skill_file.resolve()), content=content))
                loaded.add(name)
    logging.info(f"Skills initialized, having {len(SKILLS)} skills")


def list_skills() -> list[SkillInfo]:
    return SKILLS


# 执行
async def execute(cmd_and_args: list[str], work_dir: str) -> tuple[str, bool]:
    # 1. skill list
    if len(cmd_and_args) == 2 and cmd_and_args[0] == "skill" and cmd_and_args[1] == "list":
        tool_content = json.dumps([{"name": s.name, "description": s.description, "path": s.path} for s in SKILLS], ensure_ascii=False)
        return tool_content, False
    return "Unknown command", True
