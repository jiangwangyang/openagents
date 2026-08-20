# OpenAgents

> Agents work for you.

[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](LICENSE)
![Platform](https://img.shields.io/badge/platform-Windows%20%7C%20macOS%20%7C%20Linux-blue)
![Version](https://img.shields.io/badge/version-0.1.0-green)

English | [中文](README.zh-CN.md)

## Why OpenAgents

- **Absolute freedom and control**
  No preset system prompts, no extra token consumption. The agent's behavior is 100% determined by the prompts you write
- **Extreme efficiency**
  Minimal architecture: only one command tool is provided, and every model operation is executed through commands; lean context with fully hit cache, fast and cost-effective
- **Extreme transparency**
  The model's thinking, output, and tool calls are displayed flat from top to bottom with no nested hierarchy, so every step the AI takes is clear at a glance
- **Multi-agent pipelines**
  Chain multiple agents together, where the output of one becomes the input of the next, forming automated workflows
- **Multiple themes and animations**
  Multiple color themes and animations, switch them to match your mood

## Features

- Multi-agent conversations: streaming responses, toggleable thinking mode
- Multiple model providers: supports OpenAI / Anthropic compatible protocols
- MCP server integration: supports both local process and HTTP modes, unlimited tool extensibility
- Skills system: equip agents with reusable skills
- Task system + multi-agent pipelines: let agents handle complex work for you
- Scheduled tasks: run tasks automatically on a schedule using cron expressions
- Customizable themes and animations
- All data stored locally (`~/.openagents`), API keys are never uploaded to any third party

## Screenshots

### Conversation: thinking, output, and tool calls displayed flat

![Conversation interface](docs/screenshots/conversation.png)

### Agent configuration: prompts entirely up to you

![Agent configuration](docs/screenshots/agent.png)

### Multi-agent pipeline: multiple agents working in a chain

![Multi-agent pipeline](docs/screenshots/pipeline.png)

### Scheduled tasks: triggered by cron expressions

![Scheduled tasks](docs/screenshots/schedule.png)

### Multiple themes and animations

![Multiple themes](docs/screenshots/theme-dark.png)

![Multiple themes](docs/screenshots/theme-ink.png)

![Multiple themes](docs/screenshots/theme-sunset.png)

![Multiple themes](docs/screenshots/theme-aurora.png)

![Multiple themes](docs/screenshots/theme-cyberpunk.png)

![Multiple themes](docs/screenshots/theme-blackhole.png)

## Installation & Usage

Download the executable for your platform from the Releases page:

- Windows: `openagents.exe`
- macOS: `openagents`
- Linux: `openagents`

### Desktop mode (default)

Double-click the executable to run it. The app window opens automatically, and closing the window exits the program

```bash
openagents
```

### Web mode

Run as a pure HTTP service accessed through your browser. Press `Ctrl-C` to stop

```bash
openagents --web
```

After starting, open in your browser: <http://127.0.0.1:8000>

## Quick Start

1. **Add a model provider**: fill in the Base URL and API Key, supports OpenAI / Anthropic compatible protocols
2. **Create an agent**: select a model, write your own prompts, and enable thinking mode as needed
3. **Start using it**: chat with agents, chain multiple agents into a pipeline, or set up scheduled tasks for automatic execution

## Data & Privacy

All data is stored locally under `~/.openagents` in your user directory:

| Path | Contents |
| --- | --- |
| `~/.openagents/database.db` | SQLite database (conversations, agents, tasks, configuration, etc.) |
| `~/.openagents/log/` | Runtime logs (rotated daily) |

- API keys are only stored in the local database and are never uploaded to any third party
- Backup: simply copy the entire `~/.openagents` directory
- Cleanup: delete the entire `~/.openagents` directory to completely remove all data

## FAQ

### Web mode reports port 8000 is in use

Close the program occupying port 8000, or switch to desktop mode (desktop mode uses a random port and does not have this issue)

### Model connection failed

1. Check that the model provider's Base URL and API Key are correct
2. Check network connectivity (whether a proxy is needed)
3. Check the logs to locate the cause: `~/.openagents/log/`

### Where to find the logs

Logs are stored in the `~/.openagents/log/` directory, rotated daily, with filenames in the format `date.log`

## License

[MIT](LICENSE)
