const DEFAULT_MCP_CONFIG_ENDPOINT: &str =
    "https://qyapi.weixin.qq.com/cgi-bin/aibot/cli/get_mcp_config";

pub mod env {
    /// Config directory, defaults to ~/.config/wecom
    pub const CONFIG_DIR: &str = "WECOM_CLI_CONFIG_DIR";

    /// Temp directory, defaults to std::env::temp_dir().join("wecom")
    pub const TMP_DIR: &str = "WECOM_CLI_TMP_DIR";

    /// Log level
    pub const LOG_LEVEL: &str = "WECOM_CLI_LOG_LEVEL";

    /// Log file directory path
    pub const LOG_FILE: &str = "WECOM_CLI_LOG_FILE";

    /// MCP config URL (仅在启用 `custom-endpoint` feature 后使用)
    #[cfg_attr(not(feature = "custom-endpoint"), allow(dead_code))]
    pub const MCP_CONFIG_ENDPOINT: &str = "WECOM_CLI_MCP_CONFIG_ENDPOINT";

    /// Bot ID for non-interactive init (set via env var for CI/CD automation)
    pub const BOT_ID: &str = "WECOM_BOT_ID";

    /// Bot Secret for non-interactive init (set via env var for CI/CD automation)
    pub const BOT_SECRET: &str = "WECOM_SECRET";
}

/// Return the MCP config endpoint URL (env override or the default WeCom API).
pub fn mcp_config_endpoint() -> String {
    #[cfg(feature = "custom-endpoint")]
    if let Ok(url) = std::env::var(env::MCP_CONFIG_ENDPOINT) {
        return url;
    }
    DEFAULT_MCP_CONFIG_ENDPOINT.to_string()
}

pub fn get_user_agent() -> String {
    format!(
        "WeComCLI/{} distribution/{} {}/{}",
        env!("CARGO_PKG_VERSION"),
        option_env!("WECOM_CLI_DISTRIBUTION").unwrap_or_else(|| "unknown"),
        std::env::consts::OS,
        std::env::consts::ARCH,
    )
}
