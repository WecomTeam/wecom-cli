use std::fs;
use std::time::{SystemTime, UNIX_EPOCH};

use anyhow::Result;
use serde::{Deserialize, Serialize};
use serde_repr::Serialize_repr;
use sha2::{Digest, Sha256};

use crate::crypto;
use crate::{auth, fs_util};
use crate::{constants, paths};

// ---------------------------------------------------------------------------
// Request
// ---------------------------------------------------------------------------

/// 配置来源
#[derive(Debug, Clone, Copy, Serialize_repr)]
#[repr(u8)]
pub enum McpBindSource {
    /// Interactive
    Interactive = 1,
    /// QR Code
    Qrcode = 2,
    /// Environment variable (WECOM_BOT_ID + WECOM_SECRET)
    Env = 3,
}

#[derive(Debug, Clone, Serialize)]
pub struct GetMcpConfigRequest {
    pub bot_id: String,
    pub time: u64,
    pub nonce: String,
    pub signature: String,
    pub bind_source: McpBindSource,
    pub cli_version: String,
}

impl GetMcpConfigRequest {
    /// Build a signed request from stored bot credentials
    pub fn build(bind_source: McpBindSource) -> Result<Self> {
        let bot = auth::get_bot_info().ok_or_else(|| {
            anyhow::anyhow!(
                "未找到企业微信机器人信息，请先运行 `{} init`",
                env!("CARGO_BIN_NAME")
            )
        })?;

        let time = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        let nonce = super::gen_req_id("mcp");
        let signature = sign(&bot.secret, &bot.id, time, &nonce);

        let cli_version = constants::get_user_agent();

        Ok(Self {
            bot_id: bot.id,
            time,
            nonce,
            signature,
            bind_source,
            cli_version,
        })
    }
}

// ---------------------------------------------------------------------------
// Response
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Deserialize)]
pub struct GetMcpConfigResponse {
    #[serde(default)]
    pub errcode: i32,
    pub errmsg: Option<String>,
    #[serde(default)]
    pub list: Option<Vec<McpConfigItem>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpConfigItem {
    pub url: Option<String>,
    #[serde(rename = "type")]
    pub transport_type: Option<String>,
    pub is_authed: Option<bool>,
    pub biz_type: Option<String>,
}

use super::error::{FetchMcpConfigError, GetMcpConfigHttpError};

// ---------------------------------------------------------------------------
// Signature
// ---------------------------------------------------------------------------

/// Compute the request signature.
///
/// Algorithm: `sha256_hex(secret + bot_id + time + nonce)`
/// where `sha256_hex` uses the standard zero-padded lowercase hex format (`%02x`).
pub fn sign(secret: &str, bot_id: &str, time: u64, nonce: &str) -> String {
    let input = format!("{secret}{bot_id}{time}{nonce}");
    sha256_hex(&input)
}

/// Compute the SHA-256 hash of `input` and return it as a lowercase hex string.
fn sha256_hex(input: &str) -> String {
    let hash = Sha256::digest(input.as_bytes());
    let mut result = String::with_capacity(64);
    for byte in hash.iter() {
        result.push_str(&format!("{:02x}", byte));
    }
    result
}

// ---------------------------------------------------------------------------
// Persistence
// ---------------------------------------------------------------------------

/// Return the file path for the encrypted MCP config cache.
fn mcp_config_path() -> std::path::PathBuf {
    paths::wecom_home_dir().join("mcp_config.enc")
}

/// Read cached MCP config list from the encrypted file.
pub fn load_mcp_config() -> Option<Vec<McpConfigItem>> {
    let data = fs::read(mcp_config_path()).ok()?;
    crypto::try_decrypt_data(&data).ok()
}

/// Encrypt and persist the MCP config list to disk.
pub fn save_mcp_config(items: &[McpConfigItem]) -> Result<()> {
    let key = crypto::load_existing_key().unwrap_or_else(|| {
        let k = crypto::generate_random_key();
        tracing::info!("Generated new encryption key for MCP config");
        k
    });

    crypto::save_key(&key)?;

    let encrypted = crypto::encrypt_data(items, &key)?;
    let path = mcp_config_path();
    fs_util::atomic_write(&path, &encrypted, Some(0o600))?;

    tracing::info!("MCP config saved to {}", path.display());
    Ok(())
}

/// Remove the cached MCP config file from disk.
pub fn clear_mcp_config() {
    let path = mcp_config_path();
    if path.exists() {
        let _ = fs::remove_file(&path);
        tracing::info!("MCP config cache removed: {}", path.display());
    }
}

#[cfg(test)]
/// Encrypt and write the config list to a specific path with a given key (test helper).
fn save_mcp_config_to_path(
    items: &[McpConfigItem],
    path: &std::path::Path,
    key: &[u8; 32],
) -> Result<()> {
    let encrypted = crypto::encrypt_data(items, key)?;
    fs_util::atomic_write(path, &encrypted, Some(0o600))
}

#[cfg(test)]
/// Read and decrypt the config list from a specific path with a given key (test helper).
fn load_mcp_config_from_path(path: &std::path::Path, key: &[u8; 32]) -> Option<Vec<McpConfigItem>> {
    let data = fs::read(path).ok()?;
    crypto::decrypt_data(&data, key).ok()
}

// ---------------------------------------------------------------------------
// API Call
// ---------------------------------------------------------------------------

/// Always fetch the MCP config from the server, bypassing local cache, and persist the result.
pub async fn fetch_mcp_config(
    bind_source: McpBindSource,
) -> Result<GetMcpConfigResponse, FetchMcpConfigError> {
    let request = GetMcpConfigRequest::build(bind_source)?;

    let response = reqwest::Client::builder()
        .build()
        .map_err(|e| FetchMcpConfigError::Other(e.into()))?
        .post(constants::mcp_config_endpoint())
        .header("User-Agent", constants::get_user_agent())
        .json(&request)
        .send()
        .await
        .map_err(|e| FetchMcpConfigError::Other(e.into()))?;

    let status = response.status();
    if !status.is_success() {
        let mut body = response
            .text()
            .await
            .unwrap_or_else(|_| "<Failed to read response body>".to_string());
        if body.is_empty() {
            body = "<Empty response body>".to_string();
        }
        return Err(FetchMcpConfigError::Http(GetMcpConfigHttpError {
            status: status.as_u16(),
            body,
        }));
    }

    let resp = response
        .json::<GetMcpConfigResponse>()
        .await
        .map_err(|e| FetchMcpConfigError::Other(e.into()))?;

    if resp.errcode != 0 {
        return Err(FetchMcpConfigError::Api(resp));
    }

    let Some(list) = &(resp.list) else {
        return Err(FetchMcpConfigError::Other(anyhow::anyhow!(
            "<MCP config list is empty>"
        )));
    };

    save_mcp_config(list)?;

    Ok(resp)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::crypto;

    fn sample_items() -> Vec<McpConfigItem> {
        vec![
            McpConfigItem {
                url: Some("https://example.com/mcp/contact".into()),
                transport_type: Some("streamable-http".into()),
                is_authed: Some(true),
                biz_type: Some("contact".into()),
            },
            McpConfigItem {
                url: Some("https://example.com/mcp/msg".into()),
                transport_type: Some("streamable-http".into()),
                is_authed: Some(false),
                biz_type: Some("msg".into()),
            },
        ]
    }

    // -----------------------------------------------------------------------
    // Signature tests
    // -----------------------------------------------------------------------

    #[test]
    fn sha256_hex_matches_cpp_format() {
        let result = sha256_hex("test");
        // {:02x} format: standard lowercase hex, two digits per byte, zero-padded
        assert_eq!(
            result,
            "9f86d081884c7d659a2feaa0c55ad015a3bf4f1b2b0b822cd15d6c15b0f00a08"
        );
    }

    #[test]
    fn sign_produces_non_empty_signature() {
        let sig = sign("my_secret", "bot_123", 1774772074, "abc123");
        assert!(!sig.is_empty());
    }

    #[test]
    fn sign_is_deterministic() {
        let a = sign("sec", "id", 100, "nonce");
        let b = sign("sec", "id", 100, "nonce");
        assert_eq!(a, b);
    }

    #[test]
    fn sign_changes_with_different_inputs() {
        let a = sign("sec", "id", 100, "nonce1");
        let b = sign("sec", "id", 100, "nonce2");
        assert_ne!(a, b);
    }

    // -----------------------------------------------------------------------
    // Persistence tests
    // -----------------------------------------------------------------------

    #[test]
    fn save_and_load_roundtrip() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("mcp_config.enc");
        let key = crypto::generate_random_key();
        let items = sample_items();

        save_mcp_config_to_path(&items, &path, &key).unwrap();

        let loaded = load_mcp_config_from_path(&path, &key).unwrap();
        assert_eq!(loaded.len(), 2);
        assert_eq!(loaded[0].biz_type.as_deref(), Some("contact"));
        assert_eq!(
            loaded[0].url.as_deref(),
            Some("https://example.com/mcp/contact")
        );
        assert_eq!(loaded[1].biz_type.as_deref(), Some("msg"));
        assert_eq!(loaded[1].is_authed, Some(false));
    }

    #[test]
    fn load_returns_none_when_file_missing() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("nonexistent.enc");
        let key = crypto::generate_random_key();

        assert!(load_mcp_config_from_path(&path, &key).is_none());
    }

    #[test]
    fn load_returns_none_with_wrong_key() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("mcp_config.enc");
        let key = crypto::generate_random_key();
        let wrong_key = crypto::generate_random_key();
        let items = sample_items();

        save_mcp_config_to_path(&items, &path, &key).unwrap();

        assert!(load_mcp_config_from_path(&path, &wrong_key).is_none());
    }

    #[test]
    fn load_returns_none_with_corrupted_file() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("mcp_config.enc");
        let key = crypto::generate_random_key();

        std::fs::write(&path, b"garbage data").unwrap();

        assert!(load_mcp_config_from_path(&path, &key).is_none());
    }

    #[test]
    fn save_overwrites_existing_file() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("mcp_config.enc");
        let key = crypto::generate_random_key();

        let items_v1 = vec![McpConfigItem {
            url: Some("https://v1.example.com".into()),
            transport_type: Some("streamable-http".into()),
            is_authed: Some(true),
            biz_type: Some("v1".into()),
        }];
        save_mcp_config_to_path(&items_v1, &path, &key).unwrap();

        let items_v2 = sample_items();
        save_mcp_config_to_path(&items_v2, &path, &key).unwrap();

        let loaded = load_mcp_config_from_path(&path, &key).unwrap();
        assert_eq!(loaded.len(), 2);
        assert_eq!(loaded[0].biz_type.as_deref(), Some("contact"));
    }

    #[test]
    fn bind_source_serializes_as_number() {
        let json = serde_json::to_string(&McpBindSource::Interactive).unwrap();
        assert_eq!(json, "1", "Expected number 1, got: {json}");
        let json = serde_json::to_string(&McpBindSource::Qrcode).unwrap();
        assert_eq!(json, "2", "Expected number 2, got: {json}");
        let json = serde_json::to_string(&McpBindSource::Env).unwrap();
        assert_eq!(json, "3", "Expected number 3, got: {json}");
    }

    #[test]
    fn save_empty_list() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("mcp_config.enc");
        let key = crypto::generate_random_key();

        save_mcp_config_to_path(&[], &path, &key).unwrap();

        let loaded = load_mcp_config_from_path(&path, &key).unwrap();
        assert!(loaded.is_empty());
    }
}
