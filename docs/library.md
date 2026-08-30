# Library 接入指南（第三方应用）

面向需要在 Rust 应用中直接集成企业微信机器人能力的第三方调用方：不经过 `wecom-cli` 命令行，而是依赖 Library crate 组合出带鉴权的客户端。

## crate 一览

| crate | 定位 | 给你什么 |
| --- | --- | --- |
| `wecom-auth` | 认证能力 | 凭据存储（`CredentialStore`）、Token Provider（`TokenProvider` / `BotGatewayTokenProvider`）、扫码登录、签名引导 |
| `wecom-runtime` | 认证运行时 | `WecomClient` 门面客户端、`WecomClientBuilder`、鉴权 transport（Bearer 注入 / `RequireAuth` 门禁 / 853004 静默刷新重放）、网关协议端点目录 |
| `wecom-core` | 服务调用 | 动态服务发现、schema 驱动的 clap 命令树、沙箱 FS（一般无需直接依赖；lib 名仍为 `wecom`，代码中 `use wecom::` 不变） |

依赖链：`wecom-runtime` → `wecom-auth` → `wecom-transport`，`wecom-runtime` 同时依赖 `wecom-core`。多数场景**只依赖 `wecom-runtime`** 即可（它转出了 wecom-auth 的全部公共构件）。

## 依赖配置

```toml
[dependencies]
wecom-runtime = "1.2.0"      # Fork 仓库按实际发布方式调整（path/git/registry）
tokio = { version = "1", features = ["full"] }
serde_json = "1"
```

## 最小示例

```rust
use std::sync::Arc;
use std::time::Duration;

use serde_json::json;
use wecom_runtime::{
    BotGatewayTokenProvider, Credentials, MemoryCredentialStore, TokenProvider, WecomClientBuilder,
};

#[tokio::main]
async fn main() -> Result<(), wecom::Error> {
    // botid + secret 来自你的配置体系（环境变量 / Secret Manager）
    let bot_id = std::env::var("WECOM_BOT_ID")?;
    let secret = std::env::var("WECOM_BOT_SECRET")?;

    let store = Arc::new(MemoryCredentialStore::new(Credentials::default()));
    let provider = BotGatewayTokenProvider::new(bot_id, secret, store);

    // 鉴权 + 网关协议 + 动态服务发现，一步到位
    let client = WecomClientBuilder::new()
        .token_provider(Arc::new(provider))
        .timeout(Duration::from_secs(10))
        .build()
        .await?;

    // 服务/方法名以 discovery 下发为准
    for svc in client.list_services().await? {
        println!("service: {}", svc.name);
    }

    let method = client.method(&["hr", "users", "list"]).await?;
    let result = method.invoke(json!({})).await?;
    println!("{}", serde_json::to_string_pretty(&result).unwrap_or_default());
    Ok(())
}
```

完整可编译示例见 `crates/wecom-runtime/examples/`（`service_invoke.rs` 程序化调用、`cli_dispatch.rs` CLI argv 调度）。

## 构建器选项

[`WecomClientBuilder`](../crates/wecom-runtime/src/builder.rs)：

| 方法 | 作用 |
| --- | --- |
| `token_provider(Arc<dyn TokenProvider>)` | token 注入与 853004 静默刷新的来源（必配，除非只想要无鉴权 transport） |
| `base_url(url)` | 覆写网关 base URL（默认 `https://qyapi.weixin.qq.com/cli`） |
| `timeout(Duration)` | 每请求超时 |
| `header(name, value)` | 默认请求头 |
| `initial_token(token)` | 预置初始 token（缺省经 provider 读取） |
| `bin_name(name)` | 错误提示中的命令名（影响 `auth init` 引导文案） |
| `build().await` | 产出 [`WecomClient`]（含网关端点目录，推荐） |
| `build_transport().await` | 仅产出带鉴权的 `Transport`（需完全自定义 `wecom::Client` 配置时用） |

拿到 `WecomClient` 后的三类调用方式：

- **程序化方法调用**：`client.method(&["service", "resource", "method"])` → `invoke(json!({}))`；
- **服务句柄**：`client.service("hr")` → `svc.method(&["users", "list"])`（可先 `svc.schema()` 查看参数 schema）；
- **CLI argv 调度**：`client.run(argv).await`（与 `wecom-cli` 同一套命令模型，适合把 Agent Skill 的命令字符串直接投递）。

## 凭据存储选型

`BotGatewayTokenProvider` 经 `CredentialStore`（`load` / `save` / `clear`）读写 bot 凭据与 Bearer token，按部署环境选择实现：

| 环境 | 建议实现 |
| --- | --- |
| 本地开发 | `EncryptedFileCredentialStore`（`credentials.enc` + keyring，可与 wecom-cli 共享；容器中用 `.with_keyring(false)`） |
| 单实例快速落地 | 环境变量 / 挂载 Secret 注入 botid+secret + `MemoryCredentialStore`（token 进程内存，重启后自动重新引导） |
| 正式部署 | 自实现 `CredentialStore` 对接 KMS / Vault / 云厂商 Secret Manager |
| 多实例部署 | 共享 Secret Store，token 走内存缓存或 Redis 协调 |

自实现要点：`save` 必须整体覆盖写入（bot 与 token 原子更新）；`load` 对「凭据缺失 / 解密失败」返回 `None` 而非报错（与未授权启动表现一致）。

## 鉴权行为（自动处理，无需关心）

- **token 获取**：首次请求前经 provider 读取；`BotGatewayTokenProvider` 用 `sha256_hex(secret + bot_id + time + nonce)` 签名调用 `/cgi-bin/aibot/cli/get_cli_config` 换取 Bearer token；
- **注入**：持有 token 即注入 `Authorization: Bearer <token>`；换取 token 的引导端点自动抑制注入；
- **门禁**：媒体上传、schema 方法等端点挂 `RequireAuth`，无 token 时请求不发出并报 `AuthError`；
- **刷新**：命中 853004（token 失效）时静默换取新 token（并发请求自动合并刷新）并重放原请求一次。

## 错误模型

- `wecom-auth`：`AuthError`（错误码段 893300–893399：`MissingCredentials` 893301 / `QrTimeout` 893302 / `Crypto` 893303 / `Storage` 893304，transport 错误经 `AuthError::Transport` 透传）；
- `wecom` / `wecom-transport`：后台 errcode 原样透传（`wecom::Error::Transport(wecom_transport::Error::Api { code, .. })`），错误码段 893000–893099 / 893100–893199，共享兜底 893999。

## License

仓库采用 [MIT License](../LICENSE)，允许 Fork、修改和商业使用，保留许可证及版权声明即可。
