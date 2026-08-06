#!/bin/sh

# Switch to the project root directory (one level above the script directory)
cd "$(dirname "$0")/.." || exit 1

# Check if uv is installed
if ! command -v uv >/dev/null 2>&1; then
    echo "Error: uv not found. Please install uv first: https://docs.astral.sh/uv/getting-started/installation/"
    exit 1
fi

# Initialize/sync the uv environment (creates .venv and installs dependencies)
uv sync || exit 1

# Start the application
uv run openagents
