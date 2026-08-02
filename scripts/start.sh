#!/bin/sh

# 切换到项目根目录（脚本所在目录的上一级）
cd "$(dirname "$0")/.." || exit 1

# 检查 uv 是否已安装
if ! command -v uv >/dev/null 2>&1; then
    echo "错误: 未找到 uv，请先安装 uv: https://docs.astral.sh/uv/getting-started/installation/"
    exit 1
fi

# 初始化/同步 uv 环境（自动创建 .venv 并安装依赖）
uv sync || exit 1

# 启动应用
uv run openagents
