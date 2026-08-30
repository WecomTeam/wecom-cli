# 开发说明

这页面向仓库维护者和贡献者，记录源码结构、常用本地命令和打包边界。

## 仓库结构

本仓库为 Cargo workspace，Rust 核心拆为三个 crate：

| 路径 | 说明 |
| --- | --- |
| `crates/wecom/` | 核心库（lib）：`Client`/`ClientBuilder`、discovery 服务发现与缓存、schema 指令（`x-wecom-*`）、builtins（媒体上传/下载）、HelperRegistry、端点目录（`EndpointKey`/`EndpointCatalog`）、网关扁平信封（`PayloadStringReq`/`NestedRes`） |
| `crates/wecom-auth/` | 鉴权库：`CredentialStore`/`TokenProvider` 抽象、加密文件凭据存储、botid+secret 签名引导、扫码登录网络流程、网关扁平协议信封与鉴权能力标记（`RequireAuth`/`SuppressAuth`） |
| `crates/wecom-runtime/` | 认证运行时：`WecomBackend`（持有 token 即注入 Bearer token、`RequireAuth` 前置门禁 + 853004 经 `TokenProvider` 静默刷新并重放一次）、`WecomClient` 门面与 `WecomClientBuilder`（第三方应用接入入口，见 `docs/library.md`）、端点目录覆写 |
| `crates/wecom-cli/` | 二进制（bin）：`main.rs` 组装 Client 并 `run`；`auth init/show` 终端交互（二维码渲染）；config/env/logging；`auth` 命令经扩展命令点挂载 |
| `crates/wecom-transport/` | 传输层：`TransportBackend` trait、reqwest HTTP 后端、长任务轮询、请求/响应信封 trait（`RequestEnvelope`/`ResponseEnvelope`）、端点目录泛型机制（`EndpointCatalog<K>`/`CatalogKey`） |
| `bin/wecom.js` | npm 入口脚本，负责定位并执行当前平台的二进制 |
| `packages/*` | 各平台的 npm 二进制包 |
| `skills/*` | Agent Skills 及其补充参考资料 |
| `docs/` | 持续维护的使用与开发文档 |
| `README.md` | 项目首页 |

调用链路（lib 内）：

```
请求前: collect_directives → process_media_upload / multipart（x-wecom-* 指令）
调用:   transport.invoke（Envelope 双轴解析 + taskid 触发 poll_long_task 轮询）
响应后: collect_directives → process_file_save → 输出路由
```

## 本地开发

仓库的 Rust crate 使用 `edition = "2024"`，开发时建议使用较新的 stable Rust 工具链。

常用命令：

```bash
# 全量检查 / 测试 / lint
cargo check --workspace
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings

# 构建并运行
cargo run -p wecom-cli -- --help
```

## 端到端测试

各 crate 的 e2e 套件统一放在各自 `<crate>/test-e2e/` 下（`run.rs` 编译入口 + `helpers/` + `cases/<group>/<NNN>-<slug>/{desc.md,test.rs}`），规范与生成手册见 [`e2e/`](e2e/FRAMEWORK.md)：

```bash
# library-level 套件（crates/wecom）
cargo test -p wecom --test e2e

# process-level 套件（crates/wecom-cli，需 custom-endpoint feature）
cargo test -p wecom-cli --test e2e --features custom-endpoint
```

说明：

- 根包名为 `@wecom/cli`，实际可执行入口是 `bin/wecom.js`。
- 平台二进制通过 `optionalDependencies` 分发，位于 `packages/*`。
- `pnpm-workspace.yaml` 当前只管理 `packages/*` 工作区。
- 扩展命令（如 `auth`）经 `ClientBuilder::command()` 挂载，无需改动 `main.rs` 与 lib 调度层；产品 helper 经 `ClientBuilder::helper()` 注册。
