//! 鉴权引导：botid+secret 签名调用换取 Bearer token。
//!
//! 签名算法为 `sha256_hex(secret + bot_id + time + nonce)`；返回的 token
//! 由调用方（如 `auth init` / [`BotGatewayTokenProvider`](crate::provider)）
//! 统一保存至凭据存储，后续请求经 `Authorization: Bearer <token>` 注入。

use std::time::{SystemTime, UNIX_EPOCH};

use indexmap::IndexMap;
use serde::{Deserialize, Serialize};
use serde_repr::Serialize_repr;
use sha2::{Digest, Sha256};

use wecom_transport::EndpointHttpExt;

use crate::bot::BotCredential;
use crate::error::AuthError;

// ---------------------------------------------------------------------------
// Request ID
// ---------------------------------------------------------------------------

/// Generate a request ID in the format: `{prefix}_{timestamp_ms}_{random_hex}`.
fn gen_req_id(prefix: &str) -> String {
    let timestamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis();
    let random = generate_random_hex(8);
    format!("{prefix}_{timestamp}_{random}")
}

/// Generate a random hex string of the specified character length.
fn generate_random_hex(length: usize) -> String {
    use rand::RngExt;
    let byte_len = length.div_ceil(2);
    let bytes: Vec<u8> = (0..byte_len).map(|_| rand::rng().random::<u8>()).collect();
    let hex = hex::encode(bytes);
    hex[..length].to_string()
}

// ---------------------------------------------------------------------------
// Request
// ---------------------------------------------------------------------------

/// 配置来源
#[derive(Debug, Clone, Copy, Serialize_repr)]
#[repr(u8)]
pub enum BindSource {
    /// Interactive
    Interactive = 1,
    /// QR Code
    Qrcode = 2,
}

#[derive(Debug, Clone, Serialize)]
pub struct FetchAuthRequest {
    pub bot_id: String,
    pub time: u64,
    pub nonce: String,
    pub signature: String,
    pub bind_source: BindSource,
}

impl FetchAuthRequest {
    /// Build a signed request from the given bot credentials
    pub fn build(bot: &BotCredential, bind_source: BindSource) -> Result<Self, AuthError> {
        let time = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        let nonce = gen_req_id("cli");
        let signature = sign(&bot.secret, &bot.id, time, &nonce);

        Ok(Self {
            bot_id: bot.id.clone(),
            time,
            nonce,
            signature,
            bind_source,
        })
    }
}

// ---------------------------------------------------------------------------
// Response
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FetchAuthResponse {
    #[serde(default)]
    pub errcode: i32,
    pub errmsg: Option<String>,
    /// 鉴权成功后台返回的 Bearer token（后续请求经 Authorization 头携带）。
    #[serde(default)]
    pub token: Option<String>,
    #[serde(flatten)]
    pub extra: IndexMap<String, serde_json::Value>,
}

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
// API Call
// ---------------------------------------------------------------------------

/// Fetch the auth bootstrap config from the server (signed request), returning
/// the Bearer token for the caller to persist.
///
/// 复用调用方的请求能力；`endpoint` 应经 [`crate::gateway::auth_endpoint`]
/// 装配（扁平信封与鉴权抑制标记由它保证）。
///
/// # Errors
///
/// 网络/HTTP/解析经 [`AuthError::Transport`] 透传；业务错误（`errcode != 0`）
/// 由 [`FlatRes`](crate::gateway::FlatRes) 信封层校验并构造
/// `wecom_transport::Error::Api`（消息取后台 errmsg，body 透传原始响应）；
/// 响应格式不符为 transport 层 [`Parse`](wecom_transport::Error::Parse)
/// （含原始 body 与 serde source）。
pub async fn fetch_auth(
    transport: &wecom_transport::Transport,
    bot: &BotCredential,
    bind_source: BindSource,
    endpoint: &wecom_transport::Endpoint,
) -> Result<FetchAuthResponse, AuthError> {
    tracing::debug!(bind_source = ?bind_source, "auth bootstrap request");
    let request = FetchAuthRequest::build(bot, bind_source)
        .inspect_err(|e| tracing::error!(error = %e, "build auth bootstrap request failed"))?;

    // 纯字符串字段的结构体序列化失败为意料之外的系统级失败，归入兜底 Other。
    let payload = serde_json::to_value(&request).map_err(|e| AuthError::Other(e.into()))?;

    let value = transport.invoke(endpoint, &payload).await?.into_result()?;

    let resp = FetchAuthResponse::deserialize(&value).map_err(|e| {
        AuthError::from(wecom_transport::Error::Parse {
            message: format!("鉴权响应格式异常: {e}"),
            endpoint: EndpointHttpExt::full_url(endpoint),
            body: Box::new(value),
            source: Some(e),
        })
    })?;

    Ok(resp)
}

#[cfg(test)]
mod tests {
    //! ## 模块摘要：bootstrap（鉴权引导：botid+secret 签名换取 Bearer token）
    //!
    //! ### 关键接口
    //! - [sign] — `sha256_hex(secret + bot_id + time + nonce)` 签名算法
    //! - [sha256_hex] — SHA-256 小写零填充 hex
    //! - [FetchAuthRequest::build] — 构建带签名的请求体（time/nonce/signature/bind_source）
    //! - [BindSource] — 绑定来源枚举（Interactive=1 / Qrcode=2，序列化为数字）
    //!
    //! ### 关键分支与异常路径
    //! - `sha256_hex` 与 C++ 参考实现（`%02x` 小写零填充）一致
    //! - 签名确定性：相同输入 → 相同输出；不同 nonce → 不同签名
    //! - 请求体含 `bind_source`（绑定来源）与 `bot_id`

    use super::*;

    // -----------------------------------------------------------------------
    // Signature tests
    // -----------------------------------------------------------------------

    /// P0：sha256_hex 输出与 C++ 参考格式（%02x 小写零填充）一致
    /// 条件：对字符串 "test" 计算 sha256_hex
    /// 断言：输出等于已知的 64 位小写 hex 参考值
    #[test]
    fn sha256_hex_matches_cpp_format() {
        let result = sha256_hex("test");
        // {:02x} format: standard lowercase hex, two digits per byte, zero-padded
        assert_eq!(
            result,
            "9f86d081884c7d659a2feaa0c55ad015a3bf4f1b2b0b822cd15d6c15b0f00a08"
        );
    }

    /// P0：sign 产生非空签名
    /// 条件：给定 secret/bot_id/time/nonce 调用 sign
    /// 断言：签名字符串非空
    #[test]
    fn sign_produces_non_empty_signature() {
        let sig = sign("my_secret", "bot_123", 1774772074, "abc123");
        assert!(!sig.is_empty());
    }

    /// P0：sign 对相同输入输出确定性结果
    /// 条件：相同 secret/id/time/nonce 调用两次 sign
    /// 断言：两次签名相等
    #[test]
    fn sign_is_deterministic() {
        let a = sign("sec", "id", 100, "nonce");
        let b = sign("sec", "id", 100, "nonce");
        assert_eq!(a, b);
    }

    /// P1：不同输入产生不同签名
    /// 条件：仅 nonce 不同调用两次 sign
    /// 断言：两次签名不相等
    #[test]
    fn sign_changes_with_different_inputs() {
        let a = sign("sec", "id", 100, "nonce1");
        let b = sign("sec", "id", 100, "nonce2");
        assert_ne!(a, b);
    }

    // -----------------------------------------------------------------------
    // Serialization tests
    // -----------------------------------------------------------------------

    /// P0：BindSource 序列化为数字
    /// 条件：分别序列化 Interactive 与 Qrcode
    /// 断言：输出为字符串 "1" 与 "2"
    #[test]
    fn bind_source_serializes_as_number() {
        let json = serde_json::to_string(&BindSource::Interactive).unwrap();
        assert_eq!(json, "1", "Expected number 1, got: {json}");
        let json = serde_json::to_string(&BindSource::Qrcode).unwrap();
        assert_eq!(json, "2", "Expected number 2, got: {json}");
    }

    /// P0：请求体含 `bind_source` 与 `bot_id`
    /// 条件：构造 FetchAuthRequest::build 并序列化为 JSON
    /// 断言：包含 bind_source/bot_id 字段
    #[test]
    fn fetch_auth_request_includes_required_fields() {
        let bot = BotCredential::new("b".into(), "s".into());
        let req = FetchAuthRequest::build(&bot, BindSource::Interactive).unwrap();
        let json = serde_json::to_value(&req).unwrap();
        assert!(json.get("bind_source").is_some());
        assert!(json.get("bot_id").is_some());
    }
}
