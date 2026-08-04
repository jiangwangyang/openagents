from fastapi import APIRouter, HTTPException, Body

from openagents.repository import model_provider_repository
from openagents.repository.setting import ModelProvider

router = APIRouter()


# 获取当前选中的模型提供商，未选中返回 404
@router.get("/model-provider/current")
async def get_current_model_provider() -> ModelProvider:
    provider = await model_provider_repository.get_current_model_provider()
    if provider is None:
        raise HTTPException(status_code=404, detail="No model provider selected")
    return provider


# 修改当前选中的模型提供商，名称不存在返回 400
@router.put("/model-provider/current")
async def set_current_model_provider(name: str = Body(..., embed=True)) -> None:
    if not await model_provider_repository.set_current_model_provider(name):
        raise HTTPException(status_code=400, detail="Model provider not found")


# 获取当前选中的模型，未选中返回 404
@router.get("/model/current")
async def get_current_model() -> dict:
    model = await model_provider_repository.get_current_model()
    if model is None:
        raise HTTPException(status_code=404, detail="No model selected")
    return {"model": model}


# 修改当前选中的模型，模型不存在返回 400
@router.put("/model/current")
async def set_current_model(model: str = Body(..., embed=True)) -> None:
    if not await model_provider_repository.set_current_model(model):
        raise HTTPException(status_code=400, detail="Model not found in current provider")


# 获取 thinking 开关状态
@router.get("/thinking")
async def get_thinking() -> dict:
    return {"thinking": await model_provider_repository.get_thinking()}


# 修改 thinking 开关状态
@router.put("/thinking")
async def set_thinking(thinking: bool = Body(..., embed=True)) -> None:
    await model_provider_repository.set_thinking(thinking)


# 查询全部模型提供商，按名称升序
@router.get("/model-provider/list")
async def list_model_providers() -> list[ModelProvider]:
    return await model_provider_repository.list_model_providers()


# 按名称查询模型提供商，不存在返回 404
@router.get("/model-provider/{name}")
async def get_model_provider(name: str) -> ModelProvider:
    provider = await model_provider_repository.get_model_provider(name)
    if provider is None:
        raise HTTPException(status_code=404, detail="Model provider not found")
    return provider


# 新增模型提供商，名称已存在返回 409
@router.post("/model-provider")
async def add_model_provider(name: str = Body(..., embed=True), base_url: str = Body(..., embed=True), api_key: str = Body(..., embed=True), models: list[str] = Body(..., embed=True)) -> None:
    if not await model_provider_repository.add_model_provider(name, base_url, api_key, models):
        raise HTTPException(status_code=409, detail="Model provider already exists")


# 按名称更新模型提供商，不存在返回 404
@router.put("/model-provider/{name}")
async def update_model_provider(name: str, base_url: str = Body(..., embed=True), api_key: str = Body(..., embed=True), models: list[str] = Body(..., embed=True)) -> None:
    if not await model_provider_repository.update_model_provider(name, base_url, api_key, models):
        raise HTTPException(status_code=404, detail="Model provider not found")


# 按名称删除模型提供商，不存在返回 404
@router.delete("/model-provider/{name}")
async def delete_model_provider(name: str) -> None:
    if not await model_provider_repository.delete_model_provider(name):
        raise HTTPException(status_code=404, detail="Model provider not found")
