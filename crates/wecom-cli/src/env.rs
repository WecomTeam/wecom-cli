//! Environment variable name constants for the wecom CLI.

/// 测试专用：串行化所有修改全局环境变量（如 `WECOM_CLI_CONFIG_DIR`）的测试。
///
/// 进程内共享（`crate::env::TEST_ENV_LOCK`），避免不同模块的测试并行改写
/// 同一环境变量而互相串扰（如 `auth::credentials` 与 `transport` 测试）。
#[cfg(test)]
#[cfg_attr(not(feature = "custom-endpoint"), allow(dead_code))]
pub(crate) static TEST_ENV_LOCK: tokio::sync::Mutex<()> = tokio::sync::Mutex::const_new(());

/// 配置目录，默认为 ~/.config/wecom
pub const CONFIG_DIR: &str = "WECOM_CLI_CONFIG_DIR";

/// 临时目录，默认为 std::env::temp_dir().join("wecom")
pub const TMP_DIR: &str = "WECOM_CLI_TMP_DIR";

/// 额外请求头，格式：Record<string, string>
pub const ADDITIONAL_HEADERS: &str = "WECOM_CLI_ADDITIONAL_HEADERS";

/// 访问令牌（Bearer token）：存在时覆盖 `credentials.enc` 中 auth 提供的 access token。
#[cfg(feature = "custom-endpoint")]
pub const ACCESS_TOKEN: &str = "WECOM_CLI_ACCESS_TOKEN";

/// 服务基础 URL（`custom-endpoint` feature 下可用）
#[cfg(feature = "custom-endpoint")]
pub const BASE_URL: &str = "WECOM_CLI_BASE_URL";

/// 鉴权引导端点 URL（`custom-endpoint` feature 下可用）
#[cfg(feature = "custom-endpoint")]
pub const AUTH_ENDPOINT: &str = "WECOM_CLI_AUTH_ENDPOINT";
