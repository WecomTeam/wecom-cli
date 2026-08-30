# AGENTS.md

> 面向 AI Agent 与贡献者的仓库导览：快速定位关键代码、理解核心机制、正确构建与测试。
> 本文只描述当前实现；改动代码后若结构或机制发生变化，请同步更新本文。

## 项目概述

`wecom-cli` 是企业微信官方 CLI（Rust 实现，经 npm 包 `@wecom/cli` 分发），覆盖消息、邮件、在线文档、智能文档、在线表格、智能表格、待办、日程、会议、微盘、通讯录等办公能力。仓库同时内置 `skills/` 下的 Agent Skills，供 AI Agent 调用 CLI 完成业务操作。

CLI 通过 discovery 协议从服务端动态下发服务目录与方法 schema，再在本地构建 clap 命令树。crate 依赖链：`wecom-cli` → `wecom-runtime` → `wecom-auth` → `wecom-transport`（`wecom-runtime` 同时依赖 `wecom`）。命令模型（按 `Client::run` 的调度顺序）：

```bash
wecom-cli --version | --help                       # 版本与帮助
wecom-cli auth <init|show>                         # 扩展命令：bin 侧经 ClientBuilder::command() 挂载
wecom-cli cache <status|clear>                     # 内建命令：discovery 缓存管理（help 中隐藏）
wecom-cli schema <list|get>                        # 内建命令：服务/方法 schema（help 中隐藏）
wecom-cli <service> --doc | --schema               # 服务级文档（service 由 discovery 下发）
wecom-cli <service> [resource...] <method> [flags] # 远程方法调用（方法参数由 schema 生成）
wecom-cli <service> +<helper> [flags]              # 本地 helper（+ 前缀，HelperRegistry 调度）
```

## 仓库结构

Cargo workspace（`resolver = "3"`，edition 2024）+ pnpm workspace（仅管理 `packages/*` 平台二进制包）：

| 路径 | 说明 |
| --- | --- |
| `crates/wecom/` | 核心库（lib）：`Client`/`ClientBuilder`、argv 调度、discovery 与缓存、schema 驱动命令树、指令处理、输出路由、沙箱 FS |
| `crates/wecom-auth/` | 鉴权库（lib）：`CredentialStore`/`TokenProvider` 抽象、加密文件凭据存储、botid+secret 签名引导、扫码登录网络流程、网关扁平协议信封与鉴权能力标记 |
| `crates/wecom-runtime/` | 认证运行时（lib）：`WecomBackend` 鉴权出网后端、`WecomClientBuilder`、端点目录覆写；组合 wecom-auth 与 wecom |
| `crates/wecom-cli/` | 二进制（bin）：`main.rs` 装配入口、`cmd/auth.rs` 终端交互、config/env/logging、transport 组装 |
| `crates/wecom-transport/` | 传输层：`TransportBackend` trait、reqwest HTTP 后端、信封 trait、端点目录泛型、长任务轮询 |
| `bin/wecom.js` | npm 入口脚本：定位并 exec 当前平台的二进制 |
| `packages/*` | 各平台 npm 二进制包（`optionalDependencies` 分发：darwin/linux × x64/arm64、win32-x64） |
| `skills/*` | 内置 Agent Skills（14 个，导航见 `docs/skills.md`） |
| `docs/` | 持续维护的使用与开发文档（入口 `docs/README.md`） |

## 模块职责

### `crates/wecom`（lib）

| 模块 | 职责 |
| --- | --- |
| `client/` | `Client`/`ClientBuilder`（`builder.rs`）；`run.rs` 的 `CliRun` 负责 argv 调度；`invoke.rs`/`upload.rs` 程序化调用；`custom_command.rs` 扩展命令点；`catalog.rs` 定义 `EndpointKey` 内建默认与 `PayloadStringReq` 请求信封 |
| `service/` | 服务调用链：`handler.rs` 服务内分发（helper 优先于 method）；`command/` 由 schema 构建 clap 子命令树（`build.rs`/`schema_clap.rs`）并装配请求体（`assemble.rs`，命名参数 + `--json` + `--set`）；`execute.rs` 执行与游标分页；`output.rs` 输出路由；`preview.rs` `--dry-run`；`doc.rs` `--doc`；`alias.rs` `path_alias` 隐藏别名；`service_handle.rs`/`method_handle.rs` 程序化句柄 |
| `registry/` | discovery 服务目录、schema 拉取与缓存（`<config_dir>/cache`，TTL 60 秒） |
| `schema/` | schema 类型、解析、TS 文档生成（`ts_doc.rs`） |
| `directive/` | `x-wecom-*` 指令：`UploadMedia`（媒体上传）、`UploadMultipart`（表单上传）、`Save`（响应字段落盘）；请求前与响应后各收集处理一次 |
| `builtins/` | 媒体上传的内建实现（`upload_media.rs`） |
| `helpers/` | `Helper` trait 与 `HelperRegistry`：以 `+` 前缀挂载在任意命令路径上的本地命令 |
| `fs/` | 沙箱文件系统 `Fs`（按 readable/writable roots 校验）、`PathResolver` 路径解析、文件名清洗 |
| `constants.rs` | `CLI_INFO`/`CliInfo`：编译期注入的版本信息（`--version` 输出与 `X-WeCom-Cli-Info` 请求头） |
| `error.rs` | lib 层统一错误（错误码段 893000–893099）；后台 errcode 经 transport 透传 |
| `telemetry/` | lib 侧 telemetry 事件 |

`ClientBuilder` 主要配置点：`transport`、`endpoint_catalog`（整体/逐 key 覆写端点目录）、`command`（扩展顶层命令，优先于同名服务）、`helper`、`bin_name`、`cwd`/`home_dir`/`tmp_dir`、`readable_dirs`/`writable_dirs`（沙箱）、`path_resolver`。

### `crates/wecom-auth`（鉴权库）

| 模块 | 职责 |
| --- | --- |
| `credentials.rs` | `Credentials` 凭据总账（bot + token）、`CredentialStore` trait（load/save/clear）与 `MemoryCredentialStore` 内存实现（服务端可插拔：KMS/Vault/共享 Secret Store 另行实现该 trait） |
| `file_store.rs` | `EncryptedFileCredentialStore`：单一凭据总账 `credentials.enc`（AES-256-GCM，0600，密钥存 `.encryption_key` + 系统 keyring 回退，`with_keyring(false)` 适配无 keyring 环境） |
| `crypto/` | `cipher.rs` AES-256-GCM 原语；`keystore.rs` 密钥编解码与 keyring 回退 |
| `provider.rs` | `TokenProvider` trait（`access_token` / `refresh`，刷新含并发合并）与 `BotGatewayTokenProvider`（botid+secret 签名引导换取 Bearer token，结果写回凭据存储） |
| `bootstrap.rs` | `sign`（`sha256_hex(secret+bot_id+time+nonce)`）与 `fetch_auth` 引导调用 |
| `gateway.rs` | AI Bot CLI 网关协议：扁平响应信封 `NestedRes`/`FlatRes`、鉴权能力标记 `RequireAuth`（门禁）/`SuppressAuth`（抑制注入）、`auth_endpoint(url)` 引导端点装配 |
| `qrcode.rs` | 扫码登录网络流程（创建会话 → 轮询结果，3s 间隔、5 分钟超时）；终端/PNG 渲染由调用方处理 |
| `legacy_migration.rs` | 启动时旧版 `bot.enc` 自动迁移（失败静默降级、旧文件保留） |
| `error.rs` | `AuthError`（错误码段 893300–893399） |

### `crates/wecom-runtime`（认证运行时）

| 模块 | 职责 |
| --- | --- |
| `backend.rs` | `WecomBackend`：持有 token 即注入 `Authorization: Bearer`（无 token 忽略；挂 `RequireAuth` 的端点为前置门禁，换取 token 的引导端点挂 `SuppressAuth` 抑制注入），命中 853004 时经 `TokenProvider` 静默刷新 token 并重放一次（载荷经 `HttpRequestPayload` 工厂重放） |
| `client.rs` | `WecomClient` 门面：包裹 `wecom::Client`（程序化方法调用 / `run(argv)` / 服务句柄） |
| `builder.rs` | `WecomClientBuilder`：base_url/超时/默认头 + token provider → `WecomClient`（`build`）或带鉴权的 `Transport`（`build_transport`）；第三方应用接入入口（见 `docs/library.md`） |
| `catalog.rs` | `endpoint_catalog()` 端点目录覆写（附鉴权能力与扁平信封） |
| `lib.rs` | 转出 wecom-auth 认证构件（下游可单依赖 wecom-runtime） |

### `crates/wecom-cli`（bin）

| 模块 | 职责 |
| --- | --- |
| `main.rs` | 装配入口：加载 `.env` 与 `config.json` → 初始化日志与 telemetry → 构建 transport 与 `Client` → `client.run(argv)`；命令未找到时在 stderr 追加 skill 更新提示 |
| `auth.rs` | 鉴权门面：re-export wecom-auth 认证构件；按 env/config 解析鉴权引导端点（`custom-endpoint` feature） |
| `cmd/auth.rs` | `auth init` / `auth show` 的 clap 定义与处理（终端交互、二维码终端/Unicode/PNG 渲染），经 `CustomCommand` 挂载 |
| `transport.rs` | transport 组装：凭据存储 + `BotGatewayTokenProvider` → `WecomBackend`（含旧版凭据迁移与 `WECOM_CLI_ACCESS_TOKEN` 覆盖） |
| `config.rs` | `config.json` 解析（全字段可选）与环境变量应用；env 优先级高于配置文件 |
| `env.rs` | `WECOM_CLI_*` 环境变量常量 |
| `logging.rs` | `WECOM_CLI_LOG_LEVEL`（stderr 文本日志）与 `WECOM_CLI_LOG_DIR`（JSON Lines 按天滚动，前缀 `ww.log`，UTC+8） |
| `telemetry.rs` | JSON 自动修复监听：修复成功时向 stderr 输出修复前后对照 |
| `error.rs` | bin 层统一错误（错误码段 893200–893299；wecom-auth 错误按变体映射入本层） |

### `crates/wecom-transport`（传输层）

| 模块 | 职责 |
| --- | --- |
| `traits.rs` / `transport.rs` / `builder.rs` | `TransportBackend` 开放 trait、`Transport` 统一句柄、`TransportBuilder` |
| `http/` | reqwest HTTP 后端；`HttpEndpoint`；信封双轴 trait `RequestEnvelope`/`ResponseEnvelope` 及默认实现 `PassthroughReq`/`GatewayRes`；`polling.rs` 长任务轮询；`resumable.rs` 断点续传下载 |
| `http_client/` | reqwest 封装（请求/响应/流式 body） |
| `common/` | `Endpoint`、`EndpointCatalog<K>`/`CatalogKey` 泛型端点目录、`PollEndpoint`、`RequestOptions`、`Extensions` 扩展袋、统一错误（段 893100–893199） |
| `dispatch.rs` | `TransportRequest`（`IntoFuture` 驱动）、轮询事件 `PollEvent`/`PollCallback` |
| `telemetry/` | `CaptureScope` 捕获域，供 bin 侧挂载事件监听 |

## 关键机制

- **服务发现**：`/service/discovery` 下发服务目录与 schema；结果缓存于 `<config_dir>/cache`（TTL 60 秒），`cache status`/`cache clear` 管理。
- **信封双轴**：请求侧 `RequestEnvelope::wrap` 与响应侧 `ResponseEnvelope::parse` 为正交 trait，挂在 `HttpEndpoint` 上。transport 仅含默认实现；网关扁平协议（请求 `{"payload": "<stringified-json>"}`、响应 `{errcode, errmsg, results_json}`）由产品层注入：`PayloadStringReq` 在 `wecom/src/client/catalog.rs`，`NestedRes`/`FlatRes` 在 `wecom-auth/src/gateway.rs`。
- **端点目录**：非 schema 驱动的 endpoint（服务发现、媒体上传/下载、轮询、schema 方法默认信封）统一登记在 `EndpointCatalog`；`EndpointKey::builtin_default` 提供内建默认，`wecom-cli` 经 `wecom_runtime::endpoint_catalog()` 覆写（附鉴权能力与扁平信封）。
- **鉴权注入**：`WecomBackend`（wecom-runtime）持有 token 即注入 Bearer token（无 token 则忽略）；挂 `RequireAuth` 标记的端点先过前置门禁——无 token 直接报错、请求不发出；换取 token 的鉴权引导端点挂 `SuppressAuth` 抑制注入（避免 853004 刷新自死锁）。token 失效（853004）经 `TokenProvider`（wecom-auth）静默换 token（并发刷新合并 + 写回凭据存储）并重放一次。
- **长任务轮询**：响应含 `taskid` 时按 `long_task_poll` 配置轮询（`PollClawLongTask`，`polling_interval_ms`/`task_timeout`），超时返回错误。
- **指令**：schema 中的 `x-wecom-*` 指令在请求前（媒体上传、multipart）与响应后（file-save、octet-stream 落盘）由 `directive/` 处理。
- **输出路由**：默认 compact JSON 到 stdout；`--output/-o` 写文件、`--output-dir` 写目录（返回 `DownloadResult` JSON）；`--page-count` 自动分页并输出 NDJSON。
- **JSON 修复**：`--json`/`--set` 中的非法 JSON 经 jsonrepair 自动修复，bin 侧监听在 stderr 输出修复前后对照。
- **错误模型**：三层嵌套 `wecom_cli::Error::Wecom(wecom::Error::Transport(wecom_transport::Error))`；错误码段 893000–893099 / 893100–893199 / 893200–893299 / 893300–893399（wecom-auth），共享兜底 893999；后台 errcode 原样透传；后台返回 10021 时渲染当前命令 help 并以退出码 2 返回。退出码约定：`0` 成功/帮助/版本，`1` 运行时错误，`2` 用法错误。
- **沙箱 FS**：文件读写经 `Fs` 按沙箱根校验（如 `auth init --output-qrcode` 仅允许解析后落在当前目录内的路径，相对/绝对皆可）。

## 构建与测试

常用命令（pnpm 脚本封装 cargo，定义见 `package.json`）：

```bash
pnpm fmt         # rustfmt +nightly 格式化全部 Rust 代码
pnpm lint        # cargo clippy --workspace --all-targets -- -D warnings
pnpm test        # cargo test --workspace --all-targets --features custom-endpoint
pnpm check       # cargo check --workspace --all-targets

# 本地构建并运行
cargo run -p wecom-cli -- --help
```

e2e 套件（规范见 `docs/e2e/`）：

```bash
cargo test -p wecom --test e2e                                    # library-level
cargo test -p wecom --test e2e --features custom-endpoint         # 含 custom-endpoint 用例
cargo test -p wecom-cli --test e2e --features custom-endpoint     # process-level（须带 feature）
cargo test -p wecom --test e2e run::method_call                   # 单个用例
```

Git 钩子由 lefthook 管理（`pnpm install` 时自动安装）：pre-commit 跑 rustfmt --check / clippy / eslint，commit-msg 跑 commitlint（Conventional Commits），pre-push 跑 `pnpm test` + `pnpm check`。

## 测试约定

- 单元测试随源码 `#[cfg(test)]` 模块组织。
- e2e 分两层：library-level（`crates/wecom/test-e2e/`，wiremock）与 process-level（`crates/wecom-cli/test-e2e/`，assert_cmd + mockito）。每个用例为 `cases/<group>/<NNN>-<slug>/{desc.md,test.rs}`，`desc.md` 规范见 `docs/e2e/DESC_SPEC.md`，代码生成手册见 `docs/e2e/CODEGEN.md`。
- `custom-endpoint` 为内部 feature（注入 `WECOM_CLI_BASE_URL` 等测试端点），仅用于开发与 e2e，不随发布构建启用，也不写入用户文档。

## 文档地图

| 文档 | 内容 |
| --- | --- |
| `README.md` | 项目首页（功能范围、安装、快速开始） |
| `docs/cli-reference.md` | CLI 使用参考：命令模型、auth、参数与 flag、运行时路径、环境变量、退出码 |
| `docs/skills.md` | 内置 Agent Skills 导航 |
| `docs/development.md` | 仓库结构与本地开发说明 |
| `docs/library.md` | Library 接入指南（第三方应用经 wecom-runtime 集成） |
| `docs/e2e/` | e2e 框架方案、desc.md 规范与生成手册 |
| `docs/skill-trimming-guide.md` | Skill 精简指南 |

## 维护约定

- 文档只描述**现状**，不记录历史实现与变更过程（变更历史归 `CHANGELOG.md` 与 git）。
- 同一主题只保留一个主入口，其余页面通过链接复用，避免多处维护漂移。
- 用户可见行为（命令、flag、环境变量、路径、错误码）变化时，同步更新 `docs/cli-reference.md`；结构或机制变化时，同步更新本文与 `docs/development.md`。
