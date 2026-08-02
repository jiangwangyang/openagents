import logging
import pathlib
import sys
import threading
from collections.abc import AsyncIterator
from contextlib import asynccontextmanager
from importlib.resources import files

from fastapi import FastAPI
from fastapi.staticfiles import StaticFiles

from openagents.api import conversation_api, work_api, app_api, model_provider_api, mcp_server_api, task_api, agent_api, skill_api
from openagents.repository import setting, database
from openagents.tool import mcp_tool
from openagents.tool import skill_tool

STATIC_PATH = files("openagents") / "static"
LOGGING_FILE = str(pathlib.Path.home() / ".openagents" / "app.log")
pathlib.Path(LOGGING_FILE).parent.mkdir(parents=True, exist_ok=True)
logging.basicConfig(
    level=logging.INFO,
    format="%(asctime)s - %(levelname)s - %(message)s",
    datefmt="%Y-%m-%d %H:%M:%S",
    handlers=[
        logging.FileHandler(LOGGING_FILE, mode="a", encoding="utf-8"),
        logging.StreamHandler(sys.stdout)
    ]
)
startup_event = threading.Event()


@asynccontextmanager
async def lifespan(app: FastAPI) -> AsyncIterator[None]:
    # 初始化设置
    await setting.init_setting()
    # 初始化技能
    await skill_tool.init_skills()
    # 数据库/mcp 生命周期管理
    async with database.lifespan():
        async with mcp_tool.lifespan():
            # 启动完成
            startup_event.set()
            logging.info("Application started")
            yield


app: FastAPI = FastAPI(lifespan=lifespan)
app.include_router(app_api.router)
app.include_router(conversation_api.router)
app.include_router(work_api.router)
app.include_router(model_provider_api.router)
app.include_router(mcp_server_api.router)
app.include_router(task_api.router)
app.include_router(agent_api.router)
app.include_router(skill_api.router)
app.mount("/static", StaticFiles(directory=str(STATIC_PATH)), name="static")
