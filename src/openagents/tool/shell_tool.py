import asyncio
import subprocess
import sys


async def execute(cmd_and_args: list[str], work_dir: str) -> tuple[str, bool]:
    encoding = "gbk" if sys.platform.startswith("win") else "utf-8"
    encoded = [x.encode(encoding, errors="replace") for x in cmd_and_args]
    process = await asyncio.create_subprocess_exec(*encoded, cwd=work_dir, stdout=subprocess.PIPE, stderr=subprocess.PIPE)
    stdout, stderr = await process.communicate()
    tool_content = f"{stdout.decode(encoding, errors='replace')}{stderr.decode(encoding, errors='replace')}"
    is_error = process.returncode != 0
    return tool_content, is_error
