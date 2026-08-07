from fastapi import APIRouter, HTTPException, Body

from openagents.repository import model_provider_repository
from openagents.repository.database import ModelProviderEntity

router = APIRouter()


# 查询全部模型提供商，按 id 升序
@router.get("/model-provider/list")
async def list_model_providers() -> list[ModelProviderEntity]:
    return await model_provider_repository.list_model_providers()


# 按 id 查询模型提供商，不存在返回 404
@router.get("/model-provider/{provider_id}")
async def get_model_provider(provider_id: int) -> ModelProviderEntity:
    provider = await model_provider_repository.get_model_provider(provider_id)
    if provider is None:
        raise HTTPException(status_code=404, detail="Model provider not found")
    return provider


# 新增模型提供商，名称已存在返回 409
@router.post("/model-provider")
async def add_model_provider(name: str = Body(..., embed=True), type: str = Body(..., embed=True), base_url: str = Body(..., embed=True), api_key: str = Body(..., embed=True)) -> None:
    if await model_provider_repository.add_model_provider(name, type, base_url, api_key) is None:
        raise HTTPException(status_code=409, detail="Model provider already exists")


# 按 id 更新模型提供商，不存在或名称冲突返回 404
@router.put("/model-provider/{provider_id}")
async def update_model_provider(provider_id: int, name: str = Body(..., embed=True), type: str = Body(..., embed=True), base_url: str = Body(..., embed=True), api_key: str = Body(..., embed=True)) -> None:
    if not await model_provider_repository.update_model_provider(provider_id, name, type, base_url, api_key):
        raise HTTPException(status_code=404, detail="Model provider not found")


# 按 id 删除模型提供商，不存在返回 404，被 Agent 引用返回 409
@router.delete("/model-provider/{provider_id}")
async def delete_model_provider(provider_id: int) -> None:
    if await model_provider_repository.get_model_provider(provider_id) is None:
        raise HTTPException(status_code=404, detail="Model provider not found")
    if not await model_provider_repository.delete_model_provider(provider_id):
        raise HTTPException(status_code=409, detail="Model provider is referenced by agents")
