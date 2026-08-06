@echo off

REM Switch to the project root directory (one level above the script directory)
cd /d "%~dp0.."

REM Check if uv is installed
where uv >nul 2>nul
if errorlevel 1 (
    echo Error: uv not found. Please install uv first: https://docs.astral.sh/uv/getting-started/installation/
    exit /b 1
)

REM Initialize/sync the uv environment (creates .venv and installs dependencies)
uv sync
if errorlevel 1 exit /b 1

REM Start the application
uv run openagents
