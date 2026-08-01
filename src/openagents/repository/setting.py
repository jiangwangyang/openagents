import json
import os
import pathlib
from typing import TypedDict

import anyio

# 配置文件路径
SETTING_FILE = str(pathlib.Path.home() / ".openagents" / "setting.json")
# 内置模型提供商定义：(名称, base_url, API Key 环境变量, 模型列表)
PROVIDER_DEFS = [
    ("deepseek", "https://api.deepseek.com/anthropic", "DEEPSEEK_API_KEY", ["deepseek-v4-pro", "deepseek-v4-flash"]),
    ("bigmodel", "https://open.bigmodel.cn/api/anthropic", "BIGMODEL_API_KEY", ["glm-5.2", "glm-5.1", "glm-5-turbo", "glm-5"]),
    ("moonshot", "https://api.moonshot.cn/anthropic", "MOONSHOT_API_KEY", ["kimi-k3", "kimi-k2.7-code", "kimi-k2.6", "kimi-k2.5"]),
    ("minimaxi", "https://api.minimaxi.com/anthropic", "MINIMAXI_API_KEY", ["MiniMax-M3", "MiniMax-M2.7"]),
]


# 模型提供商配置
class ModelProvider(TypedDict):
    base_url: str
    api_key: str
    models: list[str]


# MCP 服务配置
class McpServer(TypedDict, total=False):
    type: str
    url: str
    headers: dict[str, str]
    command: str
    args: list[str]


# 全局配置
class Setting(TypedDict, total=False):
    model_provider: str
    model: str
    model_providers: dict[str, ModelProvider]
    mcp_servers: dict[str, McpServer]


async def init_setting():
    # 查询现有配置，文件损坏时备份后重建
    setting_file = anyio.Path(SETTING_FILE)
    content = await setting_file.read_text(encoding="utf-8") if await setting_file.exists() else ""
    try:
        setting: Setting = json.loads(content) if content.strip() else Setting()
    except json.JSONDecodeError:
        await setting_file.rename(SETTING_FILE + ".bak")
        setting = Setting()
    model_providers = setting.get("model_providers", {})
    # 根据环境变量自动补充缺失的模型提供商
    for name, base_url, env_key, models in PROVIDER_DEFS:
        api_key = os.getenv(env_key, "")
        if name not in model_providers and api_key:
            model_providers[name] = ModelProvider(
                base_url=base_url,
                api_key=api_key,
                models=models,
            )
    # 补充默认的提供商与模型，不覆盖用户已有配置
    if "model_provider" not in setting and model_providers:
        setting["model_provider"] = next(iter(model_providers))
    provider = model_providers.get(setting.get("model_provider", ""))
    if "model" not in setting and provider and provider["models"]:
        setting["model"] = provider["models"][0]
    setting["model_providers"] = model_providers
    # 配置有变化时才写入文件
    new_content = json.dumps(setting, ensure_ascii=False, indent=4)
    if new_content != content:
        await setting_file.parent.mkdir(parents=True, exist_ok=True)
        await setting_file.write_text(new_content, encoding="utf-8")
