# OpenAgents

> Agents work for you.

[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](LICENSE)
![Platform](https://img.shields.io/badge/platform-Windows%20%7C%20macOS%20%7C%20Linux-blue)
[![GitHub release](https://img.shields.io/github/v/release/jiangwangyang/openagents)](https://github.com/jiangwangyang/openagents/releases)

English | [中文](README.zh-CN.md)

## Sound familiar

- **The agent won't listen to you**: platforms inject large hidden system prompts, the agent's behavior is defined by the platform, and you can neither see nor change it
- **Tokens burn a hole in your pocket**: preset prompts and a mountain of tool definitions consume tokens on every request, bloated context misses the cache, slow and expensive, bills spiral out of control with no idea where the money goes
- **AI is a black box**: tool calls are folded in nested layers, you can't see what files it changed or what commands it ran, so you dare not let it work unsupervised
- **You are the pipeline**: one agent can't finish complex tasks, so you copy-paste outputs from one to the next yourself, and daily repetitive work can't run automatically on a schedule
- **Your data is in someone else's hands**: conversations, files, and API keys are stored on the vendor's cloud, risking privacy leaks and vendor lock-in

## Why OpenAgents

- **Absolute freedom and control**: no preset system prompts, no extra token consumption, the agent's behavior is 100% determined by the prompts you write
- **Extreme efficiency**: minimal architecture, only one command tool is provided, and every model operation is executed through commands; lean context with fully hit cache, fast and cost-effective
- **Extreme transparency**: the model's thinking, output, and tool calls are displayed flat from top to bottom with no nested hierarchy, so every step the AI takes is clear at a glance
- **Open extensibility**: equip agents with capabilities on demand through MCP servers and the skills system, no limits on your tool ecosystem
- **Multi-agent pipelines**: chain multiple agents together, where the output of one becomes the input of the next, forming automated workflows
- **Scheduled automation**: trigger tasks on a schedule with cron expressions, let agents handle repetitive work automatically
- **Your data belongs to you**: all data is stored locally (`~/.openagents`), API keys are never uploaded to any third party
- **Multiple themes and animations**: multiple color themes and animations, switch them to match your mood

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

| ![blackhole](docs/screenshots/theme-blackhole.png) | ![dark](docs/screenshots/theme-dark.png) |
| --- | --- |
| ![ink](docs/screenshots/theme-ink.png) | ![sunset](docs/screenshots/theme-sunset.png) |
| ![aurora](docs/screenshots/theme-aurora.png) | ![cyberpunk](docs/screenshots/theme-cyberpunk.png) |

## Installation & Usage

Download the archive for your platform from the [Releases](https://github.com/jiangwangyang/openagents/releases) page and extract it:

| Platform | File |
| --- | --- |
| Windows (x86_64) | `openagents-windows-x86_64.zip` |
| macOS (Apple Silicon) | `openagents-macos-aarch64.zip` |
| Linux (x86_64) | `openagents-linux-x86_64.zip` |
| Linux (ARM64) | `openagents-linux-aarch64.zip` |

The archive contains a single executable: `openagents.exe` on Windows, `openagents` on macOS / Linux (grant execute permission after extraction: `chmod +x openagents`)

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

### macOS says the app cannot be opened because the developer cannot be verified

On first run, execute `xattr -d com.apple.quarantine openagents` in the terminal, or allow it in System Settings → Privacy & Security

### How to update

Download the latest archive and replace the executable. Your data is stored in `~/.openagents` and is not affected

### Where to find the logs

Logs are stored in the `~/.openagents/log/` directory, rotated daily, with filenames in the format `date.log`

## License

[MIT](LICENSE)
