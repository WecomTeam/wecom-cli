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

常用命令：

```bash
cargo run -- --help
cargo run -- init --help
cargo test
```

如果你需要验证 npm 包装层：

```bash
pnpm install
```

说明：

- 根包名为 `@wecom/cli`，实际可执行入口是 `bin/wecom.js`。
- 平台二进制通过 `optionalDependencies` 分发，位于 `packages/*`。
- `pnpm-workspace.yaml` 当前只管理 `packages/*` 工作区。

## 文档维护建议

- 根 `README.md` 只保留首页信息和文档导航。
- 安装、CLI 行为和开发细节统一写入 `docs/`。
- 单个 Skill 的复杂示例保留在对应 `skills/*/SKILL.md`，不要在多个地方复制一遍。
