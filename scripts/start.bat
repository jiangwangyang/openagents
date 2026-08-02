@echo off

REM 切换到项目根目录（脚本所在目录的上一级）
cd /d "%~dp0.."

REM 检查 uv 是否已安装
where uv >nul 2>nul
if errorlevel 1 (
    echo 错误: 未找到 uv，请先安装 uv: https://docs.astral.sh/uv/getting-started/installation/
    exit /b 1
)

REM 初始化/同步 uv 环境（自动创建 .venv 并安装依赖）
uv sync
if errorlevel 1 exit /b 1

REM 启动应用
uv run openagents
