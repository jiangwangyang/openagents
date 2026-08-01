from openagents.repository.setting import ModelProvider, load_setting, save_setting


# 查询全部模型提供商，按名称升序
async def list_model_providers() -> list[ModelProvider]:
    setting = await load_setting()
    return list(setting.model_providers.values())


# 按名称查询模型提供商，不存在返回 None
async def get_model_provider(name: str) -> ModelProvider | None:
    setting = await load_setting()
    return setting.model_providers.get(name)


# 新增模型提供商，名称已存在返回 False
async def add_model_provider(name: str, base_url: str, api_key: str, models: list[str]) -> bool:
    setting = await load_setting()
    if name in setting.model_providers:
        return False
    setting.model_providers[name] = ModelProvider(name=name, base_url=base_url, api_key=api_key, models=models)
    await save_setting(setting)
    return True


# 按名称更新模型提供商，不存在返回 False
async def update_model_provider(name: str, base_url: str, api_key: str, models: list[str]) -> bool:
    setting = await load_setting()
    if name not in setting.model_providers:
        return False
    setting.model_providers[name] = ModelProvider(name=name, base_url=base_url, api_key=api_key, models=models)
    await save_setting(setting)
    return True


# 按名称删除模型提供商，不存在返回 False
async def delete_model_provider(name: str) -> bool:
    setting = await load_setting()
    if name not in setting.model_providers:
        return False
    del setting.model_providers[name]
    await save_setting(setting)
    return True
