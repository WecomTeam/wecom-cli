//! WeCom 认证运行时：鉴权客户端、鉴权 Transport 与客户端构建。
//!
//! 组合 [`wecom-auth`](wecom_auth) 的认证能力与 [`wecom-transport`] 的请求
//! 传输，为第三方应用提供开箱即用的 Library 入口：
//!
//! - [`WecomClient`]：门面客户端——鉴权 + 动态服务发现 + schema 驱动方法
//!   调用一步到位（`service()` / `method()` / `run(argv)`）；
//! - [`WecomClientBuilder`]：构建 [`WecomClient`] 或仅带鉴权的
//!   [`Transport`](wecom_transport::Transport)（token provider、超时、默认头等）；
//! - [`WecomBackend`]：统一出网后端——持有 token 即注入
//!   `Authorization: Bearer <token>`；挂 [`RequireAuth`] 的端点先过门禁；
//!   命中 853004（token 失效）时经 [`TokenProvider`](wecom_auth::TokenProvider)
//!   静默刷新并重放原请求一次（载荷经 `HttpRequestPayload` 工厂克隆，零成本重放）；
//! - [`endpoint_catalog`]：网关扁平协议的端点目录覆写（配合 `wecom::Client` 使用）。
//!
//! # Quick start
//!
//! ```ignore
//! use std::sync::Arc;
//! use std::time::Duration;
//! use wecom_runtime::{BotGatewayTokenProvider, EncryptedFileCredentialStore, WecomClientBuilder};
//!
//! let store = Arc::new(EncryptedFileCredentialStore::new("~/.config/wecom"));
//! let provider = BotGatewayTokenProvider::new("bot-id", "secret", store);
//!
//! let client = WecomClientBuilder::new()
//!     .token_provider(Arc::new(provider))
//!     .timeout(Duration::from_secs(10))
//!     .build()
//!     .await?;
//!
//! // 程序化调用（服务/方法名由 discovery 下发）
//! let svc = client.service("hr").await?;
//! let result = svc.method(&["users", "list"])?.invoke(serde_json::json!({})).await?;
//! ```

pub mod backend;
pub mod builder;
pub mod catalog;
pub mod client;

pub use backend::{TOKEN_EXPIRED_ERRCODE, WecomBackend, is_token_expired, set_bearer_token};
pub use builder::{DEFAULT_BASE_URL, DEFAULT_BIN_NAME, WecomClientBuilder};
pub use catalog::endpoint_catalog;
pub use client::WecomClient;

// 网关协议与鉴权能力标记定义于 wecom-auth，此处转出便于下游单依赖使用。
pub use wecom_auth::{
    AccessToken, AuthError, BotCredential, BotGatewayTokenProvider, CredentialStore, Credentials,
    EncryptedFileCredentialStore, FlatRes, MemoryCredentialStore, NestedRes, RequireAuth,
    SuppressAuth, TokenProvider, auth_endpoint, gateway::FlatApiResponse,
};
