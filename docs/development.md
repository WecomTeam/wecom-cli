# 开发说明

这页面向仓库维护者和贡献者，记录源码结构、常用本地命令和打包边界。

## 仓库结构

| 路径 | 说明 |
| --- | --- |
| `src/` | Rust CLI 主实现，包括命令解析、认证、JSON-RPC、日志和媒体处理 |
| `bin/wecom.js` | npm 入口脚本，负责定位并执行当前平台的二进制 |
| `packages/*` | 各平台的 npm 二进制包 |
| `skills/*` | Agent Skills 及其补充参考资料 |
| `docs/` | 持续维护的使用与开发文档 |
| `README.md` | 项目首页 |

## 本地开发

仓库的 Rust crate 使用 `edition = "2024"`，开发时建议使用较新的 stable Rust 工具链。

说明：

- 根包名为 `@wecom/cli`，实际可执行入口是 `bin/wecom.js`。
- 平台二进制通过 `optionalDependencies` 分发，位于 `packages/*`。
- `pnpm-workspace.yaml` 当前只管理 `packages/*` 工作区。

## 自定义 MCP 配置地址

默认构建启用 `custom-endpoint` feature，可通过环境变量覆盖 `init` 获取 MCP 配置的地址：

```bash
WECOM_CLI_MCP_CONFIG_ENDPOINT=http://127.0.0.1:8080/mcp-config wecom-cli init
```

环境变量未设置或值为空时使用企业微信官方地址。需要禁用该能力时，可以使用
`cargo build --no-default-features` 构建。
