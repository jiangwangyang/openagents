import anyio
from fastapi import APIRouter, HTTPException, Query
from starlette.responses import RedirectResponse

router = APIRouter()


@router.get("/")
async def index():
    return RedirectResponse("/static/index.html")


@router.get("/dir/list")
async def list_directory(path: str = Query(...)):
    # 如果没有传入路径，默认使用tmp目录
    if path:
        target_path = anyio.Path(path)
        if not await target_path.exists() or not await target_path.is_dir():
            raise HTTPException(status_code=404, detail="NOT FOUND")
    else:
        target_path = anyio.Path("/tmp")
        await target_path.mkdir(parents=True, exist_ok=True)
    # 只列出目录
    directories = [{
        "name": child_path.name,
        "path": str(await child_path.resolve())
    } async for child_path in target_path.iterdir() if await child_path.is_dir()]
    return {
        "current_path": str(await target_path.resolve()),
        "parent_path": str(await target_path.parent.resolve()),
        "directories": sorted(directories, key=lambda x: x["name"])
    }
