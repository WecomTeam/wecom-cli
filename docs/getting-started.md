# 安装与快速开始

这页覆盖从安装 `wecom-cli` 到第一次成功调用的最短路径。更完整的 CLI 行为说明见 [`docs/cli-reference.md`](cli-reference.md)。

## 运行前提

- Node.js `>= 18`。
- 企业微信机器人的 Bot ID 和 Secret。
- 受支持的平台为：

| OS | CPU | 对应包 |
| --- | --- | --- |
| macOS | arm64 | `@wecom/cli-darwin-arm64` |
| macOS | x64 | `@wecom/cli-darwin-x64` |
| Linux | x64 | `@wecom/cli-linux-x64` |
| Windows | x64 | `@wecom/cli-win32-x64` |

## 安装 CLI 与 Skills

```bash
npm install -g @wecom/cli
npx skills add WeComTeam/wecom-cli -y -g
```

说明：

- `@wecom/cli` 主包通过可选依赖分发平台二进制，不需要单独安装平台包。
- 如果安装后提示找不到二进制，先检查 npm 是否禁用了 optional dependencies。

## 首次初始化

```bash
wecom-cli init
```

常用变体：

```bash
wecom-cli init --bot-id <BOT_ID>
wecom-cli init --refresh
```

说明：

- `init` 会交互式收集凭证、加密写入本地，并刷新 MCP 配置。
- `--refresh` 只刷新 MCP 后台配置，不会重新录入 Secret。

## 第一次调用

```bash
wecom-cli contact get_userlist '{}'
```

对于没有业务参数的工具，也建议显式传 `'{}'`。当前 CLI 在只写到 `<method>`、不传 `json_args` 时，会展示该工具的 schema/help，而不是直接发起调用。

## 下一步

- 想先看命令结构和帮助行为：[`docs/cli-reference.md`](cli-reference.md)
- 想知道有哪些内置 Skill：[`docs/skills.md`](skills.md)
- 想本地开发或调试：[`docs/development.md`](development.md)
