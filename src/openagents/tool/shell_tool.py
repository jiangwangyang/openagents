import asyncio
import logging
import subprocess
import sys

import anyio

POWERSHELL_SETTING = """# UTF-8 编码设置
[Console]::OutputEncoding = [System.Text.Encoding]::UTF8
$OutputEncoding = [System.Text.Encoding]::UTF8
"""


async def setup_powershell_utf8():
    # Windows才配置
    if sys.platform.startswith("win"):
        # 配置 Windows PowerShell 5.1 输出编码
        home_dir = await anyio.Path.home()
        profile_path = home_dir / "Documents" / "WindowsPowerShell" / "Microsoft.PowerShell_profile.ps1"
        await profile_path.parent.mkdir(parents=True, exist_ok=True)
        await profile_path.write_text(POWERSHELL_SETTING, encoding="utf-8")
        logging.info("已配置PowerShell默认编码")


async def execute(cmd_and_args: list[str], work_dir: str) -> tuple[str, bool]:
    process = await asyncio.create_subprocess_exec(*cmd_and_args, cwd=work_dir, stdout=subprocess.PIPE, stderr=subprocess.PIPE)
    stdout, stderr = await process.communicate()
    tool_content = f"{stdout.decode('utf-8', errors='replace')}{stderr.decode('utf-8', errors='replace')}"
    is_error = process.returncode != 0
    return tool_content, is_error
