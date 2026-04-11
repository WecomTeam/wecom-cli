# wecom-cli

[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)
[![Rust](https://img.shields.io/badge/rust-%3E%3D1.75-orange.svg)](https://www.rust-lang.org/)

> 💬 扫码加入企业微信交流群：
>
> <img src="https://wwcdn.weixin.qq.com/node/wework/images/202603241759.3fb01c32cc.png" alt="扫码入群交流" width="200" />

企业微信命令行工具 — 让人类和 AI Agent 都能在终端中操作企业微信。

## 功能范围

覆盖企业微信核心业务品类：

| 品类        | 能力                                                                          |
| ----------- | ----------------------------------------------------------------------------- |
| 👤 通讯录   | 获取可见范围成员列表、按姓名/别名搜索等                                       |
| ✅ 待办     | 创建/读取/更新/删除待办，变更用户处理状态等                                   |
| 🎥 会议     | 创建预约会议、取消会议、更新受邀成员、查询列表与详情等                        |
| 💬 消息     | 会话列表查询、消息记录拉取（文本/图片/文件/语音/视频）、多媒体下载、发送文本等 |
| 📅 日程     | 日程增删改查、参与人管理、多成员闲忙查询等                                    |
| 📄 文档     | 文档创建/读取/编辑等                                                          |
| 📊 智能表格 | 智能表格创建、子表与字段管理、记录增删改查等                                  |

## 快速开始

### 前置条件

- 支持平台：macOS (x64/arm64)、Linux (x64/arm64) 及 Windows (x64)
- Node.js `>= 18`
- 企业微信账号（目前仅对 ≤ 10 人企业开放使用）
- （可选）智能机器人 Bot ID 和 Secret，获取方式参考 [说明](https://open.work.weixin.qq.com/help2/pc/cat?doc_id=21677)

### 安装 & 使用

```bash
# 安装 CLI
npm install -g @wecom/cli

# 安装 CLI Skill（必需）
npx skills add WeComTeam/wecom-cli -y -g

# 配置凭证（交互式，仅需一次）
wecom-cli init

# 获取通讯录可见范围内的成员列表
wecom-cli contact get_userlist '{}'
```

📖 更多使用方法，请参阅 [CLI 命令参考](docs/cli-reference.md)。

## 卸载

如果需要彻底移除 wecom-cli，请参考 [卸载指南](docs/uninstall.md)，按照 CLI、Skill、配置三步清理。

## Agent Skills

🤖 支持的 Skills 使用说明，请参阅 [Skills 文档](docs/skills.md)。

## 许可证

本项目基于 [MIT 许可证](./LICENSE) 开源。
