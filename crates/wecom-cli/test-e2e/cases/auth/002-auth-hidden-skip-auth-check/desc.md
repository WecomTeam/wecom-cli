# skip-auth-check 构建屏蔽 auth 命令

- **场景**：skip-auth-check 构建下 auth 顶层命令未注册：调用报"命令未找到"、顶层 help 不出现
- **Transport**：HTTP（mockito，仅服务发现目录）
- **来源**：workbuddy 沙箱渠道的命令面收敛
- **Feature**：`custom-endpoint`（断言随 `skip-auth-check` 分叉）

## 测试等级

**P0**（命令面契约）：
- **条件**：mock discovery 返回固定目录（服务名 `hr`，不含 auth）；空配置目录、无 token
- **断言**：`auth ...` 调用与顶层 help 的 auth 行随 feature 分叉

## 子测试

| 测试 | 验证 |
|------|------|
| `auth_invocation_reflects_build_feature` | `auth show --status`：常规构建 exit 0 + unauthorized；skip 构建 exit 2 + `unrecognized subcommand 'auth'` |
| `top_level_help_reflects_build_feature` | `--help`：常规含 auth 命令行；skip 不含（首 token 精确比较） |

## 前置条件

- 启用 `custom-endpoint` feature（`WECOM_CLI_BASE_URL` 指向 mock）
- mock server 挂载 `POST /service/discovery` 返回固定目录（`hr`）
- 临时配置目录（`WECOM_CLI_CONFIG_DIR`），不含任何凭据文件
- 显式 `env_remove("WECOM_CLI_ACCESS_TOKEN")`：防止外部环境 token 干扰断言

## 命令

```bash
WECOM_CLI_CONFIG_DIR=<tmp_dir> WECOM_CLI_BASE_URL=<mock> wecom-cli auth show --status
WECOM_CLI_CONFIG_DIR=<tmp_dir> WECOM_CLI_BASE_URL=<mock> wecom-cli --help
```

## 断言 — CLI

- 常规构建：`auth show --status` 退出码 0、stdout 含 `unauthorized`；`--help` 的 Commands 列表含 `auth` 行
- skip-auth-check 构建：`auth show --status` 退出码 2、stdout 含 `unrecognized subcommand 'auth'`（stderr 含 SKILL 提示）；`--help` 不含 `auth` 行

## 断言 — HTTP Endpoint

- 常规构建的 `auth show --status` **不触网**（扩展命令命中即跳过目录拉取）
- skip 构建的 `auth show --status` 与两种构建的 `--help` 会拉取目录 → mock discovery 被调用

## 断言 — FS

无——不读写凭据文件（配置目录保持为空）。

## 关键上下文

- `crates/wecom-cli/src/cmd/mod.rs` / `main.rs`：注册点经 `cfg(not(feature = "skip-auth-check"))` 条件化，未注册时 auth 不进入 clap 顶层命令树。
- `crates/wecom/src/client/run/execute.rs:57-69`：扩展命令命中时跳过目录拉取；未注册时先拉目录，再走 clap `InvalidSubcommand`（退出码 2）。
- 断言文案 `unrecognized subcommand 'auth'` 来自 clap 4.6 的错误渲染；升级 clap 主版本时需同步。
