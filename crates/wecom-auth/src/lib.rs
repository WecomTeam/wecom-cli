//! WeCom 认证能力库：凭证存储、Token Provider 与 AI Bot CLI 网关协议。
//!
//! 与运行方式（CLI / 服务端 / Agent）无关的认证构件：
//!
//! - [`bot`]：Bot 凭据（`botid + secret`）；
//! - [`credentials`]：[`CredentialStore`](credentials::CredentialStore) 抽象与
//!   [`MemoryCredentialStore`](credentials::MemoryCredentialStore) 内存实现；
//! - [`file_store`]：[`EncryptedFileCredentialStore`](file_store::EncryptedFileCredentialStore)
//!   （AES-256-GCM 加密文件 + keyring 回退）；
//! - [`provider`]：[`TokenProvider`](provider::TokenProvider) 抽象与
//!   [`BotGatewayTokenProvider`](provider::BotGatewayTokenProvider)
//!   （botid+secret 签名引导换取 Bearer token，含并发刷新合并）；
//! - [`bootstrap`]：`sha256_hex(secret + bot_id + time + nonce)` 签名与引导调用；
//! - [`gateway`]：AI Bot CLI 网关协议（扁平响应信封 + 鉴权能力标记 + 引导端点装配）；
//! - [`qrcode`]：扫码登录的网络流程（创建会话 → 轮询结果）；
//! - [`legacy_migration`]：旧版凭据（`bot.enc`/`token.enc`）自动迁移；
//! - [`error`]：统一错误 [`AuthError`](error::AuthError)（错误码段 893300–893399）。

pub mod bootstrap;
pub mod bot;
pub mod credentials;
pub mod crypto;
pub mod error;
pub mod file_store;
pub mod gateway;
pub mod legacy_migration;
pub mod provider;
pub mod qrcode;

pub use bootstrap::{BindSource, FetchAuthRequest, FetchAuthResponse, fetch_auth, sign};
pub use bot::BotCredential;
pub use credentials::{CredentialStore, Credentials, MemoryCredentialStore};
pub use error::AuthError;
pub use file_store::EncryptedFileCredentialStore;
pub use gateway::{
    DEFAULT_AUTH_ENDPOINT, FlatRes, NestedRes, RequireAuth, SuppressAuth, auth_endpoint,
};
pub use legacy_migration::try_migrate_legacy_credentials;
pub use provider::{AccessToken, BotGatewayTokenProvider, TokenProvider};
pub use qrcode::QrSession;
