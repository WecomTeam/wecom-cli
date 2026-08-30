# 根据 desc.md 生成 e2e 测试代码 — 操作手册

## 概述

本文档是一份 **操作手册**，指导 AI 或开发者根据 `test-e2e/cases/<group>/<NNN>-<slug>/desc.md` 的结构化描述，生成 `test.rs` 文件，放在 **desc.md 的同级目录**。

项目通过 discovery 协议 + HTTP 传输（网关扁平协议）调用远程服务。测试分两套件：

- **library-level**：`crates/wecom/test-e2e/`，构造 `wecom::Client` 直接驱动，mock 用 **wiremock**
- **process-level**：`crates/wecom-cli/test-e2e/`，`assert_cmd` 启动真实二进制，mock 用 **mockito**

## 输入

一份 `desc.md` 文件，包含以下结构化章节（格式见 `DESC_SPEC.md`）：

```
- 场景 / Transport / 来源（/ Feature）
- 测试等级（可选）
- 前置条件（mock、env、file）
- 命令
- 断言 — CLI（exit code、stdout、stderr）
- 断言 — HTTP Endpoint（请求验证）
- 断言 — FS（文件系统变化）
- 关键上下文（源码路径）
```

## 输出

一个 `test.rs` 文件，放在 `desc.md` 同级目录。文件内容是一个测试函数（`#[tokio::test]` 或 `#[test]`）。

```
test-e2e/cases/<group>/<NNN>-<slug>/
  desc.md      # 输入：用例描述
  test.rs      # 输出：测试代码
```

test.rs 通过所在 crate 的 `test-e2e/run.rs` 中的 `include!` 宏纳入编译，内部可直接使用 `use super::*` 访问 helpers。

---

## 步骤 1：判断测试层级与套件

读取 desc.md 的 **场景** 和 **前置条件**，按以下规则判断：

| 条件 | 层级 / 套件 | 说明 |
|---|---|---|
| 涉及 `main.rs` 入口错误处理（config 解析失败时 `--version` 也失败） | **process-level**（wecom-cli） | 需要真实二进制 + `custom-endpoint` feature |
| 涉及 `logging::init_logging()` / 日志文件 | **process-level**（wecom-cli） | 同上 |
| 涉及 json repair 的 stderr 提示（监听器挂在 `main.rs`） | **process-level**（wecom-cli） | 同上 |
| 涉及旧版凭据（`bot.enc`）启动时自动迁移 | **process-level**（wecom-cli） | 同上（迁移在启动装配阶段触发） |
| 其余所有 | **library-level**（wecom） | 构造 `wecom::Client`，调用 `Client::run(argv)` |

两种层级的 test.rs 写法不同，但都放在 desc.md 同级目录。

process-level 的 test.rs 需要 feature gate + 占位：

```rust
// test.rs (process-level)
#[cfg(feature = "custom-endpoint")]
#[test]
fn run() {
    // ...
}

#[cfg(not(feature = "custom-endpoint"))]
#[test]
#[ignore = "requires custom-endpoint feature"]
fn run() {}
```

library-level 的 test.rs 不需要 feature gate（除非 desc.md 标注了 **Feature**）：

```rust
// test.rs (library-level)
#[tokio::test]
async fn run() {
    // ...
}
```

## 步骤 2：生成函数签名

### 函数命名

每个 test.rs 独占一个 module（通过 `run.rs` 的 `include!` 引入），函数名统一用 `run`。模块路径已经包含了用例信息：

```
cargo test -p wecom-core --test e2e run::method_call
cargo test -p wecom-cli --test e2e --features custom-endpoint logging::log_file
```

### Library-level

```rust
#[tokio::test]
async fn run() {
    // ...
}
```

### Process-level

```rust
#[cfg(feature = "custom-endpoint")]
#[test]
fn run() {
    // ...
}
```

## 步骤 3：生成 Setup 阶段

按 desc.md 的 **前置条件** 逐项翻译。

### 3a. Mock server — discovery（library-level，wiremock）

如果前置条件提到 "mock server 返回 catalog + service detail"：

```rust
let server = MockServer::start().await;
setup_discovery_mocks(&server).await;
```

### 3b. Mock server — method call（library-level，wiremock）

```rust
Mock::given(method("POST"))
    .and(path("/department/list"))
    .respond_with(ResponseTemplate::new(200).set_body_json(api_response(&json!({
        "departments": [{"id": "1"}]
    }))))
    .expect(1)
    .mount(&server)
    .await;
```

需要匹配请求 body（网关扁平协议，payload 为字符串化 JSON）：

```rust
Mock::given(method("POST"))
    .and(path("/department/list"))
    .and(body_json(payload_wrap(&json!({"id": "root"}))))
    // ...
```

需要匹配 Authorization header：`.and(header("authorization", "Bearer test-token"))`。

### 3c. Mock server — 同步 server（process-level，mockito）

```rust
let (server_url, _keep) = setup_sync_discovery_server();
// _keep 必须存活到子进程结束
```

### 3d. Mock server — long-task

method mock 返回含 `taskid` 的响应，再为轮询端点按序挂载多个响应（最后一次 `done: true`）；mock 中设 `polling_interval_ms: 1` 加速测试。轮询协议见 `wecom-transport/src/http/polling.rs`。

### 3e. FS setup

```rust
let tmp = tempfile::tempdir().unwrap();
```

如果需要 `config.json`：

```rust
setup_config_json(tmp.path(), &json!({"tmp_dir": "/tmp/wecom-custom"}));
```

### 3f. Test client（library-level）

```rust
let buf = SharedBuf::new();
let client = build_test_client(&server.uri());
```

输出不属于 Client，在执行时经 `.output()` 传入（见步骤 4）。

### 3g. Process command（process-level）

```rust
let output = assert_cmd::Command::cargo_bin("wecom-cli")
    .unwrap()
    .env("WECOM_CLI_BASE_URL", &server_url)
    .env("WECOM_CLI_CONFIG_DIR", tmp.path())
    .args(["schema", "list"])
    .output()
    .unwrap();
```

## 步骤 4：生成 Execute 阶段

### Library-level

从 desc.md 的 **命令** 提取 argv（`hr_dept_list_argv` 可生成标准 argv）：

```rust
let result = client
    .run(hr_dept_list_argv(&["--dry-run"]))
    .output(wecom::CliRunOutput::new(buf.clone()))
    .await;
```

### Process-level

见 3g，`.output()` 返回 `std::process::Output`。

## 步骤 5：生成 Assert 阶段

按 desc.md 的三个断言章节逐项翻译。

### 5a. 断言 — CLI

| desc.md 描述 | Library-level 代码 | Process-level 代码 |
|---|---|---|
| 退出码 `0` | `assert_cli_ok(&result, &buf, "<case>");` | `assert!(output.status.success());` |
| 退出码 `1` + 错误码 | `assert_error_result(&result, 1, 893002);` | `assert_eq!(output.status.code(), Some(1));` + 解析 stdout JSON |
| stdout 包含 `"wecom"` | `assert_stdout_contains(&buf, "wecom");` | `String::from_utf8_lossy(&output.stdout).contains("wecom")` |
| stdout 是合法 JSON | `let v = assert_stdout_json(&buf);` | 手动 `serde_json::from_str` |
| stdout 是 DownloadResult | `let v = assert_download_result(&buf, "application/json");` | （手动解析） |
| JSON 字段相等 | `assert_json_eq!(v["departments"], json!([...]));` | （手动断言） |

### 5b. 断言 — HTTP Endpoint

| desc.md 描述 | 代码 |
|---|---|
| mock 被调用 N 次 | wiremock：创建时 `.expect(N)`（server drop 时校验）；mockito：`.expect(N)` + `.assert()` |
| mock 未被调用 | `.expect(0)` |
| 请求 header 包含 `Authorization: Bearer xxx` | wiremock `.and(header(...))` / mockito `.match_header(...)` |
| 请求 body 匹配 | wiremock `.and(body_json(payload_wrap(...)))` / mockito `.match_body(Matcher::Json(...))` |
| 多步骤交互序列 | 按序挂载多个 mock，各 `.expect(1)` |

### 5c. 断言 — FS

| desc.md 描述 | 代码 |
|---|---|
| 文件存在 | `assert_file_exists(&path);` |
| 目录下有 N 个文件 | `assert_dir_file_count(&dir, N);` |
| 日志文件存在 | 读目录，断言文件名 `starts_with("ww.log.")` |
| 无文件写入 | （不断言，或 `assert_dir_file_count(&tmp, 0)` 排除） |

## 步骤 6：组装完整测试函数

将步骤 2-5 的代码片段按 Setup → Execute → Assert 顺序组装，写入 desc.md 同级的 `test.rs`。

### 完整示例：library-level（`run/022-set-basic/test.rs`）

```rust
#[tokio::test]
async fn run() {
    use wiremock::matchers::{method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    let server = MockServer::start().await;
    setup_discovery_mocks(&server).await;

    Mock::given(method("POST"))
        .and(path("/department/list"))
        .respond_with(
            ResponseTemplate::new(200).set_body_json(api_response(&json!({
                "departments": [{"name": "Engineering"}]
            }))),
        )
        .expect(1)
        .mount(&server)
        .await;

    let buf = SharedBuf::new();
    let client = build_test_client(&server.uri());

    let result = client
        .run(hr_dept_list_argv(&["--set", "extra_field=hello"]))
        .output(wecom::CliRunOutput::new(buf.clone()))
        .await;
    assert_cli_ok(&result, &buf, "set-basic");

    let v = assert_stdout_json(&buf);
    assert_json_diff::assert_json_eq!(v["departments"], json!([{"name": "Engineering"}]));
}
```

### 完整示例：process-level（`config/004-invalid-config-json/test.rs`）

```rust
#[cfg(feature = "custom-endpoint")]
#[test]
fn run() {
    let tmp = tempfile::tempdir().unwrap();
    std::fs::write(tmp.path().join("config.json"), "{invalid json!!!").unwrap();

    let output = assert_cmd::Command::cargo_bin("wecom-cli")
        .unwrap()
        .env("WECOM_CLI_CONFIG_DIR", tmp.path().as_os_str())
        .env("WECOM_CLI_BASE_URL", "http://127.0.0.1:1")
        .args(["--version"])
        .output()
        .unwrap();

    assert_eq!(output.status.code(), Some(1));
    let stdout = String::from_utf8_lossy(&output.stdout);
    let v: Value = serde_json::from_str(stdout.trim()).unwrap();
    assert_eq!(v["error"]["type"], "ConfigError");
    assert_eq!(v["error"]["code"], 893005);
}
```

### `run.rs` 入口文件中对应的 include

```rust
// crates/wecom/test-e2e/run.rs（节选）
mod run {
    use super::*;
    mod set_basic {
        use super::*;
        include!("cases/run/022-set-basic/test.rs");
    }
}
```

---

## 翻译检查清单

生成每个 test.rs 后，按以下清单逐项检查：

- [ ] test.rs 放在 desc.md 同级目录
- [ ] 函数名为 `run`（模块路径已包含用例信息）
- [ ] 所在 crate 的 `run.rs` 入口中已添加对应的 `include!` 行
- [ ] 测试层级（library/process）与套件（wecom / wecom-cli）和步骤 1 判断一致
- [ ] process-level 的 test.rs 有 `#[cfg(feature = "custom-endpoint")]` + `#[ignore]` 占位
- [ ] 所有 desc.md 中提到的 mock endpoint 都已 setup
- [ ] 所有 desc.md 中提到的 CLI 断言都已编码
- [ ] 所有 desc.md 中提到的 HTTP 断言都已编码（特别是 `.expect(N)`）
- [ ] 所有 desc.md 中提到的 FS 断言都已编码
- [ ] tempdir 的生命周期覆盖整个测试（library-level 用 `leaked_tempdir()`）
- [ ] mock server 的生命周期覆盖整个测试（process-level 特别注意 `_keep` guard）
- [ ] 无环境变量泄漏到其他测试（library-level 不直接 `set_var`）
