import json
import os
import pathlib

import anyio
from pydantic import BaseModel, ConfigDict, Field

# 配置文件路径
SETTING_FILE = str(pathlib.Path.home() / ".openagents" / "setting.json")
# 内置模型提供商定义：(名称, base_url, API Key 环境变量, 模型列表)
PROVIDER_DEFS = [
    ("deepseek", "https://api.deepseek.com/anthropic", "DEEPSEEK_API_KEY", ["deepseek-v4-pro", "deepseek-v4-flash"]),
    ("bigmodel", "https://open.bigmodel.cn/api/anthropic", "BIGMODEL_API_KEY", ["glm-5.2", "glm-5.1", "glm-5-turbo", "glm-5"]),
    ("moonshot", "https://api.moonshot.cn/anthropic", "MOONSHOT_API_KEY", ["kimi-k3", "kimi-k2.7-code", "kimi-k2.6", "kimi-k2.5"]),
    ("minimaxi", "https://api.minimaxi.com/anthropic", "MINIMAXI_API_KEY", ["MiniMax-M3", "MiniMax-M2.7"]),
]


# 模型提供商配置，name 为 dict 键（存储时不保存，读取时从键回填，默认空串仅为通过校验），序列化时剔除防止脏数据
class ModelProvider(BaseModel):
    name: str = ""
    base_url: str
    api_key: str
    models: list[str]


# MCP 服务配置，name 为 dict 键（存储时不保存，读取时从键回填，默认空串仅为通过校验），序列化时剔除防止脏数据
class McpServer(BaseModel):
    name: str = ""
    description: str = ""
    type: str = ""
    url: str | None = None
    headers: dict[str, str] | None = None
    command: str | None = None
    args: list[str] | None = None


# 全局配置，允许配置文件中的未知字段（读写时保留不丢失）
class Setting(BaseModel):
    model_config = ConfigDict(extra="allow")

    model_provider: str | None = None
    model: str | None = None
    model_providers: dict[str, ModelProvider] = Field(default_factory=dict)
    mcp_servers: dict[str, McpServer] = Field(default_factory=dict)


# 读取配置文件，文件不存在返回空配置，文件损坏时备份后返回空配置，name 从 dict 键回填
async def load_setting() -> Setting:
    setting_file = anyio.Path(SETTING_FILE)
    content = await setting_file.read_text(encoding="utf-8") if await setting_file.exists() else ""
    try:
        setting = Setting.model_validate(json.loads(content)) if content.strip() else Setting()
    except (json.JSONDecodeError, ValueError):
        await setting_file.rename(SETTING_FILE + ".bak")
        setting = Setting()
    for name, provider in setting.model_providers.items():
        provider.name = name
    for name, server in setting.mcp_servers.items():
        server.name = name
    return setting


# 将配置写入文件，自动创建父目录，dict 中不保存 name 字段
async def save_setting(setting: Setting) -> None:
    setting_file = anyio.Path(SETTING_FILE)
    await setting_file.parent.mkdir(parents=True, exist_ok=True)
    dump = setting.model_dump(mode="json", exclude_none=True)
    dump["model_providers"] = {name: provider.model_dump(mode="json", exclude={"name"}) for name, provider in setting.model_providers.items()}
    dump["mcp_servers"] = {name: server.model_dump(mode="json", exclude={"name"}, exclude_none=True) for name, server in setting.mcp_servers.items()}
    content = json.dumps(dump, ensure_ascii=False, indent=4)
    await setting_file.write_text(content, encoding="utf-8")


# 初始化配置：加载现有配置（文件损坏时自动备份重建），根据环境变量补充内置提供商与默认选项，有变化时写回文件
async def init_setting() -> None:
    setting = await load_setting()
    before = setting.model_dump_json()
    # 根据环境变量自动补充缺失的模型提供商
    for name, base_url, env_key, models in PROVIDER_DEFS:
        api_key = os.getenv(env_key, "")
        if name not in setting.model_providers and api_key:
            setting.model_providers[name] = ModelProvider(name=name, base_url=base_url, api_key=api_key, models=models)
    # 补充默认的提供商与模型，不覆盖用户已有配置
    if setting.model_provider is None and setting.model_providers:
        setting.model_provider = next(iter(setting.model_providers))
    provider = setting.model_providers.get(setting.model_provider or "")
    if setting.model is None and provider and provider.models:
        setting.model = provider.models[0]
    # 配置有变化时才写入文件
    if setting.model_dump_json() != before:
        await save_setting(setting)
