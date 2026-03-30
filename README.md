# wecom-cli

[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)

> 💬 扫码加入企业微信交流群：
>
> <img src="https://wwcdn.weixin.qq.com/node/wework/images/202603241759.3fb01c32cc.png" alt="扫码入群交流" width="200" />

企业微信官方命令行工具，让人类和 AI Agent 都能在终端里操作企业微信。仓库同时提供 CLI、跨平台 npm 二进制分发，以及 12 个开箱即用的 Agent Skills。

[安装与快速开始](docs/getting-started.md) · [CLI 使用约定](docs/cli-reference.md) · [Skills 导航](docs/skills.md) · [开发说明](docs/development.md)

## 为什么使用 wecom-cli

- 面向 Agent：仓库内置 12 个 Skills，可直接接入支持 Skill 机制的 AI 工具。
- 覆盖核心业务：通讯录、待办、会议、消息、日程、文档、智能表格都可通过统一 CLI 访问。
- 安装简单：通过 npm 安装主包后，即可在当前平台获得对应的本地二进制。

## 能力概览

| 领域 | 能力 |
| --- | --- |
| `contact` | 获取可见范围成员列表，按姓名或别名匹配 |
| `todo` | 查询、创建、更新、删除待办，变更用户处理状态 |
| `meeting` | 创建预约会议，取消会议，查询会议列表和详情 |
| `msg` | 查询会话、拉取消息、下载媒体、发送文本消息 |
| `schedule` | 日程增删改查、参与人管理、多成员闲忙查询 |
| `doc` | 文档创建、导出、编辑，以及智能表格结构与数据管理 |

## 三分钟上手

运行前提：

- Node.js `>= 18`
- 企业微信机器人的 Bot ID 和 Secret

安装：

```bash
npm install -g @wecom/cli
npx skills add WeComTeam/wecom-cli -y -g
```

初始化：

```bash
wecom-cli init
```

第一次调用：

```bash
wecom-cli contact get_userlist '{}'
```

使用提示：

- `wecom-cli --help` 可以直接查看顶层帮助。
- `wecom-cli <category>` 和 `wecom-cli <category> --help` 需要先完成 `init`，因为会去拉取远程 MCP 工具定义。
- 对于无业务参数的工具，也建议显式传 `'{}'`；只写到 `<method>` 时，当前行为是显示 schema/help，而不是执行调用。

## 文档地图

- [`docs/README.md`](docs/README.md)：文档入口和维护边界
- [`docs/getting-started.md`](docs/getting-started.md)：安装、初始化、首个调用
- [`docs/cli-reference.md`](docs/cli-reference.md)：命令格式、帮助行为、运行时路径、环境变量
- [`docs/skills.md`](docs/skills.md)：12 个 Skills 的索引和入口
- [`docs/development.md`](docs/development.md)：源码结构、本地调试和打包边界

## Skills 一览

| 领域 | Skills |
| --- | --- |
| `contact` | `wecomcli-lookup-contact` |
| `todo` | `wecomcli-get-todo-list`, `wecomcli-get-todo-detail`, `wecomcli-edit-todo` |
| `meeting` | `wecomcli-create-meeting`, `wecomcli-edit-meeting`, `wecomcli-get-meeting` |
| `msg` | `wecomcli-get-msg` |
| `schedule` | `wecomcli-manage-schedule` |
| `doc` | `wecomcli-manage-doc`, `wecomcli-manage-smartsheet-schema`, `wecomcli-manage-smartsheet-data` |

更具体的工作流和参数示例请直接查看 [`docs/skills.md`](docs/skills.md) 中对应的 Skill 链接。

## 许可证

本项目基于 **MIT 许可证** 开源。
