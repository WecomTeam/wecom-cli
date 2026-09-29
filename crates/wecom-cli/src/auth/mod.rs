//! 鉴权域：Bot 凭据（botid+secret）存储、扫码登录、签名引导换取 Bearer token、
//! 运行时授权会话（853004 静默刷新）。
//!
//! 分层（自底向上）：
//! - [`types`]：数据类型（`Bot` / `Credentials` 凭据总账 /
//!   [`ResolvedAuthorization`](types::ResolvedAuthorization) 授权材料枚举）；
//! - [`crypto`] / [`store`]：加密原语与凭据总账 `credentials.enc` 持久化
//!   （bot + token 共存于单一加密文件，保证原子更新）；
//! - [`resolve`]：授权材料解析（`WECOM_CLI_ACCESS_TOKEN` 优先、文件回退）；
//! - [`bootstrap`] / [`qrcode`]：无状态协议（签名换 token、扫码会话）；
//! - [`session`]：[`AuthSession`] 运行时授权会话——token 内存缓存与
//!   853004 静默刷新编排（供 `transport::WecomBackend` 委托）；
//! - [`legacy_migration`]：旧版凭据（`bot.enc`/`token.enc`）自动迁移——
//!   读旧 botid/secret 自动 auth 换 token 落盘新格式；旧文件**不主动清理**。
//!
//! 目录布局（`~/.config/wecom`）固定不变。

// skip-auth-check 构建不注册 auth 命令（见 cmd/mod.rs），本域仅供该命令
// 使用的项在该构建下无引用点。
#![cfg_attr(feature = "skip-auth-check", allow(dead_code, unused_imports))]

mod bootstrap;
pub(crate) mod crypto;
mod legacy_migration;
mod qrcode;
pub(crate) mod resolve;
mod session;
mod store;
pub(crate) mod types;

/// 装配鉴权引导端点的原语（挂 FlatRes 信封 + SuppressAuth）：生产流程走
/// [`resolve_auth_endpoint`]（含 env/config 解析），本导出仅供测试注入 mock URL。
#[cfg(test)]
pub use bootstrap::auth_endpoint;
pub use bootstrap::{BindSource, fetch_auth, resolve_auth_endpoint};
pub use legacy_migration::try_migrate_legacy_credentials;
pub use qrcode::QrSession;
pub use resolve::resolve_authorization;
pub use session::{AuthSession, RefreshOutcome};
pub use store::{load_credentials, save_credentials};
pub use types::{Bot, ResolvedAuthorization};
