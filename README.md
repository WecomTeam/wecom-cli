# wecom-cli

[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)
[![Rust](https://img.shields.io/badge/rust-%3E%3D1.75-orange.svg)](https://www.rust-lang.org/)

> 💬 扫码加入企业微信交流群：
>
> <img src="https://wwcdn.weixin.qq.com/node/wework/images/202603241759.3fb01c32cc.png" alt="扫码入群交流" width="200" />

企业微信命令行工具，覆盖消息、邮件、文档、待办、日程、会议、微盘、通讯录等业务功能。支持机器人主动通知、新建与读取文档、文档搜索、新建与管理日程、预约与获取会议信息、新建与跟进待办、上传与获取微盘文件、发送与获取邮件、获取通讯录成员信息，以提升企业办公效率

## 功能范围

覆盖企业微信核心业务品类：

| 品类          | 能力                                                                    |
| ------------- | ----------------------------------------------------------------------- |
| 💬 消息       | 向机器人最近对话过的单聊/群聊主动推送消息，支持 Markdown/图片/文件/语音/视频消息  |
| 📧 邮件       | 邮件发送/回复/转发，邮件搜索，获取邮件内容详情          |
| 📄 文档       | 在线文档的新建、导入、读取、追加与覆盖写入                          |
| 🗂️ 文档管理   | 多种文档类型的搜索，在线文档/在线表格/智能表格/智能文档的重命名、成员权限与加入规则管理 |
| 📊 在线表格   | 在线表格新建、CSV/Excel 导入、内容读改、追加行、子表管理                |
| 🧮 智能表格   | 智能表格创建，子表/字段/记录/视图/图表管理，行列样式修改                |
| 📰 智能文档   | 智能文档创建、获取页面内容、编辑文档内容、内置数据表信息获取              |
| ✅ 待办       | 创建/读取/更新/删除待办，分派参与人与完成待办等                         |
| 📅 日程       | 日程增删改查、参与人管理、多成员闲忙查询、会议室查询预订等      |
| 🎥 会议       | 创建预约会议、取消会议、更新参会人、查询列表与详情、读取会议纪要与转写原文    |
| 💾 微盘       | 微盘文件的搜索、基础信息读取、上传、下载 |
| 👤 通讯录     | 按姓名/拼音/别名搜索成员，获取成员基本信息，以用于会议、日程等多人场景         |

## 快速开始

### 前置条件

- 支持平台：macOS (x64/arm64)、Linux (x64/arm64) 及 Windows (x64)
- Node.js `>= 18`
- 企业微信账号
- （可选）智能机器人 Bot ID 和 Secret，获取方式参考 [说明](https://open.work.weixin.qq.com/help2/pc/cat?doc_id=21677)

### 安装 & 使用

```bash
# 安装 CLI
npm install -g @wecom/cli

# 安装 CLI Skill（必需）
npx skills add WeComTeam/wecom-cli -y -g

# 配置凭证（交互式，仅需一次；默认扫码，--manual 可手动输入）
wecom-cli auth init

# 查看授权状态
wecom-cli auth show
```

📖 更多使用方法，请参阅 [CLI 命令参考](docs/cli-reference.md)。

### 作为 Library 集成（Rust）

不经过命令行，在 Rust 应用中直接嵌入带鉴权的客户端（动态服务发现 + 方法调用 + token 自动刷新）：

```rust
use std::sync::Arc;
use wecom_runtime::{BotGatewayTokenProvider, MemoryCredentialStore, WecomClientBuilder};

let provider = BotGatewayTokenProvider::new(
    bot_id,
    secret,
    Arc::new(MemoryCredentialStore::default()),
);
let client = WecomClientBuilder::new()
    .token_provider(Arc::new(provider))
    .build()
    .await?;
```

📖 详见 [Library 接入指南](docs/library.md)。

## Agent Skills

🤖 支持的 Skills 使用说明，请参阅 [Skills 文档](docs/skills.md)。

## 许可证

本项目基于 [MIT 许可证](./LICENSE) 开源。
