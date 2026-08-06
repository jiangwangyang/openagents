# Switch to the project root directory (one level above the script directory)
Set-Location (Split-Path $PSScriptRoot -Parent)

# Check if uv is installed
if (-not (Get-Command uv -ErrorAction SilentlyContinue)) {
    Write-Host "Error: uv not found. Please install uv first: https://docs.astral.sh/uv/getting-started/installation/"
    exit 1
}

# Initialize/sync the uv environment (creates .venv and installs dependencies)
uv sync
if ($LASTEXITCODE -ne 0) { exit 1 }

# Start the application
uv run openagents
