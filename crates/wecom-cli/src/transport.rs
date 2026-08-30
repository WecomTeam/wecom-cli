//! Transport construction for the wecom CLI.
//!
//! CLI 侧组装：解析 base URL、装配 [`wecom_auth::EncryptedFileCredentialStore`]
//! 与 [`wecom_auth::BotGatewayTokenProvider`]，最终经
//! [`wecom_runtime::WecomBackend`] 装饰出带鉴权的 transport：
//! - **authorization**：持有 token 即注入 `Authorization: Bearer <token>`
//!   （无 token 则忽略）；挂 [`wecom_runtime::RequireAuth`] 的端点先过
//!   门禁——无可用 token 直接报错且请求不发出。换取 token 的引导端点挂
//!   [`wecom_runtime::SuppressAuth`] 抑制注入。
//! - **token refresh**：命中 853004 时经 botid+signature 静默换 token、落盘
//!   并重试一次。
//!
//! 网关扁平协议（`{errcode, errmsg, results_json}`）与端点目录覆写位于
//! wecom-runtime（[`endpoint_catalog`]）。

use std::sync::Arc;

use wecom_auth::{BotGatewayTokenProvider, CredentialStore as _, EncryptedFileCredentialStore};
use wecom_runtime::{DEFAULT_BASE_URL, WecomBackend};
use wecom_transport::Transport;

use crate::Result;
use crate::auth;
use crate::config::{self, ConfigFile};
#[cfg(feature = "custom-endpoint")]
use crate::env;

pub use wecom_runtime::endpoint_catalog;

/// 解析运行时使用的 Bearer token：`custom-endpoint` feature 下优先
/// `WECOM_CLI_ACCESS_TOKEN` 环境变量，缺省回退 `credentials.enc` 中 auth 提供的 access token。
fn resolve_access_token(store: &EncryptedFileCredentialStore) -> Option<String> {
    // 仅 `custom-endpoint` feature 下读取环境变量覆盖，否则回退 auth token。
    #[cfg(feature = "custom-endpoint")]
    let env_token = std::env::var(env::ACCESS_TOKEN)
        .ok()
        .filter(|t| !t.is_empty());
    #[cfg(not(feature = "custom-endpoint"))]
    let env_token: Option<String> = None;

    env_token.or_else(|| store.load().ok().flatten().and_then(|creds| creds.token))
}

/// Build a fully-configured HTTP transport.
///
/// Bearer token 来源为 `credentials.enc`（botid+secret 签名引导获取，见
/// [`wecom_auth`]）。无 token 时不报错：`Authorization` 头由
/// [`WecomBackend`] 在调用时持有 token 即注入（无 token 则忽略）；挂
/// `RequireAuth` 的端点先过门禁，无 token 时报错。
///
/// 最终 transport 都装饰为 [`WecomBackend`]：
/// - 持有 token 即注入 `Authorization` 头；挂 `RequireAuth` 的端点
///   无 token 报错（门禁）；
/// - 网关扁平协议响应整体 body 即结果（`NestedRes` endpoint envelope 驱动）；
/// - 返回 853004（token 失效）时自动重新换取 token、落盘并重试一次。
pub async fn build(cfg: &ConfigFile) -> Result<Transport> {
    // base_url 解析：custom-endpoint feature 下 env/config 优先，缺省回退默认网关 URL。
    #[cfg(feature = "custom-endpoint")]
    let base_url = config::env_or_config(env::BASE_URL, cfg.base_url.as_deref())
        .unwrap_or_else(|| DEFAULT_BASE_URL.to_string());
    #[cfg(not(feature = "custom-endpoint"))]
    let base_url = DEFAULT_BASE_URL.to_string();

    let builder = wecom_transport::HttpTransportBackend::builder().base_url(base_url);
    let builder = config::apply_transport_config(builder, cfg)?;
    let transport = builder.build()?;

    // 凭据存储（`credentials.enc` 所在目录）与鉴权引导端点各装配一次：
    // 同一实例共享给旧版凭据迁移与 token provider（853004 刷新复用）。
    let store = Arc::new(EncryptedFileCredentialStore::new(config::default_home_dir()));
    let auth_endpoint = auth::resolve_auth_endpoint(Some(cfg));

    // 旧版凭据（bot.enc/token.enc）自动迁移：无 credentials.enc 时读取旧
    // botid/secret 自动走 auth 引导换取 token 并落盘；失败静默降级
    // （不阻塞启动、不清理旧文件，见 wecom_auth::legacy_migration）。
    auth::try_migrate_legacy_credentials(&store, &transport, &auth_endpoint).await?;

    // 初始 token 与 bot 凭据均来自 credentials.enc（`auth init` 时持久化）。
    // 不烘焙为默认头——由 WecomBackend 在调用时按端点能力动态注入。
    // `WECOM_CLI_ACCESS_TOKEN` 存在时覆盖 auth 提供的 token。
    let init_token = resolve_access_token(&store);

    let provider = BotGatewayTokenProvider::from_store(store).with_auth_endpoint(auth_endpoint);

    Ok(transport.wrap_backend(move |backend| {
        Arc::new(
            WecomBackend::new(backend, Some(Arc::new(provider)), init_token)
                .with_bin_name(env!("CARGO_BIN_NAME")),
        )
    }))
}

#[cfg(test)]
mod tests {
    //! ## 模块摘要：transport（CLI transport 组装）
    //!
    //! ### 关键接口
    //! - [resolve_access_token] — `WECOM_CLI_ACCESS_TOKEN` 覆盖 / 回退凭据存储 token
    //!
    //! ### 关键分支与异常路径
    //! - 环境变量非空 → 覆盖；空环境变量 → 走回退路径

    #[cfg(feature = "custom-endpoint")]
    use crate::env::TEST_ENV_LOCK;

    #[cfg_attr(not(feature = "custom-endpoint"), allow(unused_imports))]
    use super::*;

    /// P0：[resolve_access_token] WECOM_CLI_ACCESS_TOKEN 覆盖 auth 提供的 token
    /// 条件：设置 WECOM_CLI_ACCESS_TOKEN=env-tok，隔离凭据目录（凭据存储为空）
    /// 断言：resolve_access_token() == Some("env-tok")
    #[cfg(feature = "custom-endpoint")]
    #[tokio::test]
    async fn access_token_env_overrides_auth_token() {
        let _guard = TEST_ENV_LOCK.lock().await;
        let dir = tempfile::tempdir().unwrap();
        unsafe {
            std::env::set_var(crate::env::CONFIG_DIR, dir.path());
            std::env::set_var(crate::env::ACCESS_TOKEN, "env-tok");
        }
        let store = EncryptedFileCredentialStore::new(config::default_home_dir());
        let r = resolve_access_token(&store);
        unsafe {
            std::env::remove_var(crate::env::ACCESS_TOKEN);
            std::env::remove_var(crate::env::CONFIG_DIR);
        }
        assert_eq!(r.as_deref(), Some("env-tok"));
    }

    /// P1：[resolve_access_token] 环境变量为空时回退 auth 提供的 token
    /// 条件：WECOM_CLI_ACCESS_TOKEN=""，隔离凭据目录（无凭据 → load 返回 None）
    /// 断言：resolve_access_token() == None（空环境变量不生效，走回退路径）
    #[cfg(feature = "custom-endpoint")]
    #[tokio::test]
    async fn access_token_env_empty_falls_back_to_auth() {
        let _guard = TEST_ENV_LOCK.lock().await;
        let dir = tempfile::tempdir().unwrap();
        unsafe {
            std::env::set_var(crate::env::CONFIG_DIR, dir.path());
            std::env::set_var(crate::env::ACCESS_TOKEN, "");
        }
        let store = EncryptedFileCredentialStore::new(config::default_home_dir());
        let r = resolve_access_token(&store);
        unsafe {
            std::env::remove_var(crate::env::ACCESS_TOKEN);
            std::env::remove_var(crate::env::CONFIG_DIR);
        }
        assert_eq!(r, None);
    }
}
