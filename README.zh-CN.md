# OpenAgents

> Agents work for you.

[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](LICENSE)
![Platform](https://img.shields.io/badge/platform-Windows%20%7C%20macOS%20%7C%20Linux-blue)
![Version](https://img.shields.io/badge/version-0.1.0-green)

[English](README.md) | 中文

## 为什么选择 OpenAgents

- **绝对自由与掌控**
  不预置任何系统提示词, 不消耗额外 token, Agent 的行为 100% 由你写的提示词决定
- **极致效率**
  架构极简: 只提供一个命令工具, 模型的一切操作都通过命令执行; 上下文精简, 缓存完全命中, 又快又省
- **极致展示**
  模型的思考, 输出, 工具调用从上往下平铺展示, 无层级嵌套, AI 做的每一步一目了然
- **多智能体流水线**
  多个 Agent 串联协作, 上一个的输出作为下一个的输入, 组成自动化工作流
- **多主题配色动效**
  多套主题配色与动效, 按心情切换

## 功能特性

- 多 Agent 对话: 流式响应, 思考模式开关
- 多模型供应商接入: 支持 OpenAI / Anthropic 兼容协议
- MCP 服务器接入: 支持本地进程 / HTTP 两种方式, 无限扩展工具能力
- 技能 (Skills) 系统: 为 Agent 装载可复用的技能
- 任务系统 + 多智能体流水线: 让 Agent 替你完成复杂工作
- 定时调度: 使用 cron 表达式, 定时自动执行任务
- 主题与动效自定义
- 数据完全本地存储 (`~/.openagents`), API Key 不上传任何第三方

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

![多主题配色](docs/screenshots/theme-dark.png)

![多主题配色](docs/screenshots/theme-ink.png)

![多主题配色](docs/screenshots/theme-sunset.png)

![多主题配色](docs/screenshots/theme-aurora.png)

![多主题配色](docs/screenshots/theme-cyberpunk.png)

![多主题配色](docs/screenshots/theme-blackhole.png)

## 安装与运行

从 Releases 页面下载对应平台的可执行文件:

- Windows: `openagents.exe`
- macOS: `openagents`
- Linux: `openagents`

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

### 日志在哪里查看

日志按天滚动存储在 `~/.openagents/log/` 目录, 文件名为 `日期.log`

## 许可证

[MIT](LICENSE)
