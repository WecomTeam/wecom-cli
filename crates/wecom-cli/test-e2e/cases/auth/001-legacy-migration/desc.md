# 旧版凭据启动时自动迁移

- **场景**：配置目录下仅存在旧版 `bot.enc` 时，启动 CLI 自动迁移为 `credentials.enc`
- **Transport**：HTTP（mockito，仅鉴权引导端点）
- **来源**：凭据存储结构回归
- **Feature**：`custom-endpoint`（`skip-auth-check` 组合下不运行：该构建不注册 `auth` 命令）

## 测试等级

**P0**（迁移成功与失败两条路径）
- **条件**：临时配置目录预置旧版 `.encryption_key` + `bot.enc`，无 `credentials.enc`
- **断言**：成功路径落盘新凭据；失败路径按未授权启动且旧文件保留

## 子测试

| 测试 | 验证 |
|------|------|
| `migration_succeeds_keeps_legacy` | 引导端点返回 token → 落盘 `credentials.enc`，旧版 `bot.enc` 保留 |
| `migration_failure_keeps_legacy` | 引导端点返回业务错误 → 状态为未授权，不生成 `credentials.enc`，旧版 `bot.enc` 保留 |

## 前置条件

- 启用 `custom-endpoint` feature
- 临时配置目录写入 `.encryption_key`（base64 编码密钥）与 `bot.enc`（AES-256-GCM 加密的旧版 bot 凭据）
- mock server 挂载引导端点 `POST /get_cli_config`：
  - 成功路径返回 `{ "errcode": 0, "errmsg": "ok", "token": "tok-e2e" }`
  - 失败路径返回 `{ "errcode": 853000, "errmsg": "invalid credential" }`
- 环境变量：`WECOM_CLI_CONFIG_DIR=<tmp_dir>`、`WECOM_CLI_AUTH_ENDPOINT=<mock_server>/get_cli_config`

## 命令

```bash
WECOM_CLI_CONFIG_DIR=<tmp_dir> WECOM_CLI_AUTH_ENDPOINT=<mock_server>/get_cli_config wecom-cli auth show
```

## 断言 — CLI

- 退出码：`0`
- 成功路径 stdout：包含 `"Status: authorized"` 与 `"Bot ID: bot-e2e"`
- 失败路径 stdout：包含 `"Status: unauthorized"`（迁移失败静默降级，不报错）

## 断言 — HTTP Endpoint

- `POST /get_cli_config` 被调用 1 次（仅迁移尝试本身）

## 断言 — FS

- 成功路径：`<tmp_dir>/credentials.enc` 被创建；`<tmp_dir>/bot.enc` 保留
- 失败路径：`<tmp_dir>/credentials.enc` 不生成；`<tmp_dir>/bot.enc` 保留（后续启动可重试迁移）

## 关键上下文

- `crates/wecom-cli/src/auth/legacy_migration.rs`：`try_migrate_legacy_credentials` 在无 `credentials.enc` 且存在 `bot.enc` 时触发；解密失败、网络或业务错误一律按未授权降级，返回 `Ok` 且不清理旧文件。
- `crates/wecom-cli/src/auth/credentials.rs`：`legacy_paths()` 定位旧版文件，`save_credentials()` 原子写入新总账（0600）。
