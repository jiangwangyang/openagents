import logging
import pathlib
import sys
import threading
from contextlib import asynccontextmanager
from importlib.resources import files

from fastapi import FastAPI
from fastapi.staticfiles import StaticFiles

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
async def lifespan(app: FastAPI):
    # 启动完成
    startup_event.set()
    logging.info("Application started")
    yield


app: FastAPI = FastAPI(lifespan=lifespan)
app.mount("/static", StaticFiles(directory=str(STATIC_PATH)), name="static")
