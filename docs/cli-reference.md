# CLI 命令参考

本文档描述 `wecom-cli` 的命令模型、参数约定、运行时路径、环境变量与错误格式。

## 命令模型

`wecom-cli` 的命令分为四类：

| 形态 | 说明 |
| --- | --- |
| `wecom-cli <service> [resource...] <method> [flags]` | 调用远程服务方法。`service` 由服务端 discovery 动态下发，方法可能带嵌套资源路径（如 `message aibot sessions list`） |
| `wecom-cli <service> +<helper> [flags]` | 本地 helper（`+` 前缀），由产品层注册 |
| `wecom-cli auth <init\|show>` | 授权管理（内建扩展命令） |
| `wecom-cli schema ...` / `wecom-cli cache ...` | 内建命令（隐藏在 `--help` 之外，见下文） |

说明：

- 服务目录与工具 schema 均在线获取，因此查看帮助与调用工具需要凭证与网络。
- 可用的 `service` 列表以 `wecom-cli --help` 实际输出为准。常见品类：`message`（消息）、`mail`（邮件）、`doc`（在线文档/文档管理）、`sheet`（在线表格）、`smartsheet`（智能表格）、`smartpage`（智能文档）、`calendar`（日程）、`meeting`（会议）、`todo`（待办）、`disk`（微盘）、`contact`（通讯录）、`media`（媒体文件）、`identity`（身份）。

## 配置凭证 `auth`

交互式配置企业微信机器人凭证，加密存储到本地（见「运行时路径」）。仅需执行一次：

```bash
# 配置凭证（交互式选择接入方式；非交互环境自动使用扫码接入）
wecom-cli auth init

# 查看授权状态
wecom-cli auth show
```

支持两种接入方式：

- **扫码接入（推荐）**：终端展示二维码，使用企业微信扫码创建绑定；扫码等待超时为 5 分钟
- **手动接入**：输入 Bot ID 和 Secret，获取方式[参考](https://open.work.weixin.qq.com/help2/pc/cat?doc_id=21677)

### `auth init` 参数

| 参数 | 说明 |
| --- | --- |
| `--noninteractive` | 跳过交互选择，直接使用扫码接入（CI/脚本/管道等非交互环境适用） |
| `--no-browser` | 扫码时不自动打开浏览器 |
| `--output-qrcode <PATH>` | 将二维码输出为 PNG 文件（仅支持当前目录下的路径，如 `qr.png`） |
| `--manual` | 跳过交互选择，手动输入 Bot ID 和 Secret（需要终端） |

### `auth show` 参数

默认输出人类可读的 `Status` 与 `Bot ID`。

| 参数 | 说明 |
| --- | --- |
| `--status` | 仅输出 `authorized` / `unauthorized` 单行，便于脚本判断 |

## 查看帮助 `--help`

支持获取各级命令的使用方式：

```bash
# 列出所有支持的命令和品类
wecom-cli --help

# 列出指定品类下的所有工具
wecom-cli <service> --help

# 列出指定工具所需的输入
wecom-cli <service> [resource...] <method> --help
```

此外，每个服务与方法都支持文档 flag：

- `wecom-cli <service> --schema` / `--doc`：输出服务 schema / 文档
- `wecom-cli <service> <method> --schema` / `--doc`：输出方法 schema / 文档（含 TS 类型声明）

`--version` 输出格式：`wecom-cli <version> (<distribution> <RFC 3339 构建时间> <git_commit_id>)`。

## 调用工具

通用格式：

```bash
wecom-cli <service> [resource...] <method> [--param value ...] [--json '<JSON>'] [flags]
```

请求体有三种给出方式，可组合：

| 方式 | 说明 |
| --- | --- |
| 命名参数 | 由方法 schema 生成的 clap 参数（如 `--id root`），类型与必填性以 `--help` 为准 |
| `--json '<JSON>'` | 直接给定完整请求体 JSON 字符串 |
| `--set path=value` | 深层路径覆盖，可重复（如 `--set extra.flag=true`）；非法 JSON 片段会自动修复 |

示例：

```bash
# 无参方法（含嵌套资源路径）
wecom-cli message aibot sessions list

# --json 给定请求体
wecom-cli doc search --json '{"keywords":["周报"],"limit":10}'
```

执行相关 flag（所有方法通用）：

| Flag | 说明 |
| --- | --- |
| `--dry-run` | 仅在本地校验并打印将发送的请求，不实际调用 |
| `--page-count <n>` | 启用游标式自动分页，最多拉取 n 页；输出为 NDJSON（每行一页） |
| `--page-delay <ms>` | 分页请求间隔毫秒数，默认 100 |
| `--output` / `-o <file>` | 将响应体写入文件 |
| `--output-dir <dir>` | 将响应与附件写入目录（分页时生成 `<method>.ndjson`） |

输出形态：

- 默认：compact JSON 输出到 stdout
- 写文件/下载：stdout 输出 `DownloadResult` JSON（含 `content_type`、`file_path`、`size`），文件以 `0600` 权限落盘
- 分页：NDJSON 多行输出

## 内建命令

以下命令隐藏在 `--help` 之外，面向调试与集成场景：

```bash
wecom-cli schema list                        # 列出所有服务及方法 schema
wecom-cli schema get <service.resource.method>  # 获取指定方法 schema（点分隔路径）
wecom-cli cache status                       # 查看 discovery 缓存状态
wecom-cli cache clear                        # 清除所有 discovery 缓存
```

## 运行时路径

| 项目 | 默认位置 | 备注 |
| --- | --- | --- |
| 配置目录 | `~/.config/wecom` | 可由 `WECOM_CLI_CONFIG_DIR` 覆盖 |
| 凭据文件 | `<config_dir>/credentials.enc` | `auth init` 时创建；AES-256-GCM 加密（0600），bot 信息与 token 共存于同一文件 |
| 加密密钥 | 系统 keyring 或 `<config_dir>/.encryption_key` | 无系统 keyring 时的文件回退（0600） |
| discovery 缓存 | `<config_dir>/cache` | 服务目录与 schema 缓存，TTL 60 秒 |
| 临时目录 | `<system_tmp>/wecom` | 媒体下载、请求暂存等；可由 `WECOM_CLI_TMP_DIR` 或 `config.json` 的 `tmp_dir` 覆盖 |

## 环境变量

| 变量 | 作用 |
| --- | --- |
| `WECOM_CLI_CONFIG_DIR` | 覆盖默认配置目录 |
| `WECOM_CLI_TMP_DIR` | 覆盖临时目录根目录 |
| `WECOM_CLI_ADDITIONAL_HEADERS` | 额外请求头，值为 JSON object（`Record<string, string>`）；同时支持 `WECOM_CLI_ADDITIONAL_HEADERS_*` 后缀形式的多个变量，取值同为 JSON object |
| `WECOM_CLI_LOG_LEVEL` | 打开 stderr 文本日志并设置过滤级别（如 `debug`、`wecom=trace`；非法值回退 `warn`） |
| `WECOM_CLI_LOG_DIR` | 打开 JSON Lines 日志输出，按天写入 `<dir>/ww.log.<日期>`（UTC+8） |

## 配置文件 `config.json`

存放于 `<config_dir>/config.json`，全部字段可选：

```json
{
    "headers": { "X-Custom": "value" },
    "tmp_dir": "/tmp/wecom-custom"
}
```

| 字段 | 作用 |
| --- | --- |
| `headers` | 额外请求头（别名 `additional_headers`） |
| `tmp_dir` | 覆盖临时目录根目录 |

说明：

- 环境变量优先级高于配置文件。
- access token 不允许经 `config.json` 配置，仅来自 `credentials.enc`。

## 退出码与错误格式

| 退出码 | 含义 |
| --- | --- |
| `0` | 成功（含 `--help` / `--version`） |
| `1` | 运行时错误（网络、鉴权、IO、后台业务错误等） |
| `2` | 用法错误（参数缺失、未知命令等；后台返回用法类错误码时也会渲染当前命令 help） |

错误以结构化 JSON 输出到 stdout：

```json
{
    "error": {
        "type": "AuthError",
        "code": 893201,
        "message": "..."
    }
}
```

- CLI 自身错误的 `code` 段：`893000–893099`（lib）、`893100–893199`（transport）、`893200–893299`（bin）、`893300–893399`（wecom-auth 鉴权库；经 CLI 错误映射后渲染为 bin 层对应码），`893999` 为共享兜底码。
- 后台业务错误（`errcode != 0`）直接透传后台响应体与原 `errcode`。
- 日志与提示信息一律走 stderr，不污染 stdout 的 JSON 输出。
