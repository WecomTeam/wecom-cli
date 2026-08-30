//! 鉴权门面：wecom-auth 认证能力 re-export + CLI 特有的引导端点解析。
//!
//! 凭据与 token 的存储、签名引导、扫码登录网络流程均已提炼到
//! [`wecom-auth`](wecom_auth) 库；本模块仅为 CLI 组装入口：
//! - re-export 认证构件（bot 凭据、凭据存储、扫码会话等）；
//! - 按 `WECOM_CLI_AUTH_ENDPOINT` env / `config.json` 解析鉴权引导端点
//!   （`custom-endpoint` feature 下可覆盖，见 [`resolve_auth_endpoint`]）。

use crate::config::ConfigFile;
#[cfg(feature = "custom-endpoint")]
use crate::env;

pub use wecom_auth::{
    BindSource, BotCredential as Bot, CredentialStore, EncryptedFileCredentialStore, QrSession,
    fetch_auth, try_migrate_legacy_credentials,
};

/// 解析并装配鉴权引导端点：`custom-endpoint` feature 下按
/// `WECOM_CLI_AUTH_ENDPOINT` env > `config.json` 的 `auth_endpoint` > 默认
/// 解析 URL，再经 [`wecom_auth::auth_endpoint`] 装配（扁平信封 + 抑制注入）。
///
/// `cfg` 为调用方已加载（或经 Client 扩展袋注入）的配置，可缺省：
/// `None`（未注入）时直接回退默认端点，不报错。
#[cfg_attr(not(feature = "custom-endpoint"), allow(unused_variables))]
pub fn resolve_auth_endpoint(cfg: Option<&ConfigFile>) -> wecom_transport::Endpoint {
    #[cfg(feature = "custom-endpoint")]
    let resolved = crate::config::env_or_config(
        env::AUTH_ENDPOINT,
        cfg.and_then(|c| c.auth_endpoint.as_deref()),
    )
    .unwrap_or_else(|| wecom_auth::DEFAULT_AUTH_ENDPOINT.to_string());
    #[cfg(not(feature = "custom-endpoint"))]
    let resolved = wecom_auth::DEFAULT_AUTH_ENDPOINT.to_string();
    wecom_auth::auth_endpoint(&resolved)
}

#[cfg(test)]
mod tests {
    //! ## 模块摘要：auth（CLI 鉴权门面）
    //!
    //! ### 关键接口
    //! - [resolve_auth_endpoint] — 鉴权端点解析（基于已加载的 ConfigFile，
    //!   `custom-endpoint` feature 下 env > config.json > 默认）
    //!
    //! ### 关键分支与异常路径
    //! - 默认端点指向 product/正式环境
    //! - cfg 未注入（None）时回退默认端点，不报错
    //! - `custom-endpoint` feature 下环境变量可覆盖完整 URL

    use std::sync::Mutex;

    use wecom_transport::EndpointHttpExt;

    use crate::config::ConfigFile;

    use super::*;

    // 串行化环境变量测试（WECOM_CLI_AUTH_ENDPOINT 为全局）。
    static ENV_LOCK: Mutex<()> = Mutex::new(());

    fn with_env<T>(key: &str, value: Option<&str>, f: impl FnOnce() -> T) -> T {
        let _guard = ENV_LOCK.lock().unwrap();
        // 测试专用：设置/清理全局环境变量（Rust 2024 下为 unsafe）。
        unsafe {
            match value {
                Some(v) => std::env::set_var(key, v),
                None => std::env::remove_var(key),
            }
        }
        let r = f();
        unsafe {
            std::env::remove_var(key);
        }
        r
    }

    /// P0：默认端点指向 product/正式环境的新接口
    /// 条件：未设置 WECOM_CLI_AUTH_ENDPOINT（默认 feature 下 env/config 均不生效）
    /// 断言：resolve_auth_endpoint() 返回默认 product 端点
    #[test]
    fn auth_endpoint_defaults_to_product() {
        with_env("WECOM_CLI_AUTH_ENDPOINT", None, || {
            assert_eq!(
                resolve_auth_endpoint(Some(&ConfigFile::default())).full_url(),
                "https://qyapi.weixin.qq.com/cgi-bin/aibot/cli/get_cli_config"
            );
        });
    }

    /// P1：cfg 未注入（None）时回退默认端点，不报错
    /// 条件：resolve_auth_endpoint(None)
    /// 断言：返回默认 product 端点
    #[test]
    fn auth_endpoint_none_falls_back_to_default() {
        with_env("WECOM_CLI_AUTH_ENDPOINT", None, || {
            assert_eq!(
                resolve_auth_endpoint(None).full_url(),
                "https://qyapi.weixin.qq.com/cgi-bin/aibot/cli/get_cli_config"
            );
        });
    }

    /// P1：环境变量覆盖完整 URL（`custom-endpoint` feature 下生效，优先级最高）
    /// 条件：设置 WECOM_CLI_AUTH_ENDPOINT 为测试端点
    /// 断言：resolve_auth_endpoint() 返回环境变量指定的 URL
    #[cfg(feature = "custom-endpoint")]
    #[test]
    fn auth_endpoint_env_override() {
        with_env(
            "WECOM_CLI_AUTH_ENDPOINT",
            Some("https://example.com/cgi-bin/aibot/cli/get_cli_config"),
            || {
                assert_eq!(
                    resolve_auth_endpoint(Some(&ConfigFile::default())).full_url(),
                    "https://example.com/cgi-bin/aibot/cli/get_cli_config"
                );
            },
        );
    }

    /// P1：config.json 的 auth_endpoint 覆盖默认值（`custom-endpoint` feature 下生效）
    /// 条件：ConfigFile 含 auth_endpoint，env 未设置
    /// 断言：resolve_auth_endpoint() 返回 config 中的 URL
    #[cfg(feature = "custom-endpoint")]
    #[test]
    fn auth_endpoint_config_override() {
        with_env("WECOM_CLI_AUTH_ENDPOINT", None, || {
            let cfg = ConfigFile {
                auth_endpoint: Some("https://config.example.com/auth".to_string()),
                ..Default::default()
            };
            assert_eq!(
                resolve_auth_endpoint(Some(&cfg)).full_url(),
                "https://config.example.com/auth"
            );
        });
    }
}
