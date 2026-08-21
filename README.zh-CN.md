# OpenAgents

> Agents work for you.

[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](LICENSE)
![Platform](https://img.shields.io/badge/platform-Windows%20%7C%20macOS%20%7C%20Linux-blue)
[![GitHub release](https://img.shields.io/github/v/release/jiangwangyang/openagents)](https://github.com/jiangwangyang/openagents/releases)

[English](README.md) | 中文

## 你是否遇到过这些烦恼

- **Agent 不听你的**: 平台预置了大段隐藏的系统提示词, Agent 的行为被平台定义, 你想改却看不到也改不了
- **token 烧得心疼**: 预置提示词和海量工具定义每次请求都在消耗 token, 上下文臃肿缓存命不中, 又慢又贵, 账单失控却不知道钱花在哪
- **AI 是黑盒**: 工具调用层层折叠嵌套, AI 改了什么文件, 执行了什么命令, 一概看不清, 不敢放手让它干活
- **人肉流水线**: 一个 Agent 干不完复杂任务, 只能自己复制粘贴串联, 每天重复的活儿也没法自动定时跑
- **数据在别人手里**: 对话记录, 文件, API Key 都存在厂商云端, 担心隐私泄露, 也怕被厂商锁定

## 为什么选择 OpenAgents

- **绝对自由与掌控**: 不预置任何系统提示词, 不消耗额外 token, Agent 的行为 100% 由你写的提示词决定
- **极致效率**: 架构极简, 只提供一个命令工具, 模型的一切操作都通过命令执行; 上下文精简, 缓存完全命中, 又快又省
- **极致展示**: 模型的思考, 输出, 工具调用从上往下平铺展示, 无层级嵌套, AI 做的每一步一目了然
- **开放扩展**: 通过 MCP 服务器和技能 (Skills) 系统按需为 Agent 装载能力, 工具生态不受限
- **多智能体流水线**: 多个 Agent 串联协作, 上一个的输出作为下一个的输入, 组成自动化工作流
- **定时自动化**: 使用 cron 表达式定时触发任务, 重复性工作交给 Agent 自动完成
- **数据完全属于你**: 所有数据本地存储 (`~/.openagents`), API Key 不上传任何第三方
- **多主题配色动效**: 多套主题配色与动效, 按心情切换

## 界面预览

### 对话: 思考, 输出, 工具调用平铺展示

![对话界面](docs/screenshots/conversation.png)

### Agent 配置: 提示词完全由你决定

![Agent 配置](docs/screenshots/agent.png)

### 多智能体流水线: 多个 Agent 串联协作

![多智能体流水线](docs/screenshots/pipeline.png)

### 定时任务: cron 表达式定时触发

![定时任务](docs/screenshots/schedule.png)

### 多主题配色动效

| ![blackhole](docs/screenshots/theme-blackhole.png) | ![dark](docs/screenshots/theme-dark.png) |
| --- | --- |
| ![ink](docs/screenshots/theme-ink.png) | ![sunset](docs/screenshots/theme-sunset.png) |
| ![aurora](docs/screenshots/theme-aurora.png) | ![cyberpunk](docs/screenshots/theme-cyberpunk.png) |

## 安装与运行

从 [Releases](https://github.com/jiangwangyang/openagents/releases) 页面下载对应平台的压缩包, 解压后即可使用:

| 平台 | 文件 |
| --- | --- |
| Windows (x86_64) | `openagents-windows-x86_64.zip` |
| macOS (Apple Silicon) | `openagents-macos-aarch64.zip` |
| Linux (x86_64) | `openagents-linux-x86_64.zip` |
| Linux (ARM64) | `openagents-linux-aarch64.zip` |

压缩包内为单个可执行文件: Windows 为 `openagents.exe`, macOS / Linux 为 `openagents` (解压后需赋予执行权限: `chmod +x openagents`)

### 桌面模式 (默认)

直接双击运行可执行文件, 自动打开应用窗口, 关闭窗口即退出程序

```bash
openagents
```

### Web 模式

以纯 HTTP 服务方式运行, 通过浏览器访问, 按 `Ctrl-C` 停止

```bash
openagents --web
```

启动后在浏览器打开: <http://127.0.0.1:8000>

## 快速上手

1. **添加模型供应商**: 填入 Base URL 和 API Key, 支持 OpenAI / Anthropic 兼容协议
2. **创建 Agent**: 选择模型, 编写你自己的提示词, 按需开启思考模式
3. **开始使用**: 与 Agent 对话, 或将多个 Agent 组成流水线, 或设置定时任务自动执行

## 数据与隐私

所有数据均存储在本地用户目录下的 `~/.openagents`:

| 路径 | 内容 |
| --- | --- |
| `~/.openagents/database.db` | SQLite 数据库 (对话, Agent, 任务, 配置等) |
| `~/.openagents/log/` | 运行日志 (按天滚动) |

- API Key 仅存储在本地数据库, 不会上传任何第三方
- 备份: 复制整个 `~/.openagents` 目录即可
- 清理: 删除整个 `~/.openagents` 目录即可完全移除数据

## 常见问题

### Web 模式提示 8000 端口被占用

关闭占用 8000 端口的程序, 或改用桌面模式 (桌面模式使用随机端口, 无此问题)

### 模型连接失败

1. 检查模型供应商的 Base URL 和 API Key 是否正确
2. 检查网络连通性 (是否需要代理)
3. 查看日志定位原因: `~/.openagents/log/`

### macOS 提示无法打开, 因为无法验证开发者

首次运行在终端执行 `xattr -d com.apple.quarantine openagents`, 或在 系统设置 → 隐私与安全性 中允许运行

### 如何更新版本

重新下载最新压缩包替换可执行文件即可, 数据保存在 `~/.openagents`, 不受影响

### 日志在哪里查看

日志按天滚动存储在 `~/.openagents/log/` 目录, 文件名为 `日期.log`

## 许可证

[MIT](LICENSE)
