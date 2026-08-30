# e2e 测试框架方案

> 本文档描述 workspace 内各 crate e2e 测试的组织方式与运行机制，所有 crate 的 e2e 套件统一遵循。项目通过 discovery 协议 + HTTP 传输（网关扁平协议）调用远程服务。

## 一、核心决策

### 1.1 测试层级：library-level vs process-level

| 层级 | 位置 | 形态 | 断言能力 | 适用场景 |
|---|---|---|---|---|
| **Library-level** | `crates/wecom/test-e2e/` | 构造 `wecom::Client`，调用 `Client::run(argv)` | stdout（SharedBuf 捕获）+ mock 请求 + FS | discovery、schema、method call、分页、长任务、指令、输出路由等全部 lib 行为 |
| **Process-level** | `crates/wecom-cli/test-e2e/` | `assert_cmd` 启动真实 `wecom-cli` 二进制 | exit code + stdout/stderr + FS | 仅 `main.rs` 入口独有逻辑：config.json 解析失败、logging 初始化、json repair stderr 监听、旧版凭据自动迁移 |

**取舍原则**：

- 只有 `main.rs` 入口逻辑**必须** process-level：`config/004-invalid-config-json`（`load_config_file` 在 `client.run()` 之前）、`logging/*`（`init_logging()` 只在二进制入口执行）、`repair/*`（json repair 监听器挂在 `main.rs`）、`auth/*`（旧版凭据迁移在启动装配阶段触发）。
- 其余场景**一律** library-level：更快、更稳定、mock 控制更精确、不需要编译二进制。
- `startup/001-version` 虽在 wecom-cli 侧，但用 library 方式（`Client::run(["--version"])`）覆盖版本输出格式。

### 1.2 mock 库选择

| 套件 | mock 库 | 说明 |
|---|---|---|
| library-level（`crates/wecom`） | `wiremock` | 统一使用 wiremock |
| process-level（`crates/wecom-cli`） | `mockito` | 同步 server（`setup_sync_discovery_server`），URL 经 env var 注入子进程 |

### 1.3 文件放置

**每个 `test.rs` 放在对应用例的 `desc.md` 同级目录**，两个 crate 各有一套：

```
crates/wecom/test-e2e/              # library-level 套件
  run.rs                            # 编译入口，include! 各 case
  helpers/
    mod.rs                          # 共享导出（re-export 第三方类型 + helpers）
    discovery.rs                    # wiremock 版协议构造 + discovery/method mock
    test_client.rs                  # SharedBuf、build_test_client、断言 helpers
  cases/
    <group>/<NNN>-<slug>/
      desc.md                       # 用例描述
      test.rs                       # 测试代码

crates/wecom-cli/test-e2e/          # process-level 套件
  run.rs                            # 编译入口，include! 各 case
  helpers/
    mod.rs                          # 共享导出
    mock_builders.rs                # mockito 版协议构造（String 返回）
    mock_setup.rs                   # setup_discovery_mocks 等
    test_client.rs                  # SharedBuf、build_test_client
    assertions.rs                   # assert_cli_ok、assert_stdout_contains
    fs_setup.rs                     # setup_config_json
  cases/
    <group>/<NNN>-<slug>/
      desc.md
      test.rs

docs/e2e/                           # 各 crate 共用的 e2e 规范文档
  FRAMEWORK.md                      # 本文档
  CODEGEN.md                        # desc.md → test.rs 生成手册
  DESC_SPEC.md                      # desc.md 格式规范
  DESC_SPEC_HUMAN.md                # desc.md 速写指南
```

后续 crate 如需新增 e2e 套件，按同一模式落地 `<crate>/test-e2e/`（run.rs 入口 + helpers + cases），规范统一遵循 `docs/e2e/`。

**编译入口**：两个 crate 都在各自 `Cargo.toml` 中声明：

```toml
[[test]]
name = "e2e"
path = "test-e2e/run.rs"
```

`run.rs` 按 group 组织 module，每个 module 用 `include!` 引入对应 `test.rs`：

```rust
// crates/wecom/test-e2e/run.rs（节选）
mod helpers;
use helpers::*;

mod run {
    use super::*;
    mod method_call {
        use super::*;
        include!("cases/run/001-method-call/test.rs");
    }
}
```

这样做的好处：

- **test.rs 和 desc.md 紧挨**：修改用例时两个文件在同一目录
- **helpers 集中共享**：所有 test.rs 通过 `use super::*` 访问同一套 helper
- **cargo test 正常工作**：`cargo test -p wecom-core --test e2e` 即可运行整套
- **单用例可定位**：`cargo test -p wecom-core --test e2e run::method_call`
- **新增用例只需三步**：写 desc.md + test.rs，再在 run.rs 加一段 include

## 二、测试框架设计

### 2.1 三个阶段

框架不做 trait 抽象，提供 **helper 函数集合**，每个测试按需组合：

```
┌──────────────────────────────────────────────────────────┐
│  Test Function                                           │
│                                                          │
│  1. Setup Phase                                          │
│     ├── Mock server（wiremock / mockito）                │
│     ├── FsSetup（tempdir + config.json + 待上传文件）    │
│     └── EnvSetup（process-level 经 Command::env 注入）   │
│                                                          │
│  2. Execute Phase                                        │
│     ├── Library: client.run(argv).output(buf).await      │
│     └── Process: assert_cmd::Command::cargo_bin("wecom-cli") │
│                                                          │
│  3. Assert Phase                                         │
│     ├── CLI     → result / exit code / stdout / stderr   │
│     ├── HTTP    → mock 命中次数、请求 body / header      │
│     └── FS      → 文件存在、内容、权限                   │
└──────────────────────────────────────────────────────────┘
```

### 2.2 Helper 模块清单

#### library-level（`crates/wecom/test-e2e/helpers/`）

```rust
// ── discovery.rs：协议构造 + mock 挂载（wiremock）────────

/// `{"result": "<stringified-json>"}` 内层字符串
fn results_json(data: &Value) -> String;

/// 网关扁平响应体：`{ "errcode": 0, "errmsg": "ok", "results_json": <内层> }`
fn api_response(data: &Value) -> Value;

/// 请求封装：`{"payload": "<stringified-json>"}`
fn payload_wrap(data: &Value) -> Value;

/// catalog 响应体（单服务 "hr"）/ hr service 详情响应体
fn catalog_body() -> Value;
fn hr_service_body(service_base_url: &str) -> Value;

/// 标准 argv：`wecom hr department list` + 追加参数
fn hr_dept_list_argv(extra_args: &[&str]) -> Vec<String>;

/// 挂载标准 discovery mocks（catalog + hr service detail）
async fn setup_discovery_mocks(server: &MockServer);

/// 带 header matcher / expect count 的 discovery mocks
async fn setup_discovery_mocks_with(server: &MockServer, opts: DiscoveryMockOptions);

/// 挂载 method 调用端点 mock（可带 header matcher）
async fn setup_method_mock(server: &MockServer, endpoint: &str, response_body: Value);
async fn setup_method_mock_with_headers(server, endpoint, response_body, headers);

// ── test_client.rs：client 构造 + 断言 ───────────────────

/// 可克隆的 Write 实现，捕获输出
struct SharedBuf(Arc<Mutex<Vec<u8>>>);

/// 指向 mock server 的测试 Client（HTTP transport + Bearer test-token）
fn build_test_client(server_url: &str) -> wecom::Client;

/// 构造带指定 token / base_url 的 HTTP transport
fn build_test_http_transport(token: &str, base_url: &str) -> wecom::transport::Transport;

/// leaked tempdir（测试期间不清理）
fn leaked_tempdir() -> PathBuf;

// 断言 helpers
fn assert_cli_ok(result, buf, context);           // 失败时带丰富诊断
fn assert_stdout_json(buf) -> Value;
fn assert_stdout_contains(buf, expected);
fn assert_download_result(buf, content_type) -> Value;
fn assert_file_exists(path) -> String;
fn assert_dir_file_count(dir, count);
fn assert_error_result(result, exit_code, error_code) -> Value;
```

#### process-level（`crates/wecom-cli/test-e2e/helpers/`）

```rust
// ── mock_builders.rs（String 版本协议构造）───────────────
fn results_json(data: &Value) -> String;
fn api_response(data: &Value) -> String;
fn payload_wrap(data: &Value) -> Value;
fn catalog_body() -> String;
fn custom_catalog_body(service_name, description) -> String;
fn hr_service_body(service_base_url: &str) -> String;

// ── mock_setup.rs（mockito）──────────────────────────────
async fn setup_discovery_mocks(server: &mut Server) -> (Mock, Mock);
async fn setup_method_mock(server: &mut Server, path: &str, response_body: &str) -> Mock;
async fn setup_discovery_mocks_with(server, opts: DiscoveryMockOptions) -> (Mock, Mock);
async fn setup_custom_discovery_mocks(server, service_name, description, service_body) -> (Mock, Mock);

/// 同步 discovery server（用于子进程注入），返回 (url, guard)，guard 必须保持存活
#[cfg(feature = "custom-endpoint")]
fn setup_sync_discovery_server() -> (String, ServerGuard);

// ── test_client.rs / assertions.rs / fs_setup.rs ─────────
struct SharedBuf;
fn build_test_client(server_url: &str) -> wecom::Client;
fn leaked_tempdir() -> PathBuf;
fn assert_cli_ok(result, buf, context);
fn assert_stdout_contains(buf, expected);
fn setup_config_json(dir: &Path, config: &Value);
```

### 2.3 Library-level 测试模式

```rust
#[tokio::test]
async fn run() {
    use wiremock::matchers::{method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    // 1. Setup
    let server = MockServer::start().await;
    setup_discovery_mocks(&server).await;
    Mock::given(method("POST"))
        .and(path("/department/list"))
        .respond_with(ResponseTemplate::new(200).set_body_json(api_response(&json!({
            "departments": [{"name": "Engineering"}]
        }))))
        .expect(1)
        .mount(&server)
        .await;

    let buf = SharedBuf::new();
    let client = build_test_client(&server.uri());

    // 2. Execute（输出经 .output() 捕获，不属于 Client）
    let result = client
        .run(hr_dept_list_argv(&[]))
        .output(wecom::CliRunOutput::new(buf.clone()))
        .await;

    // 3. Assert — CLI
    assert_cli_ok(&result, &buf, "method-call");
    let v = assert_stdout_json(&buf);
    assert!(v["departments"].is_array());

    // 4. Assert — HTTP Endpoint 由 wiremock 的 .expect(1) 在 server drop 时校验
}
```

### 2.4 Process-level 测试模式

```rust
// Process-level: logging::init_logging() only runs in main.rs.
#[cfg(feature = "custom-endpoint")]
#[test]
fn run() {
    // 1. Setup — FS + 同步 mock server
    let tmp = tempfile::tempdir().unwrap();
    let (server_url, _keep) = setup_sync_discovery_server();

    // 2. Execute
    let output = assert_cmd::Command::cargo_bin("wecom-cli")
        .unwrap()
        .env("WECOM_CLI_BASE_URL", &server_url)
        .env("WECOM_CLI_CONFIG_DIR", tmp.path())
        .env("WECOM_CLI_LOG_DIR", tmp.path().join("logs"))
        .args(["schema", "list"])
        .output()
        .unwrap();

    // 3. Assert — CLI + FS
    assert!(output.status.success());
    assert!(tmp.path().join("logs").exists());
}

// 未启用 custom-endpoint 时的占位，保证 cargo test 全绿
#[cfg(not(feature = "custom-endpoint"))]
#[test]
#[ignore = "requires custom-endpoint feature"]
fn run() {}
```

### 2.5 Process-level 的特殊约束

| 挑战 | 解法 |
|---|---|
| 子进程如何连接 mock server | 必须启用 `custom-endpoint` feature，通过 `WECOM_CLI_BASE_URL` 注入 mock URL |
| mock server 生命周期 | `setup_sync_discovery_server()` 返回 guard，必须绑定到 `_keep` 存活到子进程结束 |
| 端口冲突 | mockito 自动分配随机端口，每个测试独立 server |
| 并行隔离 | 每个测试独立 tempdir + 独立 mock server + 独立 `WECOM_CLI_CONFIG_DIR` |
| 未启用 feature 时 | 提供 `#[cfg(not(feature = "custom-endpoint"))]` + `#[ignore]` 占位函数 |

## 三、运行方式

```bash
# library-level 全套件（crates/wecom，发布名 wecom-core）
cargo test -p wecom-core --test e2e

# library-level 含 custom-endpoint 用例（client/006）
cargo test -p wecom-core --test e2e --features custom-endpoint

# process-level 全套件（crates/wecom-cli，必须带 feature 才真正执行）
cargo test -p wecom-cli --test e2e --features custom-endpoint

# 运行单个用例
cargo test -p wecom-core --test e2e run::method_call
cargo test -p wecom-cli --test e2e --features custom-endpoint logging::log_file
```

## 四、用例 → 测试代码的映射规则

每个 `desc.md` 的结构化章节可机械地映射为测试代码的各阶段：

| desc.md 章节 | 映射到测试代码 |
|---|---|
| **Transport** | 决定套件归属：HTTP → library-level（wiremock）；仅进程入口行为 → process-level（mockito） |
| **Feature** | `#[cfg(feature = "custom-endpoint")]` gate |
| **前置条件** → mock server | `setup_discovery_mocks()` + `Mock::given(...)` / `setup_method_mock()` |
| **前置条件** → 环境变量 | process-level: `Command::env()`；library-level 不直接 `set_var` |
| **前置条件** → 文件 | `setup_config_json()` / 直接写 tempdir |
| **命令** | library: `client.run(argv)` / process: `Command::cargo_bin("wecom-cli").args([...])` |
| **断言 — CLI** → 退出码 | library: `assert_cli_ok()` / `assert_error_result()`；process: `output.status` |
| **断言 — CLI** → stdout | `assert_stdout_json()` / `assert_stdout_contains()` |
| **断言 — HTTP Endpoint** | wiremock `.expect(N)` / mockito `.expect(N)` + `.assert()`，body/header matcher |
| **断言 — FS** | `assert_file_exists()` / `assert_dir_file_count()` / 直接读 tempdir |

## 五、用例分布现状

### library-level（`crates/wecom/test-e2e/cases/`，51 个用例）

| Group | 用例数 | 覆盖范围 |
|---|---|---|
| `client` | 7 | Client 构建、list/get service、method call、长任务回调、custom-endpoint、端点目录内建默认（ServiceDiscovery） |
| `run` | 22 | argv 驱动的完整流程：method call、help、长任务回调、typo 提示、extra data、json extras（分页/冲突/dry-run）、path alias、无服务回退、`--set`、服务/方法 `--doc`、自定义命令（挂载/遮蔽/错误/help）、10021 渲染 help 等 |
| `schema` | 3 | `schema list`、`schema get`、service `--schema` |
| `pagination` | 4 | `--page-count` 全量拉取、页数超限、页数封顶、带 headers 分页 |
| `output` | 4 | `--output` 文件/目录、二进制下载、tmp 目录 |
| `directive` | 3 | file-save、媒体上传、octet-stream |
| `error` | 3 | 网络错误、非法 JSON body、JSON 修复 |
| `cache` | 2 | discovery 缓存状态、清理 |
| `fs` | 2 | PathResolver（builder / run 两种注入方式） |
| `headers` | 1 | additional headers 透传 |

### process-level（`crates/wecom-cli/test-e2e/cases/`，6 个用例）

| Group | 用例数 | 覆盖范围 |
|---|---|---|
| `startup` | 1 | `--version` 输出格式 |
| `config` | 1 | 非法 config.json → `ConfigError`（893005） |
| `logging` | 2 | stderr 日志、日志文件落盘 |
| `repair` | 1 | json repair 成功后 stderr 输出修复前后 JSON |
| `auth` | 1 | 旧版 `bot.enc` 凭据启动时自动迁移（成功落盘 / 失败保留旧文件，含 2 个子测试） |

## 六、风险与约束

| 风险 | 缓解 |
|---|---|
| process-level 及 custom-endpoint 用例需要 feature | 未启用时编译为 `#[ignore]` 占位；CI 需显式 `--features custom-endpoint` |
| mock 端口冲突 | wiremock/mockito 均自动分配随机端口，每个测试独立 server |
| 环境变量泄漏 | library-level 不直接 `std::env::set_var`；process-level 子进程天然隔离 |
| 长任务轮询拖慢测试 | mock 响应中设 `polling_interval_ms: 1` 加速 |
| 日志文件 flush 时机 | 进程退出后由 OS flush，process-level 在 `.output()` 返回后可安全读取 |
| tempdir 提前 drop | library-level 用 `leaked_tempdir()`；process-level 绑定变量到测试结束 |
